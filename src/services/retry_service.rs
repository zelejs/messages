use crate::{
    config::RetryConfig as ConfigRetryConfig,
    error::AppResult,
    models::retry::{DeadLetterMessage, MessageRetryRecord, RetryQuery, DLQQuery},
    repositories::retry_repository::RetryRepository,
};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing;

/// 重试服务
#[derive(Clone)]
pub struct RetryService {
    repo: RetryRepository,
    config: ConfigRetryConfig,
    dlq_service: DLQService,
}

impl RetryService {
    pub fn new(repo: RetryRepository, config: ConfigRetryConfig) -> Self {
        let dlq_service = DLQService::new(repo.clone());
        Self { repo, config, dlq_service }
    }

    /// Get reference to DLQ service
    pub fn dlq_service(&self) -> &DLQService {
        &self.dlq_service
    }

    /// 创建重试记录
    pub async fn create_retry(
        &self,
        message_id: i64,
        user_id: i64,
        channel: &str,
        error: &str,
    ) -> AppResult<i64> {
        let intervals = self.config.intervals();
        let record = MessageRetryRecord::new(
            message_id,
            user_id,
            channel,
            self.config.max_retries,
            intervals,
        );

        let mut record = record;
        record.last_error = Some(error.to_string());

        let id = self.repo.create_retry_record(&record).await?;
        tracing::info!(
            "Created retry record: id={}, message_id={}, user_id={}, channel={}, max_retries={}",
            id, message_id, user_id, channel, record.max_retries
        );

        Ok(id)
    }

    /// 记录一次重试尝试
    pub async fn record_attempt(
        &self,
        record: &mut MessageRetryRecord,
        error: Option<&str>,
    ) -> AppResult<bool> {
        record.record_attempt(error);

        if error.is_none() {
            // 重试成功
            record.mark_success();
            self.repo.update_retry_record(record).await?;
            tracing::info!(
                "Retry succeeded: message_id={}, user_id={}, channel={}, attempts={}",
                record.message_id, record.user_id, record.channel, record.retry_count
            );
            Ok(true)
        } else if record.has_more_retries() {
            // 还有重试机会
            self.repo.update_retry_record(record).await?;
            tracing::info!(
                "Retry scheduled: message_id={}, user_id={}, channel={}, attempt={}/{}, next_retry={:?}",
                record.message_id, record.user_id, record.channel,
                record.retry_count, record.max_retries, record.next_retry_at
            );
            Ok(false)
        } else {
            // 重试次数用尽，进入死信队列
            record.mark_dead_letter();
            self.repo.update_retry_record(record).await?;
            tracing::warn!(
                "Retry exhausted, moving to DLQ: message_id={}, user_id={}, channel={}, attempts={}",
                record.message_id, record.user_id, record.channel, record.retry_count
            );
            Ok(false)
        }
    }

    /// 获取待重试的记录
    pub async fn get_pending_retries(&self, limit: i64) -> AppResult<Vec<MessageRetryRecord>> {
        self.repo.get_pending_retries(limit).await
    }

    /// 获取消息的重试记录
    pub async fn get_retry_record(
        &self,
        message_id: i64,
        user_id: i64,
        channel: &str,
    ) -> AppResult<Option<MessageRetryRecord>> {
        self.repo.get_retry_record(message_id, user_id, channel).await
    }

    /// 删除重试记录
    pub async fn delete_retry(&self, id: i64) -> AppResult<()> {
        self.repo.delete_retry_record(id).await
    }

    /// 检查是否需要创建新的重试记录
    pub async fn should_retry(
        &self,
        message_id: i64,
        user_id: i64,
        channel: &str,
    ) -> AppResult<bool> {
        if !self.config.enabled {
            return Ok(false);
        }

        let existing = self.get_retry_record(message_id, user_id, channel).await?;

        if let Some(record) = existing {
            // 已有重试记录，检查状态
            Ok(record.status == 0 && record.has_more_retries())
        } else {
            // 没有重试记录，可以创建
            Ok(true)
        }
    }

