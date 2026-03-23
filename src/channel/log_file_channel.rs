use crate::channel::MessageChannel;
use crate::error::AppResult;
use crate::models::message::{ChannelResult, DispatchTarget, Message, MessageDispatchLog, MessageSource, MessageType};
use async_trait::async_trait;
use chrono::Utc;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use tracing;

/// 日志文件渠道
///
/// 默认的 stub 渠道，将消息分发信息记录到独立的日志文件
/// 日志路径: logs/message-dispatch.YYYY-MM-DD.log
pub struct LogFileChannel {
    log_dir: PathBuf,
}

impl LogFileChannel {
    /// 创建新的日志文件渠道
    pub fn new<P: Into<PathBuf>>(log_dir: P) -> Self {
        Self {
            log_dir: log_dir.into(),
        }
    }

    /// 使用默认日志目录创建
    pub fn default_with_dir() -> Self {
        Self::new("logs")
    }

    /// 获取当前日期的日志文件路径
    fn get_log_file_path(&self) -> PathBuf {
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        self.log_dir.join(format!("message-dispatch.{}.log", date))
    }

    /// 确保日志目录存在
    fn ensure_log_dir(&self) -> AppResult<()> {
        if !self.log_dir.exists() {
            std::fs::create_dir_all(&self.log_dir)?;
        }
        Ok(())
    }

    /// 写入日志
    fn write_log(&self, log_entry: &str) -> AppResult<()> {
        self.ensure_log_dir()?;

        let log_path = self.get_log_file_path();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        writeln!(file, "{}", log_entry)?;
        file.flush()?;

        // 同时输出到 tracing 日志
        tracing::info!(target: "message_dispatch", "{}", log_entry);

        Ok(())
    }

    /// 构建分发日志
    fn build_dispatch_log(
        &self,
        message: &Message,
        target: &DispatchTarget,
        source_type: &MessageSource,
        source_detail: &str,
        msg_type: &MessageType,
    ) -> MessageDispatchLog {
        // 从 message.extra_data 中解析目标信息（如果存在）
        let (target_orgs, target_roles) = if let Some(extra) = &message.extra_data {
            let orgs = extra
                .get("target_orgs")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_i64())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let roles = extra
                .get("target_roles")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            (orgs, roles)
        } else {
            (Vec::new(), Vec::new())
        };

        MessageDispatchLog {
            timestamp: Utc::now(),
            message_id: message.message_code.clone(),
            source_type: source_type.clone(),
            source_detail: source_detail.to_string(),
            target_orgs,
            target_roles,
            target_users: vec![target.user_id],
            msg_type: msg_type.clone(),
            category: message.category.clone(),
            channels: vec!["log".to_string()],
            status: "success".to_string(),
        }
    }
}

#[async_trait]
impl MessageChannel for LogFileChannel {
    fn name(&self) -> &str {
        "log_file"
    }

    fn supports(&self, _msg_type: &MessageType) -> bool {
        // 日志渠道支持所有消息类型
        true
    }

    async fn send(
        &self,
        message: &Message,
        target: &DispatchTarget,
    ) -> AppResult<ChannelResult> {
        // 从 extra_data 中解析 source_type, source_detail, msg_type
        let (source_type, source_detail, msg_type) = if let Some(extra) = &message.extra_data {
            let source_type = extra
                .get("source_type")
                .and_then(|v| v.as_str())
                .map(|s| match s {
                    "system" => MessageSource::System,
                    "organization" => MessageSource::Organization,
                    "workflow" => MessageSource::Workflow,
                    "external" => MessageSource::External,
                    _ => MessageSource::System,
                })
                .unwrap_or(MessageSource::System);

            let source_detail = extra
                .get("source_detail")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let msg_type = extra
                .get("msg_type")
                .and_then(|v| v.as_str())
                .map(|s| match s {
                    "system_announcement" => MessageType::SystemAnnouncement,
                    "system_security" => MessageType::SystemSecurity,
                    "system_maintenance" => MessageType::SystemMaintenance,
                    "org_department" => MessageType::OrgDepartment,
                    "org_change" => MessageType::OrgChange,
                    "org_activity" => MessageType::OrgActivity,
                    "workflow_todo" => MessageType::WorkflowTodo,
                    "workflow_result" => MessageType::WorkflowResult,
                    "workflow_cc" => MessageType::WorkflowCc,
                    _ => MessageType::Other,
                })
                .unwrap_or(MessageType::Other);

            (source_type, source_detail, msg_type)
        } else {
            (
                MessageSource::System,
                "unknown".to_string(),
                MessageType::Other,
            )
        };

        // 构建分发日志
        let dispatch_log = self.build_dispatch_log(
            message,
            target,
            &source_type,
            &source_detail,
            &msg_type,
        );

        // 写入日志文件
        let log_line = dispatch_log.to_log_line();
        match self.write_log(&log_line) {
            Ok(_) => Ok(ChannelResult {
                channel: self.name().to_string(),
                success: true,
                message: Some("Logged to file".to_string()),
            }),
            Err(e) => Ok(ChannelResult {
                channel: self.name().to_string(),
                success: false,
                message: Some(format!("Failed to write log: {}", e)),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::message::Message;
    use chrono::Utc;

    fn create_test_message() -> Message {
        Message {
            id: 1,
            tenant_id: 1,
            message_code: "MSG_test123".to_string(),
            template_id: None,
            category: "test".to_string(),
            priority: 2,
            title: "Test Message".to_string(),
            content: Some("Test content".to_string()),
            jump_type: None,
            jump_params: None,
            extra_data: Some(serde_json::json!({
                "source_type": "system",
                "source_detail": "system_monitor",
                "msg_type": "system_security",
                "target_orgs": [10, 11],
                "target_roles": ["admin", "manager"],
            })),
            send_type: 1,
            scheduled_at: None,
            sent_at: None,
            expire_at: None,
            sender_id: Some(1),
            sender_type: "system".to_string(),
            status: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_target() -> DispatchTarget {
        DispatchTarget {
            user_id: 1001,
            org_id: Some(10),
            role_codes: vec!["admin".to_string()],
            channels: vec!["log".to_string()],
        }
    }

    #[test]
    fn test_message_dispatch_log_format() {
        let log = MessageDispatchLog {
            timestamp: Utc::now(),
            message_id: "MSG_abc123".to_string(),
            source_type: MessageSource::System,
            source_detail: "system_monitor".to_string(),
            target_orgs: vec![10, 11],
            target_roles: vec!["admin".to_string(), "manager".to_string()],
            target_users: vec![1001, 1002, 1003],
            msg_type: MessageType::SystemSecurity,
            category: "security".to_string(),
            channels: vec!["log".to_string(), "websocket".to_string()],
            status: "success".to_string(),
        };

        let log_line = log.to_log_line();
        assert!(log_line.contains("MSG_abc123"));
        assert!(log_line.contains("SOURCE:system"));
        assert!(log_line.contains("TYPE:system_security"));
        assert!(log_line.contains("system_monitor"));
    }

    #[tokio::test]
    async fn test_log_file_channel_send() {
        let temp_dir = std::env::temp_dir().join("message_test_logs");
        let channel = LogFileChannel::new(&temp_dir);

        let message = create_test_message();
        let target = create_test_target();

        let result = channel.send(&message, &target).await;
        assert!(result.is_ok());

        let channel_result = result.unwrap();
        assert!(channel_result.success);

        // 清理
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
