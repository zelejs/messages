use crate::{
    error::{AppError, AppResult},
    models::message::{TargetRule, TargetScope},
    repositories::{
        organization_repository::OrganizationRepository,
        user_repository::UserRepository,
    },
};
use sqlx::PgPool;
use std::collections::HashSet;

#[derive(Clone)]
pub struct TargetResolver {
    user_repo: UserRepository,
    org_repo: OrganizationRepository,
}

impl TargetResolver {
    pub fn new(db: PgPool, _redis: redis::aio::ConnectionManager) -> Self {
        Self {
            user_repo: UserRepository::new(db.clone()),
            org_repo: OrganizationRepository::new(db),
        }
    }

    pub async fn resolve_target_users(
        &self,
        tenant_id: i64,
        rules: Vec<TargetRule>,
    ) -> AppResult<Vec<i64>> {
        let mut all_user_ids = HashSet::new();

        for rule in rules {
            let scope: TargetScope = serde_json::from_value(rule.target_scope)
                .map_err(|e| AppError::BadRequest(e.to_string()))?;

            let user_ids = match rule.target_type.as_str() {
                "user" => self.resolve_user_target(&scope).await?,
                "org" => self.resolve_org_target(tenant_id, &scope).await?,
                "role" => self.resolve_role_target(tenant_id, &scope).await?,
                "custom" => self.resolve_custom_target(tenant_id, &scope).await?,
                _ => vec![],
            };

            // Apply filter conditions
            let filtered_ids = if let Some(filter) = rule.filter_conditions {
                self.apply_filters(tenant_id, user_ids, filter).await?
            } else {
                user_ids
            };

            all_user_ids.extend(filtered_ids);
        }

        Ok(all_user_ids.into_iter().collect())
    }

    async fn resolve_user_target(&self, scope: &TargetScope) -> AppResult<Vec<i64>> {
        Ok(scope.user_ids.clone().unwrap_or_default())
    }

    async fn resolve_org_target(
        &self,
        tenant_id: i64,
        scope: &TargetScope,
    ) -> AppResult<Vec<i64>> {
        let org_ids = scope.org_ids.clone().unwrap_or_default();
        let include_children = scope.include_children.unwrap_or(false);

        let mut all_org_ids = org_ids.clone();

        if include_children {
            for org_id in org_ids {
                let children = self.org_repo.get_children(tenant_id, org_id).await?;
                all_org_ids.extend(children);
            }
        }

        self.user_repo.get_by_organizations(&all_org_ids).await
    }

    async fn resolve_role_target(
        &self,
        tenant_id: i64,
        scope: &TargetScope,
    ) -> AppResult<Vec<i64>> {
        let role_codes = scope.role_codes.clone().unwrap_or_default();
        self.user_repo.get_by_roles(tenant_id, &role_codes).await
    }

    async fn resolve_custom_target(
        &self,
        tenant_id: i64,
        scope: &TargetScope,
    ) -> AppResult<Vec<i64>> {
        if let Some(condition) = &scope.condition {
            self.user_repo.get_by_custom_condition(tenant_id, condition).await
        } else {
            Ok(vec![])
        }
    }

    async fn apply_filters(
        &self,
        _tenant_id: i64,
        user_ids: Vec<i64>,
        _filter: serde_json::Value,
    ) -> AppResult<Vec<i64>> {
        // TODO: Implement filter logic
        // Examples: filter by user status, last login time, custom attributes, etc.
        Ok(user_ids)
    }
}
