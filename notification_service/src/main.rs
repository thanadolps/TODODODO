use std::time::SystemTime;

use color_eyre::Result;
use futures_lite::stream::StreamExt;
use lapin::{
    options::*, publisher_confirm::Confirmation, types::FieldTable, BasicProperties, Connection,
    ConnectionProperties,
};
use serde::Deserialize;
use tracing::info;

// use gengrpc::notification::{Notifier, NotifierServer};
use webhook::client::WebhookClient;

struct NotificationService;

#[derive(Deserialize, Debug)]
struct NotificationDetail {
    task_id: String,
    title: String,
    description: String,
    deadline: Option<SystemTime>,
}

async fn send_notification(notification: &NotificationDetail) -> Result<()> {
    let url: &str = "https://discord.com/api/webhooks/1166039146989629563/Rylu9HS5c34vNSDMVY9LyhukJLtvV09-3MlN_QmsrGKQ-KFbIQd6E_aFZDqMSdlAqOgC";
    let msg = format!(
        "This is a notification for your task **{}** ({}). Description: {}. Deadline: {:?}",
        notification.title, notification.task_id, notification.description, notification.deadline
    );

    let client: WebhookClient = WebhookClient::new(url);
    client.send(|message| message.content(&msg)).await.unwrap();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::fmt::init();

    let addr = std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://127.0.0.1:5672/%2f".into());

    let conn = Connection::connect(&addr, ConnectionProperties::default()).await?;

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

    tokio::spawn(async move {
        info!("Notification Service is ready to receive tasks.");
        while let Some(delivery_result) = consumer.next().await {
            match delivery_result {
                Ok(delivery) => {
                    let task = String::from_utf8_lossy(&delivery.data);
                    info!("Received task: {}", task);

                    let notification: NotificationDetail = serde_json::from_str(&task).unwrap();
                    send_notification(&notification).await.unwrap();

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
    })
    .await
    .unwrap();

    Ok(())
}
