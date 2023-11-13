mod handler;

use color_eyre::{eyre::Context, Result};
use futures_lite::stream::StreamExt;
use lapin::{options::*, types::FieldTable, Channel, Connection, ConnectionProperties};
use poem::{listener::TcpListener, middleware, EndpointExt, Route, Server};
use poem_openapi::OpenApiService;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::info;

use time::{macros::offset, OffsetDateTime};

use tracing_subscriber::{fmt, prelude::*, EnvFilter};
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
async fn send_notification(
    notification: &NotificationDetail,
    webhook_url: Option<&str>,
) -> Result<()> {
    let deadline = notification.deadline.to_offset(offset!(+7));
    let within = notification.deadline - OffsetDateTime::now_utc();

    let msg = format!(
        "This is a notification for your task **{}** ({}). Description: {}. Deadline: {} (within {})",
        notification.title, notification.task_id, notification.description, deadline, within
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
    port: u16,
    database_url: String,
    amqp_addr: String,
    #[serde(alias = "railway_public_domain")]
    public_domain: Option<String>,
    log_mongo_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Envars
    dotenvy::dotenv().ok();
    let env = envy::from_env::<Env>().context("Failed to parse environment variables")?;

    // Setup tracing/logging
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug,info");
    }
    tracing_subscriber::registry()
        .with(fmt::layer().with_filter(EnvFilter::from_default_env()))
        .with(if let Some(uri) = env.log_mongo_url.as_ref() {
            Some(
                tracing_mongo::MongoLogger::new(uri, "log", "notification_service")
                    .await?
                    .layer(),
            )
        } else {
            tracing::warn!("No log_mongo_url envar set, not logging to MongoDB");
            None
        })
        .init();

    // Setup database
    let pool = PgPool::connect(&env.database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    // RabbitMQ
    let conn = Connection::connect(&env.amqp_addr, ConnectionProperties::default())
        .await
        .context("Fail to connect to RabbitMQ")?;
    let channel = conn.create_channel().await?; // Create a channel.
    tokio::task::spawn(listen_notification(channel, pool.clone()));

    // OpenAPI
    let server_url = if let Some(domain) = env.public_domain {
        if domain.contains("://") {
            domain
        } else {
            format!("https://{}:{}", domain, env.port)
        }
    } else {
        format!("http://localhost:{}", env.port)
    };
    let handler = handler::Api { pool };
    let api_service =
        OpenApiService::new(handler, "TODODODO - Notification Service", "1.0").server(server_url);
    let ui = api_service.openapi_explorer();
    let spec = api_service.spec_endpoint();

    // Route
    let route = Route::new()
        .nest("/", api_service)
        .nest("/docs", ui)
        .nest("/docs-json", spec)
        .with(middleware::Cors::default())
        .with(middleware::CatchPanic::default())
        .with(middleware::Tracing);

    // Start server
    let ip = format!("0.0.0.0:{}", env.port);
    Server::new(TcpListener::bind(ip))
        .run(route)
        .await
        .with_context(|| format!("Fail to start server on port {:?}", env.port))?;
    Ok(())
}

async fn listen_notification(channel: Channel, pool: PgPool) {
    info!("Notification Service CONNECTED!");

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
                let notification = match serde_json::from_str::<NotificationDetail>(&msg) {
                    Ok(notification) => notification,
                    Err(err) => {
                        tracing::error!("Error in deserialization of message {}: {:?}", msg, err);
                        continue;
                    }
                };

                // Query associated webhook URL
                let webhook_url: Option<Option<String>> = sqlx::query_scalar!(
                    "SELECT url FROM webhook WHERE user_id = $1",
                    notification.user_id
                )
                .fetch_optional(&pool)
                .await
                .unwrap();

                let webhook_url = if let Some(webhook_url) = Option::flatten(webhook_url) {
                    webhook_url
                } else {
                    tracing::warn!(
                        "No webhook URL found for user {}. Notification not sent.",
                        notification.user_id
                    );
                    continue;
                };

                send_notification(&notification, Some(&webhook_url))
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
}
