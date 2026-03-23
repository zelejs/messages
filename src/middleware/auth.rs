use axum::{
    extract::{FromRequestParts, Request, State},
    http::request::Parts,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use async_trait::async_trait;

use crate::{config::Config, error::AppError, utils::jwt::JwtService};

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct AuthContext {
    pub user_id: i64,
    pub tenant_id: i64,
    pub org_id: Option<i64>,
    pub username: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthContext>()
            .cloned()
            .ok_or(AppError::Unauthorized)
    }
}

pub async fn auth_middleware(
    State(config): State<Arc<Config>>,
    mut req: Request,
    next: Next,
) -> Response {
    // Skip auth for health check and WebSocket
    let path = req.uri().path();
    if path == "/health" || path.starts_with("/ws/") {
        return next.run(req).await;
    }

    // Extract Authorization header
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => Some(&h[7..]),
        Some(_) => None,
        None => None,
    };

    match token {
        Some(token) => {
            let jwt_service = JwtService::new(&config.jwt_secret, config.jwt_expiration);

            match jwt_service.verify(token) {
                Ok(claims) => {
                    let user_id: i64 = claims.sub.parse().unwrap_or(0);

                    let auth_ctx = AuthContext {
                        user_id,
                        tenant_id: claims.tenant_id,
                        org_id: claims.org_id,
                        username: claims.sub.clone(),
                    };

                    req.extensions_mut().insert(auth_ctx);
                    next.run(req).await
                }
                Err(_) => {
                    tracing::warn!("Invalid JWT token");
                    AppError::Unauthorized.into_response()
                }
            }
        }
        None => {
            tracing::warn!("Missing Authorization header");
            AppError::Unauthorized.into_response()
        }
    }
}
