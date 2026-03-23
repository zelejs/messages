use crate::{
    error::AppResult,
    models::user::{User, UserMessageSetting},
};
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub struct UserRepository {
    db: PgPool,
}

impl UserRepository {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn get_by_id(&self, id: i64) -> AppResult<Option<User>> {
        let user = sqlx::query_as("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.db)
            .await?;

        Ok(user)
    }

    pub async fn get_by_username(&self, username: &str) -> AppResult<Option<User>> {
        let user = sqlx::query_as("SELECT * FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(&self.db)
            .await?;

        Ok(user)
    }

    pub async fn get_by_organizations(&self, org_ids: &[i64]) -> AppResult<Vec<i64>> {
        if org_ids.is_empty() {
            return Ok(vec![]);
        }

        let user_ids = sqlx::query(
            r#"
            SELECT DISTINCT user_id
            FROM user_organizations
            WHERE organization_id = ANY($1)
            "#
        )
        .bind(org_ids)
        .fetch_all(&self.db)
        .await?
        .into_iter()
        .map(|r| r.get("user_id"))
        .collect();

        Ok(user_ids)
    }

    pub async fn get_by_roles(&self, tenant_id: i64, role_codes: &[String]) -> AppResult<Vec<i64>> {
        if role_codes.is_empty() {
            return Ok(vec![]);
        }

        let user_ids = sqlx::query(
            r#"
            SELECT DISTINCT ur.user_id
            FROM user_roles ur
            JOIN roles r ON ur.role_id = r.id
            WHERE r.tenant_id = $1 AND r.role_code = ANY($2)
            "#
        )
        .bind(tenant_id)
        .bind(role_codes)
        .fetch_all(&self.db)
        .await?
        .into_iter()
        .map(|r| r.get("user_id"))
        .collect();

        Ok(user_ids)
    }

    pub async fn get_by_custom_condition(&self, _tenant_id: i64, _condition: &str) -> AppResult<Vec<i64>> {
        // TODO: Implement custom condition query
        // This should use safe parameterized queries
        Ok(vec![])
    }

    pub async fn get_message_settings(&self, user_id: i64) -> AppResult<Vec<UserMessageSetting>> {
        let settings = sqlx::query_as("SELECT * FROM t_sys_user_message_settings WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&self.db)
            .await?;

        Ok(settings)
    }

    pub async fn get_message_settings_by_category(
        &self,
        user_id: i64,
        category: &str,
    ) -> AppResult<Option<UserMessageSetting>> {
        let setting = sqlx::query_as(
            "SELECT * FROM t_sys_user_message_settings WHERE user_id = $1 AND category = $2"
        )
        .bind(user_id)
        .bind(category)
        .fetch_optional(&self.db)
        .await?;

        Ok(setting)
    }

    pub async fn upsert_message_settings(
        &self,
        user_id: i64,
        category: Option<String>,
        web_enabled: Option<i16>,
        email_enabled: Option<i16>,
        dingtalk_enabled: Option<i16>,
        do_not_disturb: Option<i16>,
        dnd_start_time: Option<chrono::NaiveTime>,
        dnd_end_time: Option<chrono::NaiveTime>,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO t_sys_user_message_settings (
                user_id, category, web_enabled, email_enabled, dingtalk_enabled,
                do_not_disturb, dnd_start_time, dnd_end_time
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (user_id, category)
            DO UPDATE SET
                web_enabled = COALESCE(EXCLUDED.web_enabled, t_sys_user_message_settings.web_enabled),
                email_enabled = COALESCE(EXCLUDED.email_enabled, t_sys_user_message_settings.email_enabled),
                dingtalk_enabled = COALESCE(EXCLUDED.dingtalk_enabled, t_sys_user_message_settings.dingtalk_enabled),
                do_not_disturb = COALESCE(EXCLUDED.do_not_disturb, t_sys_user_message_settings.do_not_disturb),
                dnd_start_time = COALESCE(EXCLUDED.dnd_start_time, t_sys_user_message_settings.dnd_start_time),
                dnd_end_time = COALESCE(EXCLUDED.dnd_end_time, t_sys_user_message_settings.dnd_end_time)
            "#
        )
        .bind(user_id)
        .bind(&category)
        .bind(web_enabled)
        .bind(email_enabled)
        .bind(dingtalk_enabled)
        .bind(do_not_disturb)
        .bind(dnd_start_time)
        .bind(dnd_end_time)
        .execute(&self.db)
        .await?;

        Ok(())
    }
}
