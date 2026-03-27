use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 重试记录
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MessageRetryRecord {
    pub id: i64,
    pub message_id: i64,
    pub user_id: i64,
    pub channel: String,
    pub retry_count: i16,
    pub max_retries: i16,
    pub retry_intervals: serde_json::Value,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub retry_history: serde_json::Value,
    pub status: i16,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MessageRetryRecord {
    /// 创建新的重试记录
    pub fn new(
        message_id: i64,
        user_id: i64,
        channel: &str,
        max_retries: i16,
        retry_intervals: Vec<i32>,
    ) -> Self {
        let now = Utc::now();
        let next_retry_at = if !retry_intervals.is_empty() {
            Some(now + chrono::Duration::seconds(retry_intervals[0] as i64))
        } else {
            Some(now + chrono::Duration::seconds(60))
        };

        Self {
            id: 0,
            message_id,
            user_id,
            channel: channel.to_string(),
            retry_count: 0,
            max_retries,
            retry_intervals: serde_json::to_value(retry_intervals).unwrap_or_default(),
            next_retry_at,
            last_error: None,
            retry_history: serde_json::json!([]),
            status: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// 计算下次重试时间
    pub fn calculate_next_retry(&self) -> Option<DateTime<Utc>> {
        let intervals: Vec<i32> = serde_json::from_value(self.retry_intervals.clone()).unwrap_or_default();

        if self.retry_count as usize >= intervals.len() {
            None
        } else {
            let interval_seconds = intervals[self.retry_count as usize] as i64;
            Some(Utc::now() + chrono::Duration::seconds(interval_seconds))
        }
    }

    /// 记录一次重试
    pub fn record_attempt(&mut self, error: Option<&str>) {
        self.retry_count += 1;
        self.last_error = error.map(|s| s.to_string());

        let mut history: Vec<RetryAttempt> = serde_json::from_value(self.retry_history.clone()).unwrap_or_default();
        history.push(RetryAttempt {
            attempt: self.retry_count,
            time: Utc::now(),
            error: error.map(|s| s.to_string()),
        });
        self.retry_history = serde_json::to_value(history).unwrap_or_default();

        self.next_retry_at = self.calculate_next_retry();
        self.updated_at = Utc::now();
    }

    /// 检查是否还有重试机会
    pub fn has_more_retries(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// 标记为成功
    pub fn mark_success(&mut self) {
        self.status = 1;
        self.updated_at = Utc::now();
    }

    /// 标记为进入死信队列
    pub fn mark_dead_letter(&mut self) {
        self.status = 2;
        self.next_retry_at = None;
        self.updated_at = Utc::now();
    }
}

/// 单次重试尝试记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryAttempt {
    pub attempt: i16,
    pub time: DateTime<Utc>,
    pub error: Option<String>,
}

/// 死信消息
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeadLetterMessage {
    pub id: i64,
    pub message_id: i64,
    pub user_id: i64,
    pub channel: String,
    pub failed_reason: Option<String>,
    pub retry_history: Option<serde_json::Value>,
    pub status: i16,
    pub retried_at: Option<DateTime<Utc>>,
    pub retried_success: i16,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DeadLetterMessage {
    /// 从重试记录创建死信消息
    pub fn from_retry_record(record: &MessageRetryRecord) -> Self {
        Self {
            id: 0,
            message_id: record.message_id,
            user_id: record.user_id,
            channel: record.channel.clone(),
            failed_reason: record.last_error.clone(),
            retry_history: Some(record.retry_history.clone()),
            status: 0,
            retried_at: None,
            retried_success: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 标记为已重试
    pub fn mark_retried(&mut self, success: bool) {
        self.status = 1;
        self.retried_at = Some(Utc::now());
        self.retried_success = if success { 1 } else { 0 };
        self.updated_at = Utc::now();
    }

    /// 标记为已放弃
    pub fn mark_abandoned(&mut self) {
        self.status = 2;
        self.updated_at = Utc::now();
    }
}

/// 重试队列查询参数
#[derive(Debug, Clone)]
pub struct RetryQuery {
    pub status: Option<i16>,
    pub before_time: Option<DateTime<Utc>>,
    pub limit: i64,
}

impl Default for RetryQuery {
    fn default() -> Self {
        Self {
            status: Some(0), // 默认查询待重试的
            before_time: Some(Utc::now()),
            limit: 100,
        }
    }
}

/// 死信队列查询参数
#[derive(Debug, Clone)]
pub struct DLQQuery {
    pub status: Option<i16>,
    pub page: i64,
    pub page_size: i64,
}

impl Default for DLQQuery {
    fn default() -> Self {
        Self {
            status: Some(0), // 默认查询待处理的
            page: 1,
            page_size: 20,
        }
    }
}
