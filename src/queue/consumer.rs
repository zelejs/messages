use crate::{
    config::Config,
    services::message_service::MessageService,
    websocket::WebSocketManager,
};
use lapin::{
    options::*,
    types::{FieldTable, AMQPValue},
    BasicProperties, Channel, Connection, ConnectionProperties,
};
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::RwLock;

const MAX_RETRY_COUNT: u32 = 3;
const RETRY_DELAY_MS: u64 = 5000; // 5 seconds initial delay, exponential backoff

/// 消息队列消费者 - 支持重试和死信队列
pub async fn start_consumer(
    config: Config,
    db: sqlx::PgPool,
    redis: redis::aio::ConnectionManager,
    ws_manager: Arc<RwLock<WebSocketManager>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to RabbitMQ
    let conn = Connection::connect(&config.rabbitmq_url, ConnectionProperties::default()).await?;

    let channel = conn.create_channel().await?;

    // 声明死信交换机
    let dlx_exchange = "message.dlx";
    channel
        .exchange_declare(
            dlx_exchange,
            lapin::ExchangeKind::Direct,
            ExchangeDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    // 声明死信队列
    let dlq_name = "message_queue_dlq";
    channel
        .queue_declare(
            dlq_name,
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    // 绑定死信队列到死信交换机
    channel
        .queue_bind(
            dlq_name,
            dlx_exchange,
            "message.failed",
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    // 声明主队列，配置死信交换机
    let mut queue_args = FieldTable::default();
    queue_args.insert(
        "x-dead-letter-exchange".into(),
        AMQPValue::LongString(dlx_exchange.into()),
    );
    queue_args.insert(
        "x-dead-letter-routing-key".into(),
        AMQPValue::LongString("message.failed".into()),
    );
    // 设置消息 TTL 为 1 小时 (可选)
    queue_args.insert(
        "x-message-ttl".into(),
        AMQPValue::LongInt(3600000),
    );

    let queue = channel
        .queue_declare(
            "message_queue",
            QueueDeclareOptions::default(),
            queue_args,
        )
        .await?;

    // 声明重试队列 (用于延迟重试)
    let mut retry_queue_args = FieldTable::default();
    retry_queue_args.insert(
        "x-dead-letter-exchange".into(),
        AMQPValue::LongString("".into()), // 默认交换机
    );
    retry_queue_args.insert(
        "x-dead-letter-routing-key".into(),
        AMQPValue::LongString("message_queue".into()),
    );
    retry_queue_args.insert(
        "x-message-ttl".into(),
        AMQPValue::LongInt(RETRY_DELAY_MS as i32),
    );

    channel
        .queue_declare(
            "message_queue_retry",
            QueueDeclareOptions::default(),
            retry_queue_args,
        )
        .await?;

    tracing::info!("Message queue consumer started: {}", queue.name());
    tracing::info!("Dead letter queue: {}", dlq_name);
    tracing::info!("Retry queue: message_queue_retry");

    // Create service instance
    let channel_config = config.channel_config.clone();
    let message_service = MessageService::new(db.clone(), redis.clone(), ws_manager.clone(), channel_config);

    process_messages(channel, message_service).await?;

    Ok(())
}

async fn process_messages(
    channel: Channel,
    message_service: MessageService,
) -> Result<(), Box<dyn std::error::Error>> {
    let channel = Arc::new(channel);

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
        let channel = channel.clone();
        let message_service = message_service.clone();

        // Spawn task to handle message
        tokio::spawn(async move {
            let data = String::from_utf8_lossy(&delivery.data);

            if let Ok(message_id) = data.parse::<i64>() {
                tracing::info!("Processing message: {}", message_id);

                // 获取重试次数
                let retry_count = get_retry_count(&delivery.properties);

                match message_service.process_message(message_id).await {
                    Ok(_) => {
                        // ACK - 处理成功
                        if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
                            tracing::error!("Failed to ACK message {}: {:?}", message_id, e);
                        } else {
                            tracing::info!("Message {} processed successfully", message_id);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to process message {}: {:?}", message_id, e);

                        if retry_count >= MAX_RETRY_COUNT {
                            // 超过最大重试次数，拒绝消息并发送到死信队列
                            tracing::warn!(
                                "Message {} exceeded max retry count ({}), sending to DLQ",
                                message_id,
                                MAX_RETRY_COUNT
                            );

                            if let Err(e) = delivery
                                .reject(BasicRejectOptions { requeue: false })
                                .await
                            {
                                tracing::error!("Failed to reject message {}: {:?}", message_id, e);
                            }

                            // 记录到数据库（可选）
                            if let Err(db_err) = message_service
                                .log_message_failed(message_id, &format!("Exceeded max retries: {:?}", e))
                                .await
                            {
                                tracing::error!("Failed to log message failure: {:?}", db_err);
                            }
                        } else {
                            // NACK 并重新入队，或者发送到重试队列
                            let next_retry = retry_count + 1;
                            tracing::info!(
                                "Retrying message {} (attempt {}/{})",
                                message_id,
                                next_retry,
                                MAX_RETRY_COUNT
                            );

                            // ACK 当前消息
                            if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
                                tracing::error!("Failed to ACK message {}: {:?}", message_id, e);
                                return;
                            }

                            // 发送到重试队列
                            if let Err(e) = publish_to_retry_queue(&channel, message_id, next_retry).await {
                                tracing::error!("Failed to publish to retry queue: {:?}", e);
                            }
                        }
                    }
                }
            } else {
                tracing::error!("Invalid message format: {}", data);
                // ACK 无效消息，避免重复处理
                let _ = delivery.ack(BasicAckOptions::default()).await;
            }
        });
    }

    Ok(())
}

/// 从消息属性中获取重试次数
fn get_retry_count(properties: &BasicProperties) -> u32 {
    properties
        .headers()
        .as_ref()
        .and_then(|headers: &FieldTable| {
            // FieldTable is a wrapper around BTreeMap<ShortString, AMQPValue>
            headers.inner().get("x-retry-count")
        })
        .and_then(|value| match value {
            AMQPValue::LongInt(v) => Some(*v as u32),
            AMQPValue::LongLongInt(v) => Some(*v as u32),
            AMQPValue::ShortInt(v) => Some(*v as u32),
            _ => None,
        })
        .unwrap_or(0)
}

/// 发送消息到重试队列
async fn publish_to_retry_queue(
    channel: &Channel,
    message_id: i64,
    retry_count: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let payload = message_id.to_string();

    // 设置消息头，记录重试次数
    let mut headers = FieldTable::default();
    headers.insert(
        "x-retry-count".into(),
        AMQPValue::LongInt(retry_count as i32),
    );

    let properties = BasicProperties::default().with_headers(headers);

    channel
        .basic_publish(
            "",
            "message_queue_retry",
            BasicPublishOptions::default(),
            payload.as_bytes(),
            properties,
        )
        .await?;

    tracing::debug!(
        "Published message {} to retry queue (attempt {})",
        message_id,
        retry_count
    );
    Ok(())
}