    /// 启动重试轮询任务
    pub async fn start_retry_worker<F, Fut>(self: Arc<Self>, processor: F)
    where
        F: Fn(MessageRetryRecord) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = AppResult<bool>> + Send,
    {
        let mut ticker = interval(Duration::from_secs(30)); // 每30秒检查一次

        tokio::spawn(async move {
            loop {
                ticker.tick().await;

                match self.get_pending_retries(100).await {
                    Ok(records) => {
                        if !records.is_empty() {
                            tracing::info!("Processing {} retry records", records.len());

                            for record in records {
                                let processor = &processor;
                                match processor(record.clone()).await {
                                    Ok(success) => {
                                        if success {
                                            tracing::info!(
                                                "Retry succeeded for record id={}",
                                                record.id
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "Retry failed for record id={}: {:?}",
                                            record.id, e
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to get pending retries: {:?}", e);
                    }
                }
            }
        });
    }
}

/// 死信队列服务
#[derive(Clone)]
pub struct DLQService {
    repo: RetryRepository,
}

impl DLQService {
    pub fn new(repo: RetryRepository) -> Self {
        Self { repo }
    }

    /// 将重试记录移入死信队列
    pub async fn move_to_dlq(&self, retry_record: &MessageRetryRecord) -> AppResult<i64> {
        let dead_letter = DeadLetterMessage::from_retry_record(retry_record);
        let id = self.repo.create_dead_letter(&dead_letter).await?;

        tracing::info!(
            "Moved to DLQ: id={}, message_id={}, user_id={}, channel={}",
            id, dead_letter.message_id, dead_letter.user_id, dead_letter.channel
        );

        Ok(id)
    }

    /// 创建死信记录（直接创建，不经过重试）
    pub async fn create_dead_letter(
        &self,
        message_id: i64,
        user_id: i64,
        channel: &str,
        failed_reason: &str,
    ) -> AppResult<i64> {
        let dead_letter = DeadLetterMessage {
            id: 0,
            message_id,
            user_id,
            channel: channel.to_string(),
            failed_reason: Some(failed_reason.to_string()),
            retry_history: None,
            status: 0,
            retried_at: None,
            retried_success: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let id = self.repo.create_dead_letter(&dead_letter).await?;
        tracing::info!(
            "Created dead letter: id={}, message_id={}, user_id={}, channel={}",
            id, message_id, user_id, channel
        );

        Ok(id)
    }

    /// 获取死信消息
    pub async fn get_dead_letter(&self, id: i64) -> AppResult<Option<DeadLetterMessage>> {
        self.repo.get_dead_letter(id).await
    }

    /// 列出死信消息
    pub async fn list_dead_letters(
        &self,
        query: &DLQQuery,
    ) -> AppResult<(Vec<DeadLetterMessage>, i64)> {
        let dead_letters = self.repo.list_dead_letters(query).await?;
        let total = self.repo.count_dead_letters(query.status).await?;

        Ok((dead_letters, total))
    }

    /// 管理员重试死信消息
    pub async fn retry_dead_letter(&self, id: i64, success: bool) -> AppResult<()> {
        let mut dead_letter = self
            .repo
            .get_dead_letter(id)
            .await?
            .ok_or_else(|| crate::error::AppError::NotFound)?;

        dead_letter.mark_retried(success);
        self.repo.update_dead_letter(&dead_letter).await?;

        tracing::info!(
            "Retried dead letter: id={}, message_id={}, success={}",
            id, dead_letter.message_id, success
        );

        Ok(())
    }

    /// 放弃死信消息
    pub async fn abandon_dead_letter(&self, id: i64) -> AppResult<()> {
        let mut dead_letter = self
            .repo
            .get_dead_letter(id)
            .await?
            .ok_or_else(|| crate::error::AppError::NotFound)?;

        dead_letter.mark_abandoned();
        self.repo.update_dead_letter(&dead_letter).await?;

        tracing::info!(
            "Abandoned dead letter: id={}, message_id={}",
            id, dead_letter.message_id
        );

        Ok(())
    }

    /// 删除死信消息
    pub async fn delete_dead_letter(&self, id: i64) -> AppResult<()> {
        self.repo.delete_dead_letter(id).await?;
        tracing::info!("Deleted dead letter: id={}", id);
        Ok(())
    }

    /// 获取死信统计
    pub async fn get_stats(&self) -> AppResult<DLQStats> {
        let pending = self.repo.count_dead_letters(Some(0)).await?;
        let retried = self.repo.count_dead_letters(Some(1)).await?;
        let abandoned = self.repo.count_dead_letters(Some(2)).await?;
        let total = self.repo.count_dead_letters(None).await?;

        Ok(DLQStats {
            pending,
            retried,
            abandoned,
            total,
        })
    }
}

/// 死信队列统计
#[derive(Debug, Clone)]
pub struct DLQStats {
    pub pending: i64,
    pub retried: i64,
    pub abandoned: i64,
    pub total: i64,
}
