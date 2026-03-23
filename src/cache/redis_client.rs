use crate::error::AppResult;
use redis::AsyncCommands;
use std::collections::HashMap;

#[allow(dead_code)]
pub struct RedisCache {
    conn: redis::aio::ConnectionManager,
}

#[allow(dead_code)]
impl RedisCache {
    pub fn new(conn: redis::aio::ConnectionManager) -> Self {
        Self { conn }
    }

    // Online status management
    pub async fn set_user_online(&mut self, tenant_id: i64, user_id: i64) -> AppResult<()> {
        let key = format!("online:tenant:{}", tenant_id);
        self.conn.sadd::<_, _, ()>(key, user_id).await?;
        Ok(())
    }

    pub async fn set_user_offline(&mut self, tenant_id: i64, user_id: i64) -> AppResult<()> {
        let key = format!("online:tenant:{}", tenant_id);
        self.conn.srem::<_, _, ()>(key, user_id).await?;
        Ok(())
    }

    pub async fn is_user_online(&mut self, tenant_id: i64, user_id: i64) -> AppResult<bool> {
        let key = format!("online:tenant:{}", tenant_id);
        let result: bool = self.conn.sismember(key, user_id).await?;
        Ok(result)
    }

    // Unread count management
    pub async fn increment_unread(&mut self, user_id: i64, category: &str) -> AppResult<()> {
        let key = format!("unread:{}", user_id);
        self.conn.hincr::<_, _, _, ()>(key, category, 1).await?;
        Ok(())
    }

    pub async fn decrement_unread(&mut self, user_id: i64, category: &str) -> AppResult<()> {
        let key = format!("unread:{}", user_id);
        self.conn.hincr::<_, _, _, ()>(key, category, -1).await?;
        Ok(())
    }

    pub async fn get_unread_stats(&mut self, user_id: i64) -> AppResult<serde_json::Value> {
        let key = format!("unread:{}", user_id);
        let result: HashMap<String, i64> = self.conn.hgetall(key).await?;

        let total: i64 = result.values().cloned().sum();

        Ok(serde_json::json!({
            "total": total,
            "by_category": result,
        }))
    }

    pub async fn reset_unread(&mut self, user_id: i64) -> AppResult<()> {
        let key = format!("unread:{}", user_id);
        self.conn.del::<_, ()>(key).await?;
        Ok(())
    }

    pub async fn reset_unread_by_category(&mut self, user_id: i64, category: &str) -> AppResult<()> {
        let key = format!("unread:{}", user_id);
        self.conn.hdel::<_, _, ()>(key, category).await?;
        Ok(())
    }

    // Generic cache operations
    pub async fn get<T: serde::de::DeserializeOwned>(&mut self, key: &str) -> AppResult<Option<T>> {
        let val: Option<String> = self.conn.get(key).await?;
        match val {
            Some(v) => {
                let result = serde_json::from_str(&v)?;
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }

    pub async fn set<T: serde::Serialize>(&mut self, key: &str, value: &T, ttl_seconds: Option<usize>) -> AppResult<()> {
        let val = serde_json::to_string(value)?;
        match ttl_seconds {
            Some(ttl) => {
                redis::cmd("SETEX")
                    .arg(key)
                    .arg(ttl as u64)
                    .arg(val)
                    .query_async::<_, ()>(&mut self.conn)
                    .await?;
            }
            None => {
                redis::cmd("SET")
                    .arg(key)
                    .arg(val)
                    .query_async::<_, ()>(&mut self.conn)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn delete(&mut self, key: &str) -> AppResult<()> {
        self.conn.del::<_, ()>(key).await?;
        Ok(())
    }
}
