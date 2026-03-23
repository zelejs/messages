use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,
    pub tenant_id: i64,
    pub username: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub status: i16,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserMessageSetting {
    pub id: i64,
    pub user_id: i64,
    pub category: Option<String>,
    pub web_enabled: i16,
    pub email_enabled: i16,
    pub dingtalk_enabled: i16,
    pub do_not_disturb: i16,
    pub dnd_start_time: Option<chrono::NaiveTime>,
    pub dnd_end_time: Option<chrono::NaiveTime>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
