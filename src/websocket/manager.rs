use axum::extract::ws::Message;
use std::collections::HashMap;
use tokio::sync::mpsc;

pub struct WebSocketManager {
    // tenant_id -> user_id -> sender
    connections: HashMap<i64, HashMap<i64, mpsc::UnboundedSender<Message>>>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    pub async fn add_connection(
        &mut self,
        tenant_id: i64,
        user_id: i64,
        tx: mpsc::UnboundedSender<Message>,
    ) {
        self.connections
            .entry(tenant_id)
            .or_insert_with(HashMap::new)
            .insert(user_id, tx);

        tracing::info!("WebSocket connected: tenant={}, user={}", tenant_id, user_id);
    }

    pub async fn remove_connection(&mut self, tenant_id: i64, user_id: i64) {
        if let Some(tenant_conns) = self.connections.get_mut(&tenant_id) {
            tenant_conns.remove(&user_id);

            if tenant_conns.is_empty() {
                self.connections.remove(&tenant_id);
            }
        }

        tracing::info!("WebSocket disconnected: tenant={}, user={}", tenant_id, user_id);
    }

    pub async fn send_to_user(
        &self,
        tenant_id: i64,
        user_id: i64,
        payload: serde_json::Value,
    ) {
        if let Some(tenant_conns) = self.connections.get(&tenant_id) {
            if let Some(tx) = tenant_conns.get(&user_id) {
                let msg = Message::Text(serde_json::to_string(&payload).unwrap());
                let _ = tx.send(msg);
            }
        }
    }

    pub async fn broadcast_to_tenant(
        &self,
        tenant_id: i64,
        payload: serde_json::Value,
    ) {
        if let Some(tenant_conns) = self.connections.get(&tenant_id) {
            let msg = Message::Text(serde_json::to_string(&payload).unwrap());

            for tx in tenant_conns.values() {
                let _ = tx.send(msg.clone());
            }
        }
    }

    pub async fn is_connected(&self, tenant_id: i64, user_id: i64) -> bool {
        if let Some(tenant_conns) = self.connections.get(&tenant_id) {
            tenant_conns.contains_key(&user_id)
        } else {
            false
        }
    }

    pub async fn get_online_count(&self, tenant_id: i64) -> usize {
        if let Some(tenant_conns) = self.connections.get(&tenant_id) {
            tenant_conns.len()
        } else {
            0
        }
    }
}
