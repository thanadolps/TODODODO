use color_eyre::{eyre::Context, Result};
use futures_lite::stream::StreamExt;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties};
use serde::Deserialize;
use tracing::info;

use time::OffsetDateTime;

use webhook::client::WebhookClient;

#[derive(Deserialize, Debug)]
struct NotificationDetail {
    task_id: String,
    title: String,
    description: String,
    deadline: OffsetDateTime,
}

/// Send notification to external service
async fn send_notification(
    notification: &NotificationDetail,
    webhook_url: Option<&str>,
) -> Result<()> {
    let msg = format!(
        "This is a notification for your task **{}** ({}). Description: {}. Deadline: {} (within {})",
        notification.title, notification.task_id, notification.description, notification.deadline, (notification.deadline - OffsetDateTime::now_utc())
    );

    tracing::info!(msg, "Sending notification");

    if let Some(webhook_url) = webhook_url {
        let client: WebhookClient = WebhookClient::new(webhook_url);
        client
            .send(|message| {
                message.content(&msg).embed(|em| {
                    em.title(&notification.title)
                        .description(&notification.description)
                })
            })
            .await
            .unwrap();
    } else {
        tracing::warn!("No webhook URL provided. Notification not sent.");
    }

    Ok(())
}

#[derive(Deserialize, Debug)]
struct Env {
    amqp_addr: String,
    webhook_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Envars
    dotenvy::dotenv().ok();
    let env = envy::from_env::<Env>().context("Failed to parse environment variables")?;

    // Setup tracing/logging
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt::init();

    // RabbitMQ
    let conn = Connection::connect(&env.amqp_addr, ConnectionProperties::default()).await?;

    info!("Notification Service CONNECTED!");

    // Create a channel.
    let channel = conn.create_channel().await.unwrap();

    // Declare the same queue for receiving tasks.
    let task_queue = channel
        .queue_declare(
            "task_queue",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();
    info!(?task_queue, "Declared task queue");

    // Set up a consumer to receive and process tasks.
    let mut consumer = channel
        .basic_consume(
            "task_queue",
            "my_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

    // Process tasks.
    info!("Notification Service is ready to receive tasks.");
    while let Some(delivery_result) = consumer.next().await {
        match delivery_result {
            Ok(delivery) => {
                let msg = String::from_utf8_lossy(&delivery.data);
                info!("Received msg: {}", msg);

                // Send notification to external service
                let notification = match serde_json::from_str(&msg) {
                    Ok(notification) => notification,
                    Err(err) => {
                        tracing::error!("Error in deserialization of message {}: {:?}", msg, err);
                        continue;
                    }
                };

                send_notification(&notification, env.webhook_url.as_deref())
                    .await
                    .unwrap();

                // Acknowledge the task to remove it from the queue.
                channel
                    .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                    .await
                    .expect("Failed to acknowledge the task");
            }
            Err(err) => {
                info!("Error in consumer: {:?}", err);
            }
        }
    }

    Ok(())
}
