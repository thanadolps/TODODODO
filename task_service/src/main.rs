mod dtos;
mod grpc;
mod handlers;
mod models;
mod noti;
mod routine;

use std::time::Duration;

use color_eyre::eyre::Context;
use poem::{listener::TcpListener, middleware, EndpointExt, Route, Server};
use poem_grpc::{ClientConfig, RouteGrpc};
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

    check_period: Option<u64>,
    lead_time: Option<u64>,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // Envars
    dotenvy::dotenv().ok();
    let env = envy::from_env::<Env>().context("Failed to parse environment variables")?;

    // Setup tracing/logging
    tracing_init(env.log_mongo_url).await?;

    // Setup database
    let pool = PgPool::connect(&env.database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    // gRPC clients
    let performance =
        PerformanceClient::new(ClientConfig::builder().uri(env.performance_url).build()?);

    // Handler
    let handler = handlers::handlers(&pool, &performance);

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
    let route_grpc = RouteGrpc::new().add_service(grpc::task_service_server(pool.clone()));
    let route = Route::new()
        .nest("/", api_service)
        .nest("/docs", ui)
        .nest("/docs-json", spec)
        .nest("/grpc", route_grpc)
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
            Duration::from_secs(env.check_period.unwrap_or(30)),
            Duration::from_secs(env.lead_time.unwrap_or(30 * 60)),
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

#[tracing::instrument]
async fn tracing_init(log_mongo_url: Option<String>) -> color_eyre::Result<()> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug,info");
    }

    let file_appender = tracing_appender::rolling::hourly("./log", "tracing.log");

    tracing_subscriber::registry()
        .with(fmt::layer().with_filter(EnvFilter::from_default_env()))
        .with(fmt::layer().json().with_writer(file_appender))
        .with(if let Some(uri) = log_mongo_url.as_ref() {
            Some(
                tracing_mongo::MongoLogger::new(uri, "log", "task_service")
                    .await?
                    .layer(),
            )
        } else {
            tracing::warn!("No log_mongo_url envar set, not logging to MongoDB");
            None
        })
        .init();

    Ok(())
}
