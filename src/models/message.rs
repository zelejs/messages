use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

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
pub struct UnreadStats {
    pub total: i64,
    pub by_category: Vec<CategoryCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryCount {
    pub category: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}
