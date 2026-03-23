use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[allow(dead_code)]
pub struct Role {
    pub id: i64,
    pub tenant_id: i64,
    pub role_code: String,
    pub role_name: String,
    pub status: i16,
    pub created_at: DateTime<Utc>,
}
