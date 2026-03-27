use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    error::AppResult,
    models::{message::Message, retry::DLQQuery},
    repositories::{message_repository::MessageRepository, retry_repository::RetryRepository},
    services::retry_service::{DLQService, DLQStats},
    utils::pagination::PaginatedResponse,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct AdminMessageQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct MessageDetailResponse {
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
    pub send_type: i16,
    pub scheduled_at: Option<String>,
    pub sent_at: Option<String>,
    pub sender_id: Option<i64>,
    pub sender_type: String,
    pub status: i16,
    pub created_at: String,
    pub updated_at: String,
    pub target_users_count: i64,
    pub read_count: i64,
}

#[derive(Debug, Serialize)]
pub struct PushLogResponse {
    pub id: i64,
    pub message_id: i64,
    pub user_id: i64,
    pub channel: String,
    pub status: i16,
    pub error_msg: Option<String>,
    pub pushed_at: String,
}

pub async fn list_all_messages(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AdminMessageQuery>,
) -> AppResult<Json<PaginatedResponse<Message>>> {
    // TODO: Extract tenant_id from AuthContext
    let tenant_id = 1;

    let repo = MessageRepository::new(state.db.clone());

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let t_sys_messages = repo.list_messages(tenant_id, page, page_size).await?;

    let total = repo.count_messages(tenant_id).await?;

    Ok(Json(PaginatedResponse::new(t_sys_messages, total, page, page_size)))
}

pub async fn get_message_details(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> AppResult<Json<MessageDetailResponse>> {
    let repo = MessageRepository::new(state.db.clone());

    let message = repo.get_by_id(id).await?.ok_or(crate::error::AppError::NotFound)?;

    // TODO: Get target users count and read count
    let target_users_count = 0;
    let read_count = 0;

    Ok(Json(MessageDetailResponse {
        id: message.id,
        tenant_id: message.tenant_id,
        message_code: message.message_code,
        template_id: message.template_id,
        category: message.category,
        priority: message.priority,
        title: message.title,
        content: message.content,
        jump_type: message.jump_type,
        jump_params: message.jump_params,
        send_type: message.send_type,
        scheduled_at: message.scheduled_at.map(|t| t.to_rfc3339()),
        sent_at: message.sent_at.map(|t| t.to_rfc3339()),
        sender_id: message.sender_id,
        sender_type: message.sender_type,
        status: message.status,
        created_at: message.created_at.to_rfc3339(),
        updated_at: message.updated_at.to_rfc3339(),
        target_users_count,
        read_count,
    }))
}

pub async fn get_push_logs(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<i64>,
) -> AppResult<Json<Vec<PushLogResponse>>> {
    // TODO: Query t_sys_message_push_logs table
    // For now, return empty vector
    Ok(Json(vec![]))
}

pub async fn revoke_message(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let repo = MessageRepository::new(state.db.clone());

    // Mark message as cancelled
    repo.cancel_message(id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn cancel_message(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let repo = MessageRepository::new(state.db.clone());

    // Cancel scheduled message
    repo.cancel_message(id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn retry_message(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    // TODO: Re-publish message to queue
    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(Debug, Serialize)]
pub struct MessageStats {
    pub total_messages: i64,
    pub sent_today: i64,
    pub pending: i64,
    pub failed: i64,
    pub by_category: Vec<CategoryStats>,
}

#[derive(Debug, Serialize)]
pub struct CategoryStats {
    pub category: String,
    pub count: i64,
}

pub async fn get_stats(
    State(_state): State<Arc<AppState>>,
) -> AppResult<Json<MessageStats>> {
    // TODO: Query statistics from database
    Ok(Json(MessageStats {
        total_messages: 0,
        sent_today: 0,
        pending: 0,
        failed: 0,
        by_category: vec![],
    }))
}

// ==================== Dead Letter Queue (DLQ) Admin APIs ====================

#[derive(Debug, Deserialize)]
pub struct DLQListQuery {
    pub status: Option<i16>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct DeadLetterResponse {
    pub id: i64,
    pub message_id: i64,
    pub user_id: i64,
    pub channel: String,
    pub failed_reason: Option<String>,
    pub retry_history: Option<serde_json::Value>,
    pub status: i16,
    pub retried_at: Option<String>,
    pub retried_success: i16,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct DLQListResponse {
    pub data: Vec<DeadLetterResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Serialize)]
pub struct DLQStatsResponse {
    pub pending: i64,
    pub retried: i64,
    pub abandoned: i64,
    pub total: i64,
}

/// List dead letter messages
pub async fn list_dead_letters(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DLQListQuery>,
) -> AppResult<Json<DLQListResponse>> {
    let dlq_service = DLQService::new(RetryRepository::new(state.db.clone()));

    let dlq_query = DLQQuery {
        status: query.status,
        page: query.page.unwrap_or(1),
        page_size: query.page_size.unwrap_or(20),
    };

    let (dead_letters, total) = dlq_service.list_dead_letters(&dlq_query).await?;

    let data: Vec<DeadLetterResponse> = dead_letters
        .into_iter()
        .map(|dl| DeadLetterResponse {
            id: dl.id,
            message_id: dl.message_id,
            user_id: dl.user_id,
            channel: dl.channel,
            failed_reason: dl.failed_reason,
            retry_history: dl.retry_history,
            status: dl.status,
            retried_at: dl.retried_at.map(|t| t.to_rfc3339()),
            retried_success: dl.retried_success,
            created_at: dl.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(DLQListResponse {
        data,
        total,
        page: dlq_query.page,
        page_size: dlq_query.page_size,
    }))
}

/// Get DLQ statistics
pub async fn get_dlq_stats(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<DLQStatsResponse>> {
    let dlq_service = DLQService::new(RetryRepository::new(state.db.clone()));
    let stats: DLQStats = dlq_service.get_stats().await?;

    Ok(Json(DLQStatsResponse {
        pending: stats.pending,
        retried: stats.retried,
        abandoned: stats.abandoned,
        total: stats.total,
    }))
}

/// Retry a dead letter message
pub async fn retry_dead_letter(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let dlq_service = DLQService::new(RetryRepository::new(state.db.clone()));

    // TODO: Actually retry the message through the original channel
    // For now, just mark it as retried
    dlq_service.retry_dead_letter(id, true).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Dead letter marked for retry"
    })))
}

/// Abandon a dead letter message
pub async fn abandon_dead_letter(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let dlq_service = DLQService::new(RetryRepository::new(state.db.clone()));

    dlq_service.abandon_dead_letter(id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Dead letter abandoned"
    })))
}

/// Delete a dead letter message
pub async fn delete_dead_letter(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let dlq_service = DLQService::new(RetryRepository::new(state.db.clone()));

    dlq_service.delete_dead_letter(id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Dead letter deleted"
    })))
}
