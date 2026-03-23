use crate::{
    error::{AppError, AppResult},
    models::message::{CreateMessageRequest, Message},
    repositories::message_repository::MessageRepository,
    services::{push_service::PushService, target_resolver::TargetResolver, template_service::TemplateService},
    websocket::WebSocketManager,
};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct MessageService {
    db: PgPool,
    redis: redis::aio::ConnectionManager,
    repo: MessageRepository,
    template_service: TemplateService,
    target_resolver: TargetResolver,
    ws_manager: Arc<RwLock<WebSocketManager>>,
}

impl MessageService {
    pub fn new(
        db: PgPool,
        redis: redis::aio::ConnectionManager,
        ws_manager: Arc<RwLock<WebSocketManager>>,
    ) -> Self {
        Self {
            db: db.clone(),
            redis: redis.clone(),
            repo: MessageRepository::new(db.clone()),
            template_service: TemplateService::new(db.clone()),
            target_resolver: TargetResolver::new(db, redis),
            ws_manager,
        }
    }

    pub async fn send_message(
        &self,
        tenant_id: i64,
        sender_id: i64,
        request: CreateMessageRequest,
    ) -> AppResult<i64> {
        // 1. Get template
        let template = self
            .template_service
            .get_by_code(&request.template_code)
            .await?
            .ok_or_else(|| AppError::NotFound)?;

        // 2. Render message content
        let title = self.render_template(
            &template.title_template.unwrap_or_default(),
            &request.variables,
        )?;
        let content = self.render_template(
            &template.content_template.unwrap_or_default(),
            &request.variables,
        )?;

        // 3. Create message
        let message_code = format!("MSG_{}", Uuid::new_v4());
        let send_type = request.send_type.unwrap_or(1);

        let message_id = self
            .repo
            .create(
                tenant_id,
                message_code.clone(),
                Some(template.id),
                template.category.clone(),
                template.priority,
                title,
                Some(content),
                template.jump_type.clone(),
                template.jump_params.clone(),
                None,
                send_type,
                request.scheduled_at,
                Some(sender_id),
            )
            .await?;

        // 4. Save target rules
        for rule in request.target_rules {
            self.repo
                .create_target_rule(
                    message_id,
                    rule.target_type,
                    rule.target_scope,
                    rule.filter_conditions,
                )
                .await?;
        }

        Ok(message_id)
    }

    pub async fn process_message(&self, message_id: i64) -> AppResult<Message> {
        // 1. Get message
        let message = self
            .repo
            .get_by_id(message_id)
            .await?
            .ok_or(AppError::NotFound)?;

        // 2. Get target rules
        let rules = self.repo.get_target_rules(message_id).await?;

        // 3. Resolve target users
        let user_ids = self
            .target_resolver
            .resolve_target_users(message.tenant_id, rules)
            .await?;

        tracing::info!("Message {} target users: {}", message_id, user_ids.len());

        // 4. Create user t_sys_messages
        self.repo.create_user_messages(message_id, &user_ids).await?;

        // 5. Push via channels
        let push_service = PushService::with_default_channels(
            self.db.clone(),
            self.redis.clone(),
            self.ws_manager.clone(),
        );
        push_service.push_to_users(&message, user_ids.clone()).await?;

        // 6. Update message status
        self.repo
            .update_status(message_id, 1, Some(chrono::Utc::now()))
            .await?;

        Ok(message)
    }

    #[allow(dead_code)]
    pub async fn get_message(&self, id: i64) -> AppResult<Option<Message>> {
        self.repo.get_by_id(id).await
    }

    #[allow(dead_code)]
    pub async fn cancel_message(&self, id: i64) -> AppResult<()> {
        self.repo.cancel_message(id).await
    }

    fn render_template(
        &self,
        template: &str,
        variables: &serde_json::Value,
    ) -> AppResult<String> {
        if template.is_empty() {
            return Ok(String::new());
        }

        let mut tera = tera::Tera::default();
        tera.add_raw_template("msg", template)
            .map_err(|e| AppError::TemplateRender(e.to_string()))?;

        let context = tera::Context::from_serialize(variables)
            .map_err(|e| AppError::TemplateRender(e.to_string()))?;

        tera.render("msg", &context)
            .map_err(|e| AppError::TemplateRender(e.to_string()))
    }
}
