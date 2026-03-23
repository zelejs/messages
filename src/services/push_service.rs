use crate::{
    error::AppResult,
    models::message::Message,
    repositories::{message_repository::MessageRepository, user_repository::UserRepository},
    services::{channel::MessageChannel, channels::{WebSocketChannel, EmailChannel, DingTalkChannel}},
    websocket::WebSocketManager,
    config::ChannelConfig,
};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::NaiveTime;

pub struct PushService {
    message_repo: MessageRepository,
    user_repo: UserRepository,
    ws_manager: Arc<RwLock<WebSocketManager>>,
    channels: Vec<Arc<dyn MessageChannel>>,
    channel_config: ChannelConfig,
}

impl PushService {
    pub fn new(
        db: PgPool,
        _redis: redis::aio::ConnectionManager,
        ws_manager: Arc<RwLock<WebSocketManager>>,
        channels: Vec<Arc<dyn MessageChannel>>,
        channel_config: ChannelConfig,
    ) -> Self {
        Self {
            message_repo: MessageRepository::new(db.clone()),
            user_repo: UserRepository::new(db),
            ws_manager,
            channels,
            channel_config,
        }
    }

    pub fn with_default_channels(
        db: PgPool,
        redis: redis::aio::ConnectionManager,
        ws_manager: Arc<RwLock<WebSocketManager>>,
        channel_config: ChannelConfig,
    ) -> Self {
        let mut channels: Vec<Arc<dyn MessageChannel>> = Vec::new();

        if channel_config.websocket_enabled {
            channels.push(Arc::new(WebSocketChannel::new(ws_manager.clone())));
        }
        if channel_config.email_enabled {
            channels.push(Arc::new(EmailChannel::new()));
        }
        if channel_config.dingtalk_enabled {
            channels.push(Arc::new(DingTalkChannel::new()));
        }

        Self::new(db, redis, ws_manager, channels, channel_config)
    }

    /// 检查用户是否处于免打扰时段
    fn is_in_dnd(settings: &crate::models::user::UserMessageSetting) -> bool {
        if settings.do_not_disturb != 1 {
            return false;
        }

        // 检查是否在 DND 时段内
        if let (Some(start), Some(end)) = (settings.dnd_start_time, settings.dnd_end_time) {
            let now = chrono::Local::now().time();
            let start: NaiveTime = start;
            let end: NaiveTime = end;

            if start <= end {
                // 例如 22:00 - 08:00 (跨天)
                now >= start || now <= end
            } else {
                // 例如 08:00 - 18:00 (当天)
                now >= start && now <= end
            }
        } else {
            // 设置了 DND 但没有时段，表示全天免打扰
            true
        }
    }

    pub async fn push_to_users(
        &self,
        message: &Message,
        user_ids: Vec<i64>,
    ) -> AppResult<()> {
        self.message_repo.create_user_messages(message.id, &user_ids).await?;

        for user_id in user_ids {
            let settings_vec = self.user_repo.get_message_settings(user_id).await?;
            let settings_opt = settings_vec
                .iter()
                .find(|s| s.category.as_deref() == Some(&message.category))
                .or_else(|| settings_vec.first());

            let settings = if let Some(s) = settings_opt {
                s.clone()
            } else {
                // 用户没有设置，使用默认设置
                crate::models::user::UserMessageSetting {
                    id: 0,
                    user_id,
                    category: Some(message.category.clone()),
                    web_enabled: 1,
                    email_enabled: 0,
                    dingtalk_enabled: 0,
                    do_not_disturb: 0,
                    dnd_start_time: None,
                    dnd_end_time: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }
            };

            // 1. 检查 DND（免打扰）
            if Self::is_in_dnd(&settings) {
                self.message_repo
                    .log_push(
                        message.id,
                        user_id,
                        "system",
                        0, // 失败状态
                        Some("用户处于免打扰时段，消息被拒绝发送"),
                    )
                    .await?;
                tracing::info!("message={} user={} rejected due to DND", message.id, user_id);
                continue;
            }

            let mut pushed_channels: Vec<String> = Vec::new();

            for channel in self.channels.iter() {
                // 2. 检查渠道是否被用户禁用
                if !channel.enabled(&settings) {
                    self.message_repo
                        .log_push(
                            message.id,
                            user_id,
                            channel.name(),
                            0, // 失败状态
                            Some("渠道被用户设置禁用"),
                        )
                        .await?;
                    continue;
                }

                let is_websocket = channel.name() == "websocket";
                let is_connected = if is_websocket {
                    let manager = self.ws_manager.read().await;
                    manager.is_connected(message.tenant_id, user_id).await
                } else {
                    false
                };

                if is_websocket && !is_connected {
                    // WebSocket 未连接，记录失败但继续尝试其他渠道
                    self.message_repo
                        .log_push(
                            message.id,
                            user_id,
                            channel.name(),
                            0, // 失败
                            Some("WebSocket 未连接"),
                        )
                        .await?;
                    continue;
                }

                // 3. 执行推送
                match channel.send(message, user_id, &settings).await {
                    Ok(_) => {
                        // 推送成功
                        self.message_repo
                            .log_push(
                                message.id,
                                user_id,
                                channel.name(),
                                1, // 成功
                                None,
                            )
                            .await?;
                        pushed_channels.push(channel.name().to_string());
                        tracing::info!("channel={} message={} user={} push success", channel.name(), message.id, user_id);
                    }
                    Err(err) => {
                        // 推送失败
                        let error_msg = format!("{:?}", err);
                        self.message_repo
                            .log_push(
                                message.id,
                                user_id,
                                channel.name(),
                                0, // 失败
                                Some(&error_msg),
                            )
                            .await?;
                        tracing::warn!("channel={} message={} user={} push failed: {:?}", channel.name(), message.id, user_id, err);
                    }
                }
            }

            if pushed_channels.is_empty() {
                tracing::warn!("message={} user={} no active push channels", message.id, user_id);
            }
        }

        self.message_repo
            .update_status(message.id, 1, Some(chrono::Utc::now()))
            .await?;

        Ok(())
    }
}
