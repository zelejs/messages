use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    error::{AppError, AppResult},
    models::message_template::{CreateTemplateRequest, MessageTemplate, UpdateTemplateRequest},
    services::template_service::TemplateService,
    utils::pagination::PaginatedResponse,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct TemplateListQuery {
    pub category: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct TemplateResponse {
    pub id: i64,
    pub template_code: String,
    pub template_name: String,
    pub category: String,
    pub priority: i16,
    pub title_template: Option<String>,
    pub content_template: Option<String>,
    pub jump_type: Option<String>,
    pub jump_params: Option<serde_json::Value>,
    pub channels: Option<Vec<String>>,
    pub is_system: i16,
    pub created_at: String,
    pub updated_at: String,
}

impl From<MessageTemplate> for TemplateResponse {
    fn from(t: MessageTemplate) -> Self {
        Self {
            id: t.id,
            template_code: t.template_code,
            template_name: t.template_name,
            category: t.category,
            priority: t.priority,
            title_template: t.title_template,
            content_template: t.content_template,
            jump_type: t.jump_type,
            jump_params: t.jump_params,
            channels: t.channels.and_then(|v| serde_json::from_value(v).ok()),
            is_system: t.is_system,
            created_at: t.created_at.to_rfc3339(),
            updated_at: t.updated_at.to_rfc3339(),
        }
    }
}

pub async fn create_template(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateTemplateRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let service = TemplateService::new(state.db.clone());

    let template_id = service.create(request).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "template_id": template_id
    })))
}

pub async fn list_templates(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TemplateListQuery>,
) -> AppResult<Json<PaginatedResponse<TemplateResponse>>> {
    let service = TemplateService::new(state.db.clone());

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let templates = service
        .list(query.category.as_deref(), page, page_size)
        .await?;

    let total = service.count(query.category.as_deref()).await?;

    let response = PaginatedResponse::new(
        templates.into_iter().map(TemplateResponse::from).collect(),
        total,
        page,
        page_size,
    );

    Ok(Json(response))
}

pub async fn get_template(
    State(state): State<Arc<AppState>>,
    Path(code): Path<String>,
) -> AppResult<Json<TemplateResponse>> {
    let service = TemplateService::new(state.db.clone());

    let template = service
        .get_by_code(&code)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(TemplateResponse::from(template)))
}

pub async fn get_template_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> AppResult<Json<TemplateResponse>> {
    let service = TemplateService::new(state.db.clone());

    let template = service.get_by_id(id).await?.ok_or(AppError::NotFound)?;

    Ok(Json(TemplateResponse::from(template)))
}

pub async fn update_template(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(request): Json<UpdateTemplateRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let service = TemplateService::new(state.db.clone());

    service.update(id, request).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn delete_template(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let service = TemplateService::new(state.db.clone());

    service.delete(id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}
