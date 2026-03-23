use crate::{
    error::AppResult,
    models::message::Message,
    repositories::{message_repository::MessageRepository, user_repository::UserRepository},
    websocket::WebSocketManager,
};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;

#[allow(dead_code)]
pub struct PushService {
    message_repo: MessageRepository,
    user_repo: UserRepository,
    ws_manager: Arc<RwLock<WebSocketManager>>,
}

impl PushService {
    #[allow(dead_code)]
    pub fn new(
        db: PgPool,
        _redis: redis::aio::ConnectionManager,
        ws_manager: Arc<RwLock<WebSocketManager>>,
    ) -> Self {
        Self {
            message_repo: MessageRepository::new(db.clone()),
            user_repo: UserRepository::new(db),
            ws_manager,
        }
    }

    #[allow(dead_code)]
    pub async fn push_to_users(
        &self,
        message: &Message,
        user_ids: Vec<i64>,
    ) -> AppResult<()> {
        // 1. Batch create user message records
        self.message_repo.create_user_messages(message.id, &user_ids).await?;

        // 2. Check user online status and push
        for user_id in user_ids {
            // Check if user is online (via WebSocket manager)
            let is_online = {
                let manager = self.ws_manager.read().await;
                manager.is_connected(message.tenant_id, user_id).await
            };

            if is_online {
                // WebSocket real-time push
                self.push_via_websocket(message, user_id).await?;
            } else {
                // Offline push (email, DingTalk, etc.)
                self.push_offline_channels(message, user_id).await?;
            }

            // Log push
            self.message_repo
                .log_push(
                    message.id,
                    user_id,
                    "web",
                    if is_online { 1 } else { 0 },
                    None,
                )
                .await?;
        }

        // 3. Update message status
        self.message_repo
            .update_status(message.id, 1, Some(chrono::Utc::now()))
            .await?;

        Ok(())
    }

    #[allow(dead_code)]
    async fn push_via_websocket(&self, message: &Message, user_id: i64) -> AppResult<()> {
        let payload = serde_json::json!({
            "type": "new_message",
            "data": {
                "id": message.id,
                "title": message.title,
                "category": message.category,
                "priority": message.priority,
                "created_at": message.created_at,
            }
        });

        let manager = self.ws_manager.read().await;
        manager.send_to_user(message.tenant_id, user_id, payload).await;

        Ok(())
    }

    #[allow(dead_code)]
    async fn push_offline_channels(&self, message: &Message, user_id: i64) -> AppResult<()> {
        // Get user message settings
        let settings = self.user_repo.get_message_settings(user_id).await?;

        // TODO: Implement email, DingTalk, etc. push based on settings
        // - Email (via lettre)
        // - DingTalk (via webhook)
        // - Enterprise WeChat
        // - SMS

        tracing::debug!("Offline push: user={}, message={}", user_id, message.id);
        tracing::debug!("User settings: {:?}", settings);

        Ok(())
    }
}
