use axum::{
    extract::State,
    Json,
};
use std::sync::Arc;

use crate::{
    error::AppResult,
    repositories::user_repository::UserRepository,
    AppState,
};

#[derive(serde::Deserialize)]
pub struct UpdateSettingsRequest {
    pub category: Option<String>,
    pub web_enabled: Option<bool>,
    pub email_enabled: Option<bool>,
    pub dingtalk_enabled: Option<bool>,
    pub do_not_disturb: Option<bool>,
    pub dnd_start_time: Option<String>,
    pub dnd_end_time: Option<String>,
}

pub async fn get_settings(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<serde_json::Value>> {
    // TODO: Extract user_id from AuthContext
    let user_id = 1;

    let user_repo = UserRepository::new(state.db.clone());
    let settings = user_repo.get_message_settings(user_id).await?;

    Ok(Json(serde_json::json!({ "settings": settings })))
}

pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    Json(request): Json<UpdateSettingsRequest>,
) -> AppResult<Json<serde_json::Value>> {
    // TODO: Extract user_id from AuthContext
    let user_id = 1;

    let dnd_start = request
        .dnd_start_time
        .and_then(|t| chrono::NaiveTime::parse_from_str(&t, "%H:%M:%S").ok());

    let dnd_end = request
        .dnd_end_time
        .and_then(|t| chrono::NaiveTime::parse_from_str(&t, "%H:%M:%S").ok());

    let user_repo = UserRepository::new(state.db.clone());

    user_repo
        .upsert_message_settings(
            user_id,
            request.category,
            request.web_enabled.map(|b| b as i16),
            request.email_enabled.map(|b| b as i16),
            request.dingtalk_enabled.map(|b| b as i16),
            request.do_not_disturb.map(|b| b as i16),
            dnd_start,
            dnd_end,
        )
        .await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn update_dnd(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    // TODO: Extract user_id from AuthContext
    let user_id = 1;

    let enabled = payload
        .get("enabled")
        .and_then(|v| v.as_bool())
        .ok_or_else(|| crate::error::AppError::BadRequest("enabled is required".to_string()))?;

    let start_time = payload.get("start_time").and_then(|v| v.as_str()).and_then(
        |t| chrono::NaiveTime::parse_from_str(t, "%H:%M:%S").ok(),
    );

    let end_time = payload.get("end_time").and_then(|v| v.as_str()).and_then(|t| chrono::NaiveTime::parse_from_str(t, "%H:%M:%S").ok());

    let user_repo = UserRepository::new(state.db.clone());

    user_repo
        .upsert_message_settings(user_id, None, None, None, None, Some(enabled as i16), start_time, end_time)
        .await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn update_channels(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    // TODO: Extract user_id from AuthContext
    let user_id = 1;

    let category = payload.get("category").and_then(|v| v.as_str());

    let web_enabled = payload.get("web_enabled").and_then(|v| v.as_bool());
    let email_enabled = payload.get("email_enabled").and_then(|v| v.as_bool());
    let dingtalk_enabled = payload.get("dingtalk_enabled").and_then(|v| v.as_bool());

    let user_repo = UserRepository::new(state.db.clone());

    user_repo
        .upsert_message_settings(
            user_id,
            category.map(|s| s.to_string()),
            web_enabled.map(|b| b as i16),
            email_enabled.map(|b| b as i16),
            dingtalk_enabled.map(|b| b as i16),
            None,
            None,
            None,
        )
        .await?;

    Ok(Json(serde_json::json!({ "success": true })))
}
