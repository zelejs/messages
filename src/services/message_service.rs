use crate::{
    channel::{ChannelManager, LogFileChannel, MessageChannel},
    config::{ChannelConfig, RetryConfig},
    error::{AppError, AppResult},
    models::message::{CreateMessageRequest, DispatchTarget, Message, MessageType},
    repositories::message_repository::MessageRepository,
    services::{push_service::PushService, target_resolver::TargetResolver, template_service::TemplateService},
    websocket::WebSocketManager,
};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone)]
pub struct MessageService {
    db: PgPool,
    redis: redis::aio::ConnectionManager,
    repo: MessageRepository,
    template_service: TemplateService,
    target_resolver: TargetResolver,
    ws_manager: Arc<RwLock<WebSocketManager>>,
    channel_config: ChannelConfig,
    retry_config: RetryConfig,
}

impl MessageService {
    pub fn new(
        db: PgPool,
        redis: redis::aio::ConnectionManager,
        ws_manager: Arc<RwLock<WebSocketManager>>,
        channel_config: ChannelConfig,
        retry_config: RetryConfig,
    ) -> Self {
        Self {
            db: db.clone(),
            redis: redis.clone(),
            repo: MessageRepository::new(db.clone()),
            template_service: TemplateService::new(db.clone()),
            target_resolver: TargetResolver::new(db, redis),
            ws_manager,
            channel_config,
            retry_config,
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

        // 3. Build extra_data with source and target information
        let target_orgs: Vec<i64> = request.target_rules.iter()
            .filter_map(|r| r.target_scope.get("org_ids"))
            .filter_map(|v| v.as_array())
            .flat_map(|arr| arr.iter().filter_map(|v| v.as_i64()))
            .collect();

        let target_roles: Vec<String> = request.target_rules.iter()
            .filter_map(|r| r.target_scope.get("role_codes"))
            .filter_map(|v| v.as_array())
            .flat_map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())))
            .collect();

        let target_users: Vec<i64> = request.target_rules.iter()
            .filter_map(|r| r.target_scope.get("user_ids"))
            .filter_map(|v| v.as_array())
            .flat_map(|arr| arr.iter().filter_map(|v| v.as_i64()))
            .collect();

        let extra_data = serde_json::json!({
            "source_type": request.source_type.to_string(),
            "source_detail": request.source_detail,
            "msg_type": request.msg_type.to_string(),
            "target_orgs": target_orgs,
            "target_roles": target_roles,
            "target_users": target_users,
        });

        // 4. Create message
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
                Some(extra_data),
                send_type,
                request.scheduled_at,
                Some(sender_id),
                request.source_type.to_string(),
                Some(request.source_detail.clone()),
            )
            .await?;

        // 5. Save target rules
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

        tracing::info!(
            "Message created: id={}, code={}, source={:?}, type={:?}",
            message_id,
            message_code,
            request.source_type,
            request.msg_type
        );

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

        // 5. Setup channel manager with LogFileChannel as default stub
        let channel_manager = ChannelManager::new();
        // Note: LogFileChannel is used as a stub for logging all message dispatches
        // In production, this would include WebSocket, Email, DingTalk channels

        // 6. Push via channels (stub: LogFileChannel)
        // Parse msg_type from extra_data
        let msg_type = if let Some(extra) = &message.extra_data {
            extra.get("msg_type")
                .and_then(|v| v.as_str())
                .map(|s| match s {
                    "system_announcement" => MessageType::SystemAnnouncement,
                    "system_security" => MessageType::SystemSecurity,
                    "system_maintenance" => MessageType::SystemMaintenance,
                    "org_department" => MessageType::OrgDepartment,
                    "org_change" => MessageType::OrgChange,
                    "org_activity" => MessageType::OrgActivity,
                    "workflow_todo" => MessageType::WorkflowTodo,
                    "workflow_result" => MessageType::WorkflowResult,
                    "workflow_cc" => MessageType::WorkflowCc,
                    _ => MessageType::Other,
                })
                .unwrap_or(MessageType::Other)
        } else {
            MessageType::Other
        };

        // Create LogFileChannel and dispatch to each user
        let log_channel = LogFileChannel::default_with_dir();
        for user_id in &user_ids {
            let target = DispatchTarget {
                user_id: *user_id,
                org_id: None,
                role_codes: vec![],
                channels: vec!["log".to_string()],
            };

            match log_channel.send(&message, &target).await {
                Ok(result) => {
                    tracing::debug!("Channel result for user {}: {:?}", user_id, result);
                }
                Err(e) => {
                    tracing::error!("Failed to dispatch to user {}: {}", user_id, e);
                }
            }
        }

        // Also push via WebSocket for real-time delivery
        let push_service = PushService::with_default_channels(
            self.db.clone(),
            self.redis.clone(),
            self.ws_manager.clone(),
            self.channel_config.clone(),
            self.retry_config.clone(),
        );
        push_service.push_to_users(&message, user_ids.clone()).await?;

        // 7. Update message status
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

    /// 记录消息处理失败日志
    pub async fn log_message_failed(&self, message_id: i64, error_msg: &str) -> AppResult<()> {
        tracing::error!("Message {} failed: {}", message_id, error_msg);
        // 更新消息状态为失败 (status = 3)
        self.repo
            .update_status(message_id, 3, Some(chrono::Utc::now()))
            .await?;
        Ok(())
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
