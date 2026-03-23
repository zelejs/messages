use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::fmt;

/// 消息源类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageSource {
    /// 系统消息源
    System,
    /// 组织消息源
    Organization,
    /// 审批任务源
    Workflow,
    /// 外部集成源
    External,
}

impl fmt::Display for MessageSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageSource::System => write!(f, "system"),
            MessageSource::Organization => write!(f, "organization"),
            MessageSource::Workflow => write!(f, "workflow"),
            MessageSource::External => write!(f, "external"),
        }
    }
}

/// 消息类型（业务分类）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    // 系统类
    SystemAnnouncement,
    SystemSecurity,
    SystemMaintenance,
    // 组织类
    OrgDepartment,
    OrgChange,
    OrgActivity,
    // 审批任务类
    WorkflowTodo,
    WorkflowResult,
    WorkflowCc,
    // 其他
    Other,
}

impl fmt::Display for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageType::SystemAnnouncement => write!(f, "system_announcement"),
            MessageType::SystemSecurity => write!(f, "system_security"),
            MessageType::SystemMaintenance => write!(f, "system_maintenance"),
            MessageType::OrgDepartment => write!(f, "org_department"),
            MessageType::OrgChange => write!(f, "org_change"),
            MessageType::OrgActivity => write!(f, "org_activity"),
            MessageType::WorkflowTodo => write!(f, "workflow_todo"),
            MessageType::WorkflowResult => write!(f, "workflow_result"),
            MessageType::WorkflowCc => write!(f, "workflow_cc"),
            MessageType::Other => write!(f, "other"),
        }
    }
}

/// 分发目标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchTarget {
    pub user_id: i64,
    pub org_id: Option<i64>,
    pub role_codes: Vec<String>,
    pub channels: Vec<String>,
}

/// 渠道推送结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelResult {
    pub channel: String,
    pub success: bool,
    pub message: Option<String>,
}

/// 消息分发日志记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDispatchLog {
    /// 分发时间
    pub timestamp: DateTime<Utc>,
    /// 消息唯一标识
    pub message_id: String,
    /// 消息源类型
    pub source_type: MessageSource,
    /// 具体来源（系统模块/组织ID/审批流程ID）
    pub source_detail: String,
    /// 目标组织ID列表
    pub target_orgs: Vec<i64>,
    /// 目标角色编码列表
    pub target_roles: Vec<String>,
    /// 目标用户ID列表
    pub target_users: Vec<i64>,
    /// 消息类型
    pub msg_type: MessageType,
    /// 业务分类
    pub category: String,
    /// 使用的渠道
    pub channels: Vec<String>,
    /// 分发状态
    pub status: String,
}

impl MessageDispatchLog {
    /// 格式化为日志行
    pub fn to_log_line(&self) -> String {
        format!(
            "[{}] {} | SOURCE:{} | FROM:{} | TYPE:{} | ORGS:{:?} | ROLES:{:?} | USERS:{:?} | CHANNELS:{:?} | STATUS:{}",
            self.timestamp.to_rfc3339(),
            self.message_id,
            self.source_type,
            self.source_detail,
            self.msg_type,
            self.target_orgs,
            self.target_roles,
            self.target_users,
            self.channels,
            self.status
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Message {
    pub id: i64,
    pub tenant_id: i64,
    pub message_code: String,
    pub template_id: Option<i64>,
    pub category: String,
    pub priority: i16,
    pub title: String,
    pub content: Option<String>,
    pub jump_type: Option<String>,
    pub jump_params: Option<serde_json::Value>,
    pub extra_data: Option<serde_json::Value>,
    pub send_type: i16,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub sent_at: Option<DateTime<Utc>>,
    pub expire_at: Option<DateTime<Utc>>,
    pub sender_id: Option<i64>,
    pub sender_type: String,
    // 消息源信息
    pub source_type: String,
    pub source_detail: Option<String>,
    pub status: i16,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    pub template_code: String,
    pub target_rules: Vec<TargetRule>,
    pub variables: serde_json::Value,
    pub send_type: Option<i16>,
    pub scheduled_at: Option<DateTime<Utc>>,
    /// 消息源类型
    pub source_type: MessageSource,
    /// 消息源详情（系统模块名/组织ID/流程ID）
    pub source_detail: String,
    /// 消息类型
    pub msg_type: MessageType,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TargetRule {
    pub target_type: String,
    pub target_scope: serde_json::Value,
    pub filter_conditions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetScope {
    pub user_ids: Option<Vec<i64>>,
    pub org_ids: Option<Vec<i64>>,
    pub include_children: Option<bool>,
    pub role_codes: Option<Vec<String>>,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[allow(dead_code)]
pub struct UserMessage {
    pub id: i64,
    pub message_id: i64,
    pub user_id: i64,
    pub tenant_id: i64,
    pub is_read: i16,
    pub read_at: Option<DateTime<Utc>>,
    pub is_deleted: i16,
    pub deleted_at: Option<DateTime<Utc>>,
    pub is_pinned: i16,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessageDetail {
    // Message fields flattened
    pub id: i64,
    pub tenant_id: i64,
    pub message_code: String,
    pub template_id: Option<i64>,
    pub category: String,
    pub priority: i16,
    pub title: String,
    pub content: Option<String>,
    pub jump_type: Option<String>,
    pub jump_params: Option<serde_json::Value>,
    pub extra_data: Option<serde_json::Value>,
    pub send_type: i16,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub sent_at: Option<DateTime<Utc>>,
    pub expire_at: Option<DateTime<Utc>>,
    pub sender_id: Option<i64>,
    pub sender_type: String,
    pub status: i16,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // User message fields
    pub is_read: i16,
    pub read_at: Option<DateTime<Utc>>,
    pub is_pinned: i16,
}

// Note: UserMessageDetail is manually constructed from joins in the repository
// The FromRow trait cannot be derived because the structure combines fields from multiple tables

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageListQuery {
    pub category: Option<String>,
    pub is_read: Option<i16>,
    pub priority: Option<i16>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct UnreadStats {
    pub total: i64,
    pub by_category: Vec<CategoryCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct CategoryCount {
    pub category: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}
