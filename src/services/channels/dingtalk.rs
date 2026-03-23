use crate::{error::AppResult, models::{message::Message, user::UserMessageSetting}, services::channel::MessageChannel};

pub struct DingTalkChannel;

impl DingTalkChannel {
    pub fn new() -> Self {
        DingTalkChannel
    }
}

#[async_trait::async_trait]
impl MessageChannel for DingTalkChannel {
    fn name(&self) -> &'static str {
        "dingtalk"
    }

    fn enabled(&self, settings: &UserMessageSetting) -> bool {
        settings.dingtalk_enabled == 1
    }

    async fn send(&self, message: &Message, user_id: i64, _settings: &UserMessageSetting) -> AppResult<()> {
        tracing::info!("[dingtalk] push user={} message={} title={}", user_id, message.id, message.title);
        // 这里可插入钉钉 webhook 调用实现
        Ok(())
    }
}
