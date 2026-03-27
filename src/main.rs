mod cache;
mod channel;
mod config;
mod error;
mod handlers;
mod middleware;
mod models;
mod queue;
mod repositories;
mod services;
mod utils;
mod websocket;

use axum::{
    routing::{get, post, put, delete},
    Router,
};
use config::Config;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use websocket::WebSocketManager;

use crate::{
    handlers::{
        admin_handler::*,
        message_handler::*,
        setting_handler::*,
        template_handler::*,
    },
    middleware::auth::auth_middleware,
};

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub redis: redis::aio::ConnectionManager,
    pub ws_manager: Arc<RwLock<WebSocketManager>>,
    pub config: Arc<Config>,
    pub repos: Repos,
    pub channel_manager: Arc<channel::ChannelManager>,
}

#[derive(Clone)]
pub struct Repos {
    pub message: repositories::message_repository::MessageRepository,
    pub user: repositories::user_repository::UserRepository,
    pub org: repositories::organization_repository::OrganizationRepository,
    pub template: repositories::template_repository::TemplateRepository,
    pub retry: repositories::retry_repository::RetryRepository,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,message_system=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    dotenv::dotenv().ok();
    let config = Config::from_env()?;

    tracing::info!("Starting Message System...");

    // Connect to database
    let db = sqlx::PgPool::connect(&config.database_url).await?;
    tracing::info!("Connected to database");

    // Note: Run migrations manually with:
    // sqlx migrate run --source migrations

    // Connect to Redis
    let redis_client = redis::Client::open(config.redis_url.as_str())?;
    let redis = redis::aio::ConnectionManager::new(redis_client).await?;
    tracing::info!("Connected to Redis");

    // Create WebSocket manager
    let ws_manager = Arc::new(RwLock::new(WebSocketManager::new()));

    // Create repositories
    let repos = Repos {
        message: repositories::message_repository::MessageRepository::new(db.clone()),
        user: repositories::user_repository::UserRepository::new(db.clone()),
        org: repositories::organization_repository::OrganizationRepository::new(db.clone()),
        template: repositories::template_repository::TemplateRepository::new(db.clone()),
        retry: repositories::retry_repository::RetryRepository::new(db.clone()),
    };

    // Create channel manager and register channels
    let mut channel_manager = channel::ChannelManager::new();
    channel_manager.register_channel(Box::new(channel::LogFileChannel::default_with_dir()));
    let channel_manager = Arc::new(channel_manager);
    tracing::info!("Registered message channels: log_file");

    // Create application state
    let app_state = AppState {
        db: db.clone(),
        redis: redis.clone(),
        ws_manager: ws_manager.clone(),
        config: Arc::new(config.clone()),
        repos,
        channel_manager: channel_manager.clone(),
    };

    // Wrap state in Arc for sharing across handlers
    let state = Arc::new(app_state);

    // Start message queue consumer
    let consumer_config = config.clone();
    let consumer_db = db.clone();
    let consumer_redis = redis.clone();
    let consumer_ws_manager = ws_manager.clone();
    tokio::spawn(async move {
        if let Err(e) = queue::consumer::start_consumer(
            consumer_config,
            consumer_db,
            consumer_redis,
            consumer_ws_manager,
        )
        .await
        {
            tracing::error!("Message queue consumer error: {:?}", e);
        }
    });

