use crate::{
    error::AppResult,
    models::message_template::{CreateTemplateRequest, MessageTemplate, UpdateTemplateRequest},
};
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub struct TemplateRepository {
    db: PgPool,
}

impl TemplateRepository {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn create(&self, request: CreateTemplateRequest) -> AppResult<i64> {
        let channels_json = request.channels.map(|c| serde_json::to_value(c).ok());

        let result = sqlx::query(
            r#"
            INSERT INTO t_sys_message_templates (
                template_code, template_name, category, priority,
                title_template, content_template, jump_type, jump_params, channels
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id
            "#
        )
        .bind(&request.template_code)
        .bind(&request.template_name)
        .bind(&request.category)
        .bind(request.priority.unwrap_or(2))
        .bind(&request.title_template)
        .bind(&request.content_template)
        .bind(&request.jump_type)
        .bind(&request.jump_params)
        .bind(&channels_json)
        .fetch_one(&self.db)
        .await?;

        Ok(result.get("id"))
    }

    pub async fn get_by_id(&self, id: i64) -> AppResult<Option<MessageTemplate>> {
        let template = sqlx::query_as("SELECT * FROM t_sys_message_templates WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.db)
            .await?;

        Ok(template)
    }

    pub async fn get_by_code(&self, code: &str) -> AppResult<Option<MessageTemplate>> {
        let template = sqlx::query_as("SELECT * FROM t_sys_message_templates WHERE template_code = $1")
            .bind(code)
            .fetch_optional(&self.db)
            .await?;

        Ok(template)
    }

    pub async fn list(
        &self,
        category: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> AppResult<Vec<MessageTemplate>> {
        let offset = (page - 1) * page_size;

        let templates = sqlx::query_as(
            r#"
            SELECT * FROM t_sys_message_templates
            WHERE ($1::VARCHAR IS NULL OR category = $1)
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(category)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        Ok(templates)
    }

    pub async fn count(&self, category: Option<&str>) -> AppResult<i64> {
        let result = sqlx::query(
            "SELECT COUNT(*) as count FROM t_sys_message_templates WHERE ($1::VARCHAR IS NULL OR category = $1)"
        )
        .bind(category)
        .fetch_one(&self.db)
        .await?;

        Ok(result.get("count"))
    }

    pub async fn update(&self, id: i64, request: UpdateTemplateRequest) -> AppResult<()> {
        let channels_json = request.channels.map(|c| serde_json::to_value(c).ok());

        sqlx::query(
            r#"
            UPDATE t_sys_message_templates
            SET
                template_name = COALESCE($1, template_name),
                category = COALESCE($2, category),
                priority = COALESCE($3, priority),
                title_template = COALESCE($4, title_template),
                content_template = COALESCE($5, content_template),
                jump_type = COALESCE($6, jump_type),
                jump_params = COALESCE($7, jump_params),
                channels = COALESCE($8, channels),
                updated_at = NOW()
            WHERE id = $9
            "#
        )
        .bind(&request.template_name)
        .bind(&request.category)
        .bind(request.priority)
        .bind(&request.title_template)
        .bind(&request.content_template)
        .bind(&request.jump_type)
        .bind(&request.jump_params)
        .bind(&channels_json)
        .bind(id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn delete(&self, id: i64) -> AppResult<()> {
        sqlx::query("DELETE FROM t_sys_message_templates WHERE id = $1")
            .bind(id)
            .execute(&self.db)
            .await?;

        Ok(())
    }
}
