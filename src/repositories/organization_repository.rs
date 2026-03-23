use crate::{
    error::AppResult,
    models::organization::{Organization, OrganizationTree},
};
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub struct OrganizationRepository {
    db: PgPool,
}

impl OrganizationRepository {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn get_by_id(&self, id: i64) -> AppResult<Option<Organization>> {
        let org = sqlx::query_as("SELECT * FROM organizations WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.db)
            .await?;

        Ok(org)
    }

    pub async fn get_by_tenant(&self, tenant_id: i64) -> AppResult<Vec<Organization>> {
        let orgs = sqlx::query_as("SELECT * FROM organizations WHERE tenant_id = $1 ORDER BY level, org_code")
            .bind(tenant_id)
            .fetch_all(&self.db)
            .await?;

        Ok(orgs)
    }

    pub async fn get_children(&self, tenant_id: i64, parent_id: i64) -> AppResult<Vec<i64>> {
        let children = sqlx::query(
            r#"
            WITH RECURSIVE org_tree AS (
                SELECT id FROM organizations WHERE parent_id = $2 AND tenant_id = $1
                UNION ALL
                SELECT o.id FROM organizations o
                INNER JOIN org_tree ot ON o.parent_id = ot.id
                WHERE o.tenant_id = $1
            )
            SELECT id FROM org_tree
            "#
        )
        .bind(tenant_id)
        .bind(parent_id)
        .fetch_all(&self.db)
        .await?
        .into_iter()
        .map(|r| r.get("id"))
        .collect();

        Ok(children)
    }

    pub async fn get_tree(&self, tenant_id: i64) -> AppResult<Vec<OrganizationTree>> {
        let orgs = self.get_by_tenant(tenant_id).await?;
        Ok(self.build_tree(orgs, 0))
    }

    fn build_tree(&self, orgs: Vec<Organization>, parent_id: i64) -> Vec<OrganizationTree> {
        let mut result = Vec::new();

        for org in orgs.iter().filter(|o| o.parent_id == parent_id) {
            let children = self.build_tree(
                orgs.iter()
                    .filter(|o| o.parent_id == org.id)
                    .cloned()
                    .collect(),
                org.id,
            );

            result.push(OrganizationTree {
                id: org.id,
                tenant_id: org.tenant_id,
                parent_id: org.parent_id,
                org_code: org.org_code.clone(),
                org_name: org.org_name.clone(),
                org_type: org.org_type.clone(),
                level: org.level,
                children,
            });
        }

        result
    }

    pub async fn get_org_users(&self, org_id: i64) -> AppResult<Vec<i64>> {
        let user_ids = sqlx::query("SELECT user_id FROM user_organizations WHERE organization_id = $1")
            .bind(org_id)
            .fetch_all(&self.db)
            .await?
            .into_iter()
            .map(|r| r.get("user_id"))
            .collect();

        Ok(user_ids)
    }

    pub async fn search_by_path(&self, tenant_id: i64, path_pattern: &str) -> AppResult<Vec<Organization>> {
        let pattern = format!("%{}%", path_pattern);
        let orgs = sqlx::query_as("SELECT * FROM organizations WHERE tenant_id = $1 AND path LIKE $2")
            .bind(tenant_id)
            .bind(&pattern)
            .fetch_all(&self.db)
            .await?;

        Ok(orgs)
    }
}
