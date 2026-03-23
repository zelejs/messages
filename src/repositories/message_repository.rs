use crate::{
    error::{AppError, AppResult},
    models::message::{Message, TargetRule, UserMessageDetail},
};
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub struct MessageRepository {
    db: PgPool,
}

impl MessageRepository {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn create(
        &self,
        tenant_id: i64,
        message_code: String,
        template_id: Option<i64>,
        category: String,
        priority: i16,
        title: String,
        content: Option<String>,
        jump_type: Option<String>,
        jump_params: Option<serde_json::Value>,
        extra_data: Option<serde_json::Value>,
        send_type: i16,
        scheduled_at: Option<chrono::DateTime<chrono::Utc>>,
        sender_id: Option<i64>,
    ) -> AppResult<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO t_sys_messages (
                tenant_id, message_code, template_id, category, priority,
                title, content, jump_type, jump_params, extra_data,
                send_type, scheduled_at, sender_id, status
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 0)
            RETURNING id
            "#
        )
        .bind(tenant_id)
        .bind(&message_code)
        .bind(template_id)
        .bind(&category)
        .bind(priority)
        .bind(&title)
        .bind(&content)
        .bind(&jump_type)
        .bind(&jump_params)
        .bind(&extra_data)
        .bind(send_type)
        .bind(scheduled_at)
        .bind(sender_id)
        .fetch_one(&self.db)
        .await?;

        Ok(result.get("id"))
    }

    pub async fn create_target_rule(
        &self,
        message_id: i64,
        target_type: String,
        target_scope: serde_json::Value,
        filter_conditions: Option<serde_json::Value>,
    ) -> AppResult<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO t_sys_message_target_rules (
                message_id, target_type, target_scope, filter_conditions
            ) VALUES ($1, $2, $3, $4)
            RETURNING id
            "#
        )
        .bind(message_id)
        .bind(&target_type)
        .bind(&target_scope)
        .bind(&filter_conditions)
        .fetch_one(&self.db)
        .await?;

        Ok(result.get("id"))
    }

    pub async fn get_by_id(&self, id: i64) -> AppResult<Option<Message>> {
        let message = sqlx::query_as("SELECT * FROM t_sys_messages WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.db)
            .await?;

        Ok(message)
    }

    pub async fn get_by_code(&self, code: &str) -> AppResult<Option<Message>> {
        let message = sqlx::query_as("SELECT * FROM t_sys_messages WHERE message_code = $1")
            .bind(code)
            .fetch_optional(&self.db)
            .await?;

        Ok(message)
    }

    pub async fn get_target_rules(&self, message_id: i64) -> AppResult<Vec<TargetRule>> {
        let rules = sqlx::query_as(
            "SELECT target_type, target_scope, filter_conditions FROM t_sys_message_target_rules WHERE message_id = $1"
        )
        .bind(message_id)
        .fetch_all(&self.db)
        .await?;

        Ok(rules)
    }

    pub async fn create_user_messages(
        &self,
        message_id: i64,
        user_ids: &[i64],
    ) -> AppResult<()> {
        for user_id in user_ids {
            sqlx::query(
                r#"
                INSERT INTO t_sys_user_messages (message_id, user_id, tenant_id)
                SELECT $1, $2, tenant_id FROM t_sys_messages WHERE id = $1
                ON CONFLICT (message_id, user_id) DO NOTHING
                "#
            )
            .bind(message_id)
            .bind(user_id)
            .execute(&self.db)
            .await?;
        }

        Ok(())
    }

    pub async fn list_user_messages(
        &self,
        user_id: i64,
        category: Option<&str>,
        is_read: Option<i16>,
        page: i64,
        page_size: i64,
    ) -> AppResult<Vec<UserMessageDetail>> {
        let offset = (page - 1) * page_size;

        let rows = sqlx::query(
            r#"
            SELECT
                m.id,
                m.tenant_id,
                m.message_code,
                m.template_id,
                m.category,
                m.priority,
                m.title,
                m.content,
                m.jump_type,
                m.jump_params,
                m.extra_data,
                m.send_type,
                m.scheduled_at,
                m.sent_at,
                m.expire_at,
                m.sender_id,
                m.sender_type,
                m.status,
                m.created_at,
                m.updated_at,
                um.is_read,
                um.read_at,
                um.is_pinned
            FROM t_sys_user_messages um
            JOIN t_sys_messages m ON um.message_id = m.id
            WHERE um.user_id = $1
              AND um.is_deleted = 0
              AND ($2::VARCHAR IS NULL OR m.category = $2)
              AND ($3::SMALLINT IS NULL OR um.is_read = $3)
            ORDER BY um.is_pinned DESC, m.created_at DESC
            LIMIT $4 OFFSET $5
            "#
        )
        .bind(user_id)
        .bind(category)
        .bind(is_read)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.db)
        .await
        .map_err(AppError::Database)?;

        let t_sys_messages: Vec<UserMessageDetail> = rows.into_iter().map(|row: sqlx::postgres::PgRow| {
            use sqlx::Row;
            UserMessageDetail {
                id: row.try_get("id").unwrap_or(0),
                tenant_id: row.try_get("tenant_id").unwrap_or(0),
                message_code: row.try_get("message_code").unwrap_or_default(),
                template_id: row.try_get("template_id").ok(),
                category: row.try_get("category").unwrap_or_default(),
                priority: row.try_get("priority").unwrap_or(0),
                title: row.try_get("title").unwrap_or_default(),
                content: row.try_get("content").ok(),
                jump_type: row.try_get("jump_type").ok(),
                jump_params: row.try_get("jump_params").ok(),
                extra_data: row.try_get("extra_data").ok(),
                send_type: row.try_get("send_type").unwrap_or(0),
                scheduled_at: row.try_get("scheduled_at").ok(),
                sent_at: row.try_get("sent_at").ok(),
                expire_at: row.try_get("expire_at").ok(),
                sender_id: row.try_get("sender_id").ok(),
                sender_type: row.try_get("sender_type").unwrap_or_default(),
                status: row.try_get("status").unwrap_or(0),
                created_at: row.try_get("created_at").unwrap_or_else(|_| chrono::Utc::now()),
                updated_at: row.try_get("updated_at").unwrap_or_else(|_| chrono::Utc::now()),
                is_read: row.try_get("is_read").unwrap_or(0),
                read_at: row.try_get("read_at").ok(),
                is_pinned: row.try_get("is_pinned").unwrap_or(0),
            }
        }).collect();

        Ok(t_sys_messages)
    }

    pub async fn mark_as_read(&self, message_id: i64, user_id: i64) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE t_sys_user_messages
            SET is_read = 1, read_at = NOW()
            WHERE message_id = $1 AND user_id = $2
            "#
        )
        .bind(message_id)
        .bind(user_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn batch_mark_as_read(&self, message_ids: &[i64], user_id: i64) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE t_sys_user_messages
            SET is_read = 1, read_at = NOW()
            WHERE message_id = ANY($1) AND user_id = $2
            "#
        )
        .bind(message_ids)
        .bind(user_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn mark_all_as_read(&self, user_id: i64, category: Option<&str>) -> AppResult<()> {
        match category {
            Some(cat) => {
                sqlx::query(
                    r#"
                    UPDATE t_sys_user_messages um
                    SET is_read = 1, read_at = NOW()
                    FROM t_sys_messages m
                    WHERE um.message_id = m.id
                      AND um.user_id = $1
                      AND m.category = $2
                      AND um.is_deleted = 0
                    "#
                )
                .bind(user_id)
                .bind(cat)
                .execute(&self.db)
                .await?;
            }
            None => {
                sqlx::query(
                    r#"
                    UPDATE t_sys_user_messages
                    SET is_read = 1, read_at = NOW()
                    WHERE user_id = $1 AND is_deleted = 0
                    "#
                )
                .bind(user_id)
                .execute(&self.db)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn delete_message(&self, message_id: i64, user_id: i64) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE t_sys_user_messages
            SET is_deleted = 1, deleted_at = NOW()
            WHERE message_id = $1 AND user_id = $2
            "#
        )
        .bind(message_id)
        .bind(user_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn batch_delete(&self, message_ids: &[i64], user_id: i64) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE t_sys_user_messages
            SET is_deleted = 1, deleted_at = NOW()
            WHERE message_id = ANY($1) AND user_id = $2
            "#
        )
        .bind(message_ids)
        .bind(user_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn pin_message(&self, message_id: i64, user_id: i64, pinned: bool) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE t_sys_user_messages
            SET is_pinned = $1
            WHERE message_id = $2 AND user_id = $3
            "#
        )
        .bind(if pinned { 1 } else { 0 })
        .bind(message_id)
        .bind(user_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn update_status(
        &self,
        message_id: i64,
        status: i16,
        sent_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE t_sys_messages
            SET status = $1, sent_at = $2
            WHERE id = $3
            "#
        )
        .bind(status)
        .bind(sent_at)
        .bind(message_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn log_push(
        &self,
        message_id: i64,
        user_id: i64,
        channel: &str,
        status: i16,
        error_msg: Option<&str>,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO t_sys_message_push_logs (message_id, user_id, channel, status, error_msg)
            VALUES ($1, $2, $3, $4, $5)
            "#
        )
        .bind(message_id)
        .bind(user_id)
        .bind(channel)
        .bind(status)
        .bind(error_msg)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn count_user_messages(
        &self,
        user_id: i64,
        category: Option<&str>,
        is_read: Option<i16>,
    ) -> AppResult<i64> {
        let result = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM t_sys_user_messages um
            JOIN t_sys_messages m ON um.message_id = m.id
            WHERE um.user_id = $1
              AND um.is_deleted = 0
              AND ($2::VARCHAR IS NULL OR m.category = $2)
              AND ($3::SMALLINT IS NULL OR um.is_read = $3)
            "#
        )
        .bind(user_id)
        .bind(category)
        .bind(is_read)
        .fetch_one(&self.db)
        .await?;

        Ok(result.get("count"))
    }

    pub async fn list_messages(
        &self,
        tenant_id: i64,
        page: i64,
        page_size: i64,
    ) -> AppResult<Vec<Message>> {
        let offset = (page - 1) * page_size;

        let t_sys_messages = sqlx::query_as(
            r#"
            SELECT * FROM t_sys_messages
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(tenant_id)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        Ok(t_sys_messages)
    }

    pub async fn count_messages(&self, tenant_id: i64) -> AppResult<i64> {
        let result = sqlx::query("SELECT COUNT(*) as count FROM t_sys_messages WHERE tenant_id = $1")
            .bind(tenant_id)
            .fetch_one(&self.db)
            .await?;

        Ok(result.get("count"))
    }

    pub async fn cancel_message(&self, message_id: i64) -> AppResult<()> {
        sqlx::query("UPDATE t_sys_messages SET status = 2 WHERE id = $1")
            .bind(message_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    pub async fn get_scheduled_messages(&self) -> AppResult<Vec<Message>> {
        let t_sys_messages = sqlx::query_as(
            r#"
            SELECT * FROM t_sys_messages
            WHERE status = 0
              AND scheduled_at IS NOT NULL
              AND scheduled_at <= NOW()
            ORDER BY scheduled_at ASC
            "#
        )
        .fetch_all(&self.db)
        .await?;

        Ok(t_sys_messages)
    }
}
