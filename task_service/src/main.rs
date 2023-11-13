mod dtos;
mod handlers;
mod models;
mod noti;
mod routine;

use std::time::Duration;

use color_eyre::eyre::Context;
use poem::{listener::TcpListener, middleware, EndpointExt, Route, Server};
use poem_grpc::ClientConfig;
use poem_openapi::OpenApiService;
use sqlx::postgres::PgPool;

use tracing::info;

use gengrpc::performance::PerformanceClient;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(serde::Deserialize, Debug)]
struct Env {
    port: u16,
    database_url: String,
    amqp_addr: String,
    performance_url: String,
    #[serde(alias = "railway_public_domain")]
    public_domain: Option<String>,
    log_mongo_url: Option<String>,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
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
                tracing_mongo::MongoLogger::new(&uri, "log", "task_service")
                    .await?
                    .layer(),
            )
        } else {
            tracing::warn!("No log_mongo_url envar set, not logging to MongoDB");
            None
        })
        .init();

    tracing::info!(?env, "Environment Variable");

    // Setup database
    let pool = PgPool::connect(&env.database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    // gRPC clients
    let performance =
        PerformanceClient::new(ClientConfig::builder().uri(env.performance_url).build()?);

    // Handler
    let handler = handlers::Api {
        pool: pool.clone(),
        performance,
    };

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
    let api_service =
        OpenApiService::new(handler, "TODODODO - Task Service", "1.0").server(server_url);
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

    // Watch for deadline
    tokio::spawn(async move {
        let conn = Connection::connect(&env.amqp_addr, ConnectionProperties::default())
            .await
            .unwrap();

        info!("Task Service CONNECTED!");

        let channel = conn.create_channel().await.unwrap();

        //send channel into noti.rs

        // Declare a queue for sending tasks.
        let task_queue = channel
            .queue_declare(
                "task_queue",
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap();

        info!(?task_queue, "Declared task queue");

        tokio::spawn(noti::watch_notification_task(
            pool.clone(),
            Duration::from_secs(10),
            Duration::from_secs(30 * 60),
            channel,
        ));

        tokio::spawn(routine::refresh_routine_task(
            pool.clone(),
            Duration::from_secs(60),
        ));
    });

    // Start server
    let ip = format!("0.0.0.0:{}", env.port);
    Server::new(TcpListener::bind(ip))
        .run(route)
        .await
        .with_context(|| format!("Fail to start server on port {:?}", env.port))?;
    Ok(())
}
