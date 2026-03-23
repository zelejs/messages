use crate::{error::AppResult, models::{message::Message, user::UserMessageSetting}};

#[async_trait::async_trait]
pub trait MessageChannel: Send + Sync {
    fn name(&self) -> &'static str;

    fn enabled(&self, settings: &UserMessageSetting) -> bool;

    async fn send(&self, message: &Message, user_id: i64, settings: &UserMessageSetting) -> AppResult<()>;
}
