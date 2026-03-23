use axum::{
    extract::{Path, Query, State},
    Json,
};
use std::sync::Arc;

use crate::{
    error::{AppError, AppResult},
    middleware::AuthContext,
    models::message::{CreateMessageRequest, MessageListQuery, UserMessageDetail},
    queue::producer::MessageProducer,
    repositories::message_repository::MessageRepository,
    services::message_service::MessageService,
    utils::pagination::PaginatedResponse,
    AppState,
};

pub async fn send_message(
    State(state): State<Arc<AppState>>,
    _auth: AuthContext,
    Json(request): Json<CreateMessageRequest>,
) -> AppResult<Json<serde_json::Value>> {
    // TODO: Extract tenant_id and user_id from AuthContext
    let tenant_id = 1;
    let user_id = 1;

    let producer = MessageProducer::new(&state.config).await?;
    let service = MessageService::new(state.db.clone(), state.redis.clone());

    let message_id = service
        .send_message(tenant_id, user_id, request)
        .await?;

    // Publish to queue
    producer.publish_message(message_id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message_id": message_id
    })))
}

pub async fn list_user_messages(
    State(state): State<Arc<AppState>>,
    _auth: AuthContext,
    Query(query): Query<MessageListQuery>,
) -> AppResult<Json<PaginatedResponse<UserMessageDetail>>> {
    // TODO: Extract user_id from AuthContext
    let user_id = 1;

    let repo = MessageRepository::new(state.db.clone());

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let messages = repo
        .list_user_messages(
            user_id,
            query.category.as_deref(),
            query.is_read,
            page,
            page_size,
        )
        .await?;

    let total = repo
        .count_user_messages(user_id, query.category.as_deref(), query.is_read)
        .await?;

    Ok(Json(PaginatedResponse::new(messages, total, page, page_size)))
}

pub async fn get_message_detail(
    State(state): State<Arc<AppState>>,
    _auth: AuthContext,
    Path(id): Path<i64>,
) -> AppResult<Json<UserMessageDetail>> {
    let repo = MessageRepository::new(state.db.clone());

    // TODO: Get user_id from AuthContext and verify ownership
    let user_id = 1;

    let messages = repo.list_user_messages(user_id, None, None, 1, 1000).await?;

    let message = messages
        .into_iter()
        .find(|m| m.id == id)
        .ok_or(AppError::NotFound)?;

    Ok(Json(message))
}

pub async fn mark_as_read(
    State(state): State<Arc<AppState>>,
    _auth: AuthContext,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = 1;

    let repo = MessageRepository::new(state.db.clone());
    repo.mark_as_read(id, user_id).await?;

    // TODO: Update Redis unread count

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn batch_mark_as_read(
    State(state): State<Arc<AppState>>,
    _auth: AuthContext,
    Json(payload): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = 1;

    let message_ids: Vec<i64> = serde_json::from_value(
        payload
            .get("message_ids")
            .cloned()
            .unwrap_or_default(),
    )?;

    let repo = MessageRepository::new(state.db.clone());
    repo.batch_mark_as_read(&message_ids, user_id).await?;

    // TODO: Update Redis unread count

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn mark_category_as_read(
    State(state): State<Arc<AppState>>,
    _auth: AuthContext,
    Json(payload): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = 1;
    let category = payload
        .get("category")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("category is required".to_string()))?;

    let repo = MessageRepository::new(state.db.clone());
    repo.mark_all_as_read(user_id, Some(category)).await?;

    // TODO: Update Redis unread count

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn mark_all_as_read(
    State(state): State<Arc<AppState>>,
    _auth: AuthContext,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = 1;

    let repo = MessageRepository::new(state.db.clone());
    repo.mark_all_as_read(user_id, None).await?;

    // TODO: Reset Redis unread count

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn delete_message(
    State(state): State<Arc<AppState>>,
    _auth: AuthContext,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = 1;

    let repo = MessageRepository::new(state.db.clone());
    repo.delete_message(id, user_id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn batch_delete(
    State(state): State<Arc<AppState>>,
    _auth: AuthContext,
    Json(payload): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = 1;

    let message_ids: Vec<i64> = serde_json::from_value(
        payload
            .get("message_ids")
            .cloned()
            .unwrap_or_default(),
    )?;

    let repo = MessageRepository::new(state.db.clone());
    repo.batch_delete(&message_ids, user_id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn pin_message(
    State(state): State<Arc<AppState>>,
    _auth: AuthContext,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = 1;

    let repo = MessageRepository::new(state.db.clone());
    repo.pin_message(id, user_id, true).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn unpin_message(
    State(state): State<Arc<AppState>>,
    _auth: AuthContext,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = 1;

    let repo = MessageRepository::new(state.db.clone());
    repo.pin_message(id, user_id, false).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn get_unread_count(
    State(state): State<Arc<AppState>>,
    _auth: AuthContext,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = 1;

    let repo = MessageRepository::new(state.db.clone());
    let count = repo.count_user_messages(user_id, None, Some(0)).await?;

    // TODO: Get from Redis cache

    Ok(Json(serde_json::json!({ "count": count })))
}

pub async fn get_unread_stats(
    State(_state): State<Arc<AppState>>,
    _auth: AuthContext,
) -> AppResult<Json<serde_json::Value>> {
    let _user_id = 1;

    // TODO: Get from Redis cache

    Ok(Json(serde_json::json!({
        "total": 0,
        "by_category": {}
    })))
}

pub async fn get_org_tree(
    State(state): State<Arc<AppState>>,
    _auth: AuthContext,
) -> AppResult<Json<serde_json::Value>> {
    // TODO: Extract tenant_id from AuthContext
    let tenant_id = 1;

    let org_repo = crate::repositories::organization_repository::OrganizationRepository::new(
        state.db.clone(),
    );

    let tree = org_repo.get_tree(tenant_id).await?;

    Ok(Json(serde_json::json!(tree)))
}

pub async fn get_org_users(
    State(state): State<Arc<AppState>>,
    _auth: AuthContext,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let org_repo = crate::repositories::organization_repository::OrganizationRepository::new(
        state.db.clone(),
    );

    let user_ids = org_repo.get_org_users(id).await?;

    Ok(Json(serde_json::json!({ "user_ids": user_ids })))
}
