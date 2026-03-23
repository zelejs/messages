use crate::{
    error::{AppError, AppResult},
    models::message_template::{CreateTemplateRequest, MessageTemplate, UpdateTemplateRequest},
    repositories::template_repository::TemplateRepository,
};
use sqlx::PgPool;

pub struct TemplateService {
    repo: TemplateRepository,
}

impl TemplateService {
    pub fn new(db: PgPool) -> Self {
        Self {
            repo: TemplateRepository::new(db),
        }
    }

    pub async fn create(&self, request: CreateTemplateRequest) -> AppResult<i64> {
        // Check if template code already exists
        if let Some(_) = self.repo.get_by_code(&request.template_code).await? {
            return Err(AppError::BadRequest("Template code already exists".to_string()));
        }

        self.repo.create(request).await
    }

    pub async fn get_by_id(&self, id: i64) -> AppResult<Option<MessageTemplate>> {
        self.repo.get_by_id(id).await
    }

    pub async fn get_by_code(&self, code: &str) -> AppResult<Option<MessageTemplate>> {
        self.repo.get_by_code(code).await
    }

    pub async fn list(
        &self,
        category: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> AppResult<Vec<MessageTemplate>> {
        self.repo.list(category, page, page_size).await
    }

    pub async fn count(&self, category: Option<&str>) -> AppResult<i64> {
        self.repo.count(category).await
    }

    pub async fn update(&self, id: i64, request: UpdateTemplateRequest) -> AppResult<()> {
        // Check if template exists
        let template = self
            .repo
            .get_by_id(id)
            .await?
            .ok_or(AppError::NotFound)?;

        // Prevent modification of system templates
        if template.is_system == 1 {
            return Err(AppError::BadRequest("Cannot modify system templates".to_string()));
        }

        self.repo.update(id, request).await
    }

    pub async fn delete(&self, id: i64) -> AppResult<()> {
        // Check if template exists
        let template = self
            .repo
            .get_by_id(id)
            .await?
            .ok_or(AppError::NotFound)?;

        // Prevent deletion of system templates
        if template.is_system == 1 {
            return Err(AppError::BadRequest("Cannot delete system templates".to_string()));
        }

        self.repo.delete(id).await
    }
}
