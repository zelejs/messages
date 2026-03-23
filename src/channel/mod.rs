use crate::error::AppResult;
use crate::models::message::{ChannelResult, DispatchTarget, Message, MessageType};
use async_trait::async_trait;

pub mod log_file_channel;

pub use log_file_channel::LogFileChannel;

/// 消息渠道 trait
///
/// 所有推送渠道（WebSocket、Email、钉钉、日志等）都需要实现此 trait
#[async_trait]
pub trait MessageChannel: Send + Sync {
    /// 渠道名称
    fn name(&self) -> &str;

    /// 是否支持该消息类型
    fn supports(&self, msg_type: &MessageType) -> bool;

    /// 发送消息
    async fn send(
        &self,
        message: &Message,
        target: &DispatchTarget,
    ) -> AppResult<ChannelResult>;
}

/// 渠道管理器
pub struct ChannelManager {
    channels: Vec<Box<dyn MessageChannel>>,
}

impl ChannelManager {
    pub fn new() -> Self {
        Self {
            channels: Vec::new(),
        }
    }

    /// 注册渠道
    pub fn register_channel(&mut self, channel: Box<dyn MessageChannel>) {
        self.channels.push(channel);
    }

    /// 获取所有支持该消息类型的渠道
    pub fn get_supported_channels(&self, msg_type: &MessageType) -> Vec<&dyn MessageChannel> {
        self.channels
            .iter()
            .filter(|c| c.supports(msg_type))
            .map(|c| c.as_ref())
            .collect()
    }

    /// 发送消息到所有支持的渠道
    pub async fn dispatch(
        &self,
        message: &Message,
        target: &DispatchTarget,
        msg_type: &MessageType,
    ) -> Vec<AppResult<ChannelResult>> {
        let mut results = Vec::new();

        for channel in self.get_supported_channels(msg_type) {
            let result = channel.send(message, target).await;
            results.push(result);
        }

        results
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self::new()
    }
}
