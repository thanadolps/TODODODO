use color_eyre::{
    eyre::{Context, ContextCompat},
    Result,
};
use futures_lite::stream::StreamExt;
use lapin::{options::*, types::FieldTable, Channel};

use serde::Deserialize;
use sqlx::PgPool;
use tracing::{error, info};

use time::{macros::offset, OffsetDateTime};

use uuid::Uuid;
use webhook::client::WebhookClient;

#[derive(Deserialize, Debug)]
struct NotificationDetail {
    user_id: Uuid,
    task_id: String,
    title: String,
    description: String,
    deadline: OffsetDateTime,
}

/// Send notification to external service
#[tracing::instrument]
async fn send_notification(notification: &NotificationDetail, webhook_url: Option<&str>) {
    let deadline = notification.deadline.to_offset(offset!(+7));
    let within = notification.deadline - OffsetDateTime::now_utc();

    let msg = format!(
        "This is a notification for your task **{}** ({}). Description: {}. Deadline: {} (within {})",
        notification.title, notification.task_id, notification.description, deadline, within
    );

    tracing::info!(msg, "Sending notification");

    let Some(webhook_url) = webhook_url else {
        tracing::warn!("No webhook URL provided. Notification not sent.");
        return;
    };

    let client: WebhookClient = WebhookClient::new(webhook_url);
    if let Err(err) = client
        .send(|message| {
            message.content(&msg).embed(|em| {
                em.title(&notification.title)
                    .description(&notification.description)
            })
        })
        .await
    {
        error!(error = err, "Error while sending notification");
    }
}

#[tracing::instrument(skip(channel, pool))]
pub async fn listen_notification(channel: Channel, pool: PgPool) -> color_eyre::Result<()> {
    info!("Notification Service CONNECTED!");

    // Declare the same queue for receiving tasks.
    let task_queue = channel
        .queue_declare(
            "task_queue",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;
    info!(?task_queue, "Declared task queue");

    // Set up a consumer to receive and process tasks.
    let mut consumer = channel
        .basic_consume(
            "task_queue",
            "my_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    // Process tasks.
    info!("Notification Service is ready to receive tasks.");
    while let Some(delivery_result) = consumer.next().await {
        match delivery_result {
            Ok(delivery) => {
                // Handle the delivery.
                if let Err(err) = handle_delivery(&delivery, &pool)
                    .await
                    .context("Error while handling delivery")
                {
                    error!(error = ?err);
                }

                // Acknowledge the task to remove it from the queue.
                channel
                    .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                    .await
                    .expect("Failed to acknowledge the task");
            }
            Err(err) => info!("Error in consumer: {:?}", err),
        }
    }

    Ok(())
}

async fn handle_delivery(delivery: &lapin::message::Delivery, pool: &PgPool) -> Result<()> {
    let msg = String::from_utf8_lossy(&delivery.data);
    info!(msg=?msg, "Received msg");

    // Send notification to external service
    let notification = serde_json::from_str::<NotificationDetail>(&msg)
        .context("Error in deserialization of message")?;

    // Query associated webhook URL
    let webhook_url: Option<Option<String>> = sqlx::query_scalar!(
        "SELECT url FROM webhook WHERE user_id = $1",
        notification.user_id
    )
    .fetch_optional(pool)
    .await?;

    let webhook_url = Option::flatten(webhook_url).context(format!(
        "No webhook URL found for user {}. Notification not sent.",
        notification.user_id
    ))?;

    // Send notification
    send_notification(&notification, Some(&webhook_url)).await;

    Ok(())
}
