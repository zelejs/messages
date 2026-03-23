use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Organization {
    pub id: i64,
    pub tenant_id: i64,
    pub parent_id: i64,
    pub org_code: String,
    pub org_name: String,
    pub org_type: Option<String>,
    pub level: i32,
    pub path: Option<String>,
    pub status: i16,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationTree {
    pub id: i64,
    pub tenant_id: i64,
    pub parent_id: i64,
    pub org_code: String,
    pub org_name: String,
    pub org_type: Option<String>,
    pub level: i32,
    pub children: Vec<OrganizationTree>,
}
