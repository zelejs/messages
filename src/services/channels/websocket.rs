use crate::{error::AppResult, models::{message::Message, user::UserMessageSetting}, services::channel::MessageChannel, websocket::WebSocketManager};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct WebSocketChannel {
    ws_manager: Arc<RwLock<WebSocketManager>>,
}

impl WebSocketChannel {
    pub fn new(ws_manager: Arc<RwLock<WebSocketManager>>) -> Self {
        Self { ws_manager }
    }
}

#[async_trait::async_trait]
impl MessageChannel for WebSocketChannel {
    fn name(&self) -> &'static str {
        "websocket"
    }

    fn enabled(&self, settings: &UserMessageSetting) -> bool {
        settings.web_enabled == 1
    }

    async fn send(&self, message: &Message, user_id: i64, _settings: &UserMessageSetting) -> AppResult<()> {
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
}
