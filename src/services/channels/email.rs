use crate::{error::AppResult, models::{message::Message, user::UserMessageSetting}, services::channel::MessageChannel};

pub struct EmailChannel;

impl EmailChannel {
    pub fn new() -> Self {
        EmailChannel
    }
}

#[async_trait::async_trait]
impl MessageChannel for EmailChannel {
    fn name(&self) -> &'static str {
        "email"
    }

    fn enabled(&self, settings: &UserMessageSetting) -> bool {
        settings.email_enabled == 1
    }

    async fn send(&self, message: &Message, user_id: i64, _settings: &UserMessageSetting) -> AppResult<()> {
        tracing::info!("[email] push user={} message={} title={}", user_id, message.id, message.title);
        // 这里可插入 lettre 或其他邮件实现
        Ok(())
    }
}
