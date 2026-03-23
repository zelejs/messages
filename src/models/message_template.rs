use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MessageTemplate {
    pub id: i64,
    pub template_code: String,
    pub template_name: String,
    pub category: String,
    pub priority: i16,
    pub title_template: Option<String>,
    pub content_template: Option<String>,
    pub jump_type: Option<String>,
    pub jump_params: Option<serde_json::Value>,
    pub channels: Option<serde_json::Value>,
    pub is_system: i16,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTemplateRequest {
    pub template_code: String,
    pub template_name: String,
    pub category: String,
    pub priority: Option<i16>,
    pub title_template: Option<String>,
    pub content_template: Option<String>,
    pub jump_type: Option<String>,
    pub jump_params: Option<serde_json::Value>,
    pub channels: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTemplateRequest {
    pub template_name: Option<String>,
    pub category: Option<String>,
    pub priority: Option<i16>,
    pub title_template: Option<String>,
    pub content_template: Option<String>,
    pub jump_type: Option<String>,
    pub jump_params: Option<serde_json::Value>,
    pub channels: Option<Vec<String>>,
}
