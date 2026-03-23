use crate::{
    error::AppResult,
    models::message::Message,
    repositories::{message_repository::MessageRepository, user_repository::UserRepository},
    services::{channel::MessageChannel, channels::{WebSocketChannel, EmailChannel, DingTalkChannel}},
    websocket::WebSocketManager,
};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct PushService {
    message_repo: MessageRepository,
    user_repo: UserRepository,
    ws_manager: Arc<RwLock<WebSocketManager>>,
    channels: Vec<Arc<dyn MessageChannel>>,
}

impl PushService {
    pub fn new(
        db: PgPool,
        _redis: redis::aio::ConnectionManager,
        ws_manager: Arc<RwLock<WebSocketManager>>,
        channels: Vec<Arc<dyn MessageChannel>>,
    ) -> Self {
        Self {
            message_repo: MessageRepository::new(db.clone()),
            user_repo: UserRepository::new(db),
            ws_manager,
            channels,
        }
    }

    pub fn with_default_channels(
        db: PgPool,
        redis: redis::aio::ConnectionManager,
        ws_manager: Arc<RwLock<WebSocketManager>>,
    ) -> Self {
        let mut channels: Vec<Arc<dyn MessageChannel>> = Vec::new();
        channels.push(Arc::new(WebSocketChannel::new(ws_manager.clone())));
        channels.push(Arc::new(EmailChannel::new()));
        channels.push(Arc::new(DingTalkChannel::new()));

        Self::new(db, redis, ws_manager, channels)
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
                tracing::warn!("no message settings found for user={}", user_id);
                continue;
            };

            let mut pushed_channels: Vec<String> = Vec::new();

            for channel in self.channels.iter() {
                if !channel.enabled(&settings) {
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
                    continue; // avoid websocket for offline users
                }

                if let Err(err) = channel.send(message, user_id, &settings).await {
                    tracing::warn!("channel={} send failed user={} message={} error={:?}", channel.name(), user_id, message.id, err);
                    continue;
                }

                pushed_channels.push(channel.name().to_string());

                self.message_repo
                    .log_push(
                        message.id,
                        user_id,
                        channel.name(),
                        if is_websocket { 1 } else { 0 },
                        None,
                    )
                    .await?;
            }

            if pushed_channels.is_empty() {
                tracing::info!("no active push channels for user={}" , user_id);
            }
        }

        self.message_repo
            .update_status(message.id, 1, Some(chrono::Utc::now()))
            .await?;

        Ok(())
    }
}
