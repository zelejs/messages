use crate::{
    config::Config,
    error::{AppError, AppResult},
};
use lapin::{
    options::*, types::FieldTable, BasicProperties, Channel, Connection, ConnectionProperties,
};
use std::sync::Arc;

pub struct MessageProducer {
    channel: Arc<Channel>,
}

impl MessageProducer {
    pub async fn new(config: &Config) -> AppResult<Self> {
        let conn = Connection::connect(&config.rabbitmq_url, ConnectionProperties::default())
            .await
            .map_err(|e| AppError::Internal(format!("Failed to connect to RabbitMQ: {}", e)))?;

        let channel = conn
            .create_channel()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to create channel: {}", e)))?;

        // Declare queue
        channel
            .queue_declare(
                "message_queue",
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to declare queue: {}", e)))?;

        tracing::info!("Message queue producer initialized");

        Ok(Self {
            channel: Arc::new(channel),
        })
    }

    pub async fn publish_message(&self, message_id: i64) -> AppResult<()> {
        let payload = message_id.to_string();
        let exchange = "";
        let routing_key = "message_queue";

        self.channel
            .basic_publish(
                exchange,
                routing_key,
                BasicPublishOptions::default(),
                payload.as_bytes(),
                BasicProperties::default(),
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to publish message: {}", e)))?;

        tracing::debug!("Published message {} to queue", message_id);
        Ok(())
    }

    pub async fn publish_message_with_delay(
        &self,
        message_id: i64,
        delay_seconds: u64,
    ) -> AppResult<()> {
        // For delayed messages, we could use:
        // 1. RabbitMQ delayed message plugin
        // 2. A separate delayed queue
        // 3. Store in DB and let scheduler pick it up

        if delay_seconds == 0 {
            return self.publish_message(message_id).await;
        }

        // For now, just publish immediately
        // The scheduled_at field in messages table will be used by scheduler
        self.publish_message(message_id).await
    }
}
