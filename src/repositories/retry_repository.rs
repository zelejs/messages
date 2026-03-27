use crate::{
    error::{AppError, AppResult},
    models::retry::{DeadLetterMessage, MessageRetryRecord, RetryQuery, DLQQuery},
};
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub struct RetryRepository {
    db: PgPool,
}

impl RetryRepository {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// 创建重试记录
    pub async fn create_retry_record(&self, record: &MessageRetryRecord) -> AppResult<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO t_sys_message_retry_records (
                message_id, user_id, channel, retry_count, max_retries,
                retry_intervals, next_retry_at, last_error, retry_history, status
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id
            "#
        )
        .bind(record.message_id)
        .bind(record.user_id)
        .bind(&record.channel)
        .bind(record.retry_count)
        .bind(record.max_retries)
        .bind(&record.retry_intervals)
        .bind(record.next_retry_at)
        .bind(&record.last_error)
        .bind(&record.retry_history)
        .bind(record.status)
        .fetch_one(&self.db)
        .await?;

        Ok(result.get("id"))
    }

    /// 更新重试记录
    pub async fn update_retry_record(&self, record: &MessageRetryRecord) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE t_sys_message_retry_records
            SET retry_count = $1,
                next_retry_at = $2,
                last_error = $3,
                retry_history = $4,
                status = $5,
                updated_at = NOW()
            WHERE id = $6
            "#
        )
        .bind(record.retry_count)
        .bind(record.next_retry_at)
        .bind(&record.last_error)
        .bind(&record.retry_history)
        .bind(record.status)
        .bind(record.id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// 获取待重试的记录
    pub async fn get_pending_retries(&self, limit: i64) -> AppResult<Vec<MessageRetryRecord>> {
        let records = sqlx::query_as(
            r#"
            SELECT * FROM t_sys_message_retry_records
            WHERE status = 0 AND next_retry_at <= NOW()
            ORDER BY next_retry_at ASC
            LIMIT $1
            "#
        )
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(records)
    }

    /// 获取消息的重试记录
    pub async fn get_retry_record(
        &self,
        message_id: i64,
        user_id: i64,
        channel: &str,
    ) -> AppResult<Option<MessageRetryRecord>> {
        let record = sqlx::query_as(
            r#"
            SELECT * FROM t_sys_message_retry_records
            WHERE message_id = $1 AND user_id = $2 AND channel = $3
            ORDER BY created_at DESC
            LIMIT 1
            "#
        )
        .bind(message_id)
        .bind(user_id)
        .bind(channel)
        .fetch_optional(&self.db)
        .await?;

        Ok(record)
    }

    /// 删除重试记录
    pub async fn delete_retry_record(&self, id: i64) -> AppResult<()> {
        sqlx::query("DELETE FROM t_sys_message_retry_records WHERE id = $1")
            .bind(id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    // ==================== Dead Letter Queue ====================

    /// 创建死信记录
    pub async fn create_dead_letter(&self, dead_letter: &DeadLetterMessage) -> AppResult<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO t_sys_message_dead_letters (
                message_id, user_id, channel, failed_reason,
                retry_history, status
            ) VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id
            "#
        )
        .bind(dead_letter.message_id)
        .bind(dead_letter.user_id)
        .bind(&dead_letter.channel)
        .bind(&dead_letter.failed_reason)
        .bind(&dead_letter.retry_history)
        .bind(dead_letter.status)
        .fetch_one(&self.db)
        .await?;

        Ok(result.get("id"))
    }

    /// 获取死信消息
    pub async fn get_dead_letter(&self, id: i64) -> AppResult<Option<DeadLetterMessage>> {
        let dead_letter = sqlx::query_as(
            "SELECT * FROM t_sys_message_dead_letters WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?;

        Ok(dead_letter)
    }

    /// 列出死信消息
    pub async fn list_dead_letters(&self, query: &DLQQuery) -> AppResult<Vec<DeadLetterMessage>> {
        let offset = (query.page - 1) * query.page_size;

        let dead_letters = if let Some(status) = query.status {
            sqlx::query_as(
                r#"
                SELECT * FROM t_sys_message_dead_letters
                WHERE status = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#
            )
            .bind(status)
            .bind(query.page_size)
            .bind(offset)
            .fetch_all(&self.db)
            .await?
        } else {
            sqlx::query_as(
                r#"
                SELECT * FROM t_sys_message_dead_letters
                ORDER BY created_at DESC
                LIMIT $1 OFFSET $2
                "#
            )
            .bind(query.page_size)
            .bind(offset)
            .fetch_all(&self.db)
            .await?
        };

        Ok(dead_letters)
    }

    /// 统计死信数量
    pub async fn count_dead_letters(&self, status: Option<i16>) -> AppResult<i64> {
        let count = if let Some(s) = status {
            sqlx::query(
                "SELECT COUNT(*) as count FROM t_sys_message_dead_letters WHERE status = $1"
            )
            .bind(s)
            .fetch_one(&self.db)
            .await?
        } else {
            sqlx::query("SELECT COUNT(*) as count FROM t_sys_message_dead_letters")
            .fetch_one(&self.db)
            .await?
        };

        Ok(count.get("count"))
    }

    /// 更新死信消息
    pub async fn update_dead_letter(&self, dead_letter: &DeadLetterMessage) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE t_sys_message_dead_letters
            SET status = $1,
                retried_at = $2,
                retried_success = $3,
                updated_at = NOW()
            WHERE id = $4
            "#
        )
        .bind(dead_letter.status)
        .bind(dead_letter.retried_at)
        .bind(dead_letter.retried_success)
        .bind(dead_letter.id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// 删除死信消息
    pub async fn delete_dead_letter(&self, id: i64) -> AppResult<()> {
        sqlx::query("DELETE FROM t_sys_message_dead_letters WHERE id = $1")
            .bind(id)
            .execute(&self.db)
            .await?;

        Ok(())
    }
}
