use crate::{
    config::Config,
    services::message_service::MessageService,
    websocket::WebSocketManager,
};
use lapin::{
    options::*,
    types::FieldTable,
    Channel, Connection, ConnectionProperties,
};
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn start_consumer(
    config: Config,
    db: sqlx::PgPool,
    redis: redis::aio::ConnectionManager,
    _ws_manager: Arc<RwLock<WebSocketManager>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to RabbitMQ
    let conn = Connection::connect(&config.rabbitmq_url, ConnectionProperties::default()).await?;

    let channel = conn.create_channel().await?;

    // Declare queue
    let queue = channel
        .queue_declare(
            "message_queue",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    tracing::info!("Message queue consumer started: {}", queue.name());

    // Create service instance
    let message_service = MessageService::new(db, redis);

    process_messages(channel, message_service).await?;

    Ok(())
}

async fn process_messages(
    channel: Channel,
    message_service: MessageService,
) -> Result<(), Box<dyn std::error::Error>> {
    // Start consuming
    let mut consumer = channel
        .basic_consume(
            "message_queue",
            "message_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    while let Some(delivery_result) = consumer.next().await {
        let delivery = delivery_result?;

        let data = String::from_utf8_lossy(&delivery.data);

        if let Ok(message_id) = data.parse::<i64>() {
            tracing::info!("Processing message: {}", message_id);

            match message_service.process_message(message_id).await {
                Ok(_) => {
                    // ACK
                    delivery.ack(BasicAckOptions::default()).await?;
                }
                Err(e) => {
                    tracing::error!("Failed to process message: {:?}", e);
                    // NACK with requeue=false to move to dead letter queue or just discard
                    delivery.nack(BasicNackOptions::default()).await?;
                }
            }
        }
    }

    Ok(())
}