    // Start retry worker
    let retry_db = db.clone();
    let retry_redis = redis.clone();
    let retry_ws_manager = ws_manager.clone();
    let retry_channel_config = config.channel_config.clone();
    let retry_config = config.retry_config.clone();
    tokio::spawn(async move {
        use crate::services::retry_service::RetryService;
        use crate::repositories::retry_repository::RetryRepository;

        let retry_repo = RetryRepository::new(retry_db.clone());
        let retry_service = std::sync::Arc::new(RetryService::new(retry_repo, retry_config.clone()));

        // Start the retry worker with a processor function
        let retry_config_clone = retry_config.clone();
        retry_service.clone().start_retry_worker(move |retry_record| {
            let db = retry_db.clone();
            let redis = retry_redis.clone();
            let ws_manager = retry_ws_manager.clone();
            let channel_config = retry_channel_config.clone();
            let retry_service_for_dlq = retry_service.clone();
            let retry_config = retry_config_clone.clone();

            async move {
                use crate::services::push_service::PushService;
                use crate::repositories::message_repository::MessageRepository;

                // Get the message
                let message_repo = MessageRepository::new(db.clone());
                let message = match message_repo.get_by_id(retry_record.message_id).await {
                    Ok(Some(msg)) => msg,
                    Ok(None) => {
                        tracing::warn!("Message not found for retry: message_id={}", retry_record.message_id);
                        return Ok(false);
                    }
                    Err(e) => {
                        tracing::error!("Failed to get message for retry: {:?}", e);
                        return Ok(false);
                    }
                };

                // Create push service and attempt retry
                let push_service = PushService::with_default_channels(
                    db,
                    redis,
                    ws_manager,
                    channel_config,
                    retry_config,
                );

                match push_service.process_retry(&message, retry_record.user_id, &retry_record.channel).await {
                    Ok(success) => {
                        let mut record = retry_record.clone();
                        if success {
                            record.mark_success();
                        } else {
                            if !record.has_more_retries() {
                                // Move to DLQ
                                retry_service_for_dlq.dlq_service().move_to_dlq(&record).await?;
                                record.mark_dead_letter();
                            }
                        }
                        let _ = retry_service_for_dlq.record_attempt(&mut record, None).await;
                        Ok(success)
                    }
                    Err(e) => {
                        tracing::error!("Failed to process retry: {:?}", e);
                        Ok(false)
                    }
                }
            }
        }).await;
    });

    tracing::info!("Retry worker started");

    // Build routes
    let app = Router::new()
        // Message template routes (business prefix: /api/message/)
        .route(
            "/api/message/templates",
            post(create_template).get(list_templates),
        )
        .route(
            "/api/message/templates/code/:code",
            get(get_template),
        )
        .route(
            "/api/message/templates/:id",
            get(get_template_by_id).put(update_template).delete(delete_template),
        )

        // Message sending routes (business prefix: /api/message/)
        .route("/api/message/send", post(send_message))

        // User message routes (business prefix: /api/message/)
        .route("/api/message/list", get(list_user_messages))
        .route("/api/message/:id", get(get_message_detail))
        .route("/api/message/:id/read", post(mark_as_read))
        .route("/api/message/batch-read", post(batch_mark_as_read))
        .route(
            "/api/message/read-by-category",
            post(mark_category_as_read),
        )
        .route("/api/message/read-all", post(mark_all_as_read))
        .route("/api/message/:id", delete(delete_message))
        .route("/api/message/batch-delete", post(batch_delete))
        .route("/api/message/:id/pin", post(pin_message))
        .route("/api/message/:id/pin", delete(unpin_message))
        .route("/api/message/unread-count", get(get_unread_count))
        .route("/api/message/unread-stats", get(get_unread_stats))

        // User settings routes (business prefix: /api/message/)
        .route("/api/message/settings", get(get_settings))
        .route("/api/message/settings", put(update_settings))
        .route("/api/message/settings/dnd", put(update_dnd))
        .route("/api/message/settings/channels", put(update_channels))

        // Admin routes (admin prefix: /api/adm/message/)
        .route("/api/adm/message/list", get(list_all_messages))
        .route("/api/adm/message/:id/details", get(get_message_details))
        .route("/api/adm/message/:id/push-logs", get(get_push_logs))
        .route("/api/adm/message/:id/revoke", post(revoke_message))
        .route("/api/adm/message/:id/cancel", post(cancel_message))
        .route("/api/adm/message/:id/retry", post(retry_message))
        .route("/api/adm/message/stats", get(get_stats))
        // DLQ Admin routes
        .route("/api/adm/message/dlq/list", get(list_dead_letters))
        .route("/api/adm/message/dlq/stats", get(get_dlq_stats))
        .route("/api/adm/message/dlq/:id/retry", post(retry_dead_letter))
        .route("/api/adm/message/dlq/:id/abandon", post(abandon_dead_letter))
        .route("/api/adm/message/dlq/:id", delete(delete_dead_letter))

        // Organization query routes (business prefix: /api/message/)
        .route("/api/message/org-tree", get(get_org_tree))
        .route("/api/message/org-users/:id", get(get_org_users))

        // WebSocket route
        .route("/ws/:tenant_id", get(websocket::handler::ws_handler))

        // Health check
        .route("/health", get(|| async { "OK" }))

        .layer(CorsLayer::permissive())
        .layer(axum::middleware::from_fn_with_state(
            state.config.clone(),
            auth_middleware,
        ))
        .with_state(state);

    // Start server
    let addr = format!("{}:{}", config.server_host, config.server_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
