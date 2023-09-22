mod dtos;
mod handlers;
mod models;
mod noti;

use std::time::Duration;

use color_eyre::eyre::Context;
use poem::{listener::TcpListener, middleware, EndpointExt, Route, Server};
use poem_grpc::ClientConfig;
use poem_openapi::OpenApiService;
use sqlx::postgres::PgPool;

#[derive(serde::Deserialize, Debug)]
struct Env {
    port: String,
    database_url: String,
    notifer_url: String,
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
    tracing_subscriber::fmt::init();

    // Setup database
    let pool = PgPool::connect(&env.database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    // gRPC clients
    let notifier = noti::NotifierClient::new(ClientConfig::builder().uri(env.notifer_url).build()?);

    // Handler
    let handler = handlers::Api { pool: pool.clone() };

    // OpenAPI
    let api_service = OpenApiService::new(handler, "TODODODO - Task Service", "1.0")
        .server(format!("http://localhost:{}", env.port));
    let ui = api_service.openapi_explorer();
    let spec = api_service.spec_endpoint();

    // Route
    let route = Route::new()
        .nest("/", api_service)
        .nest("/docs", ui)
        .nest("/docs-json", spec)
        .with(middleware::Cors::default())
        .with(middleware::CatchPanic::default());

    // Watch for deadline
    tokio::spawn(noti::watch_notification_task(
        pool,
        notifier,
        Duration::from_secs(60),
        Duration::from_secs(30 * 60),
    ));

    // Start server
    let ip = format!("127.0.0.1:{}", env.port);
    Server::new(TcpListener::bind(ip)).run(route).await?;
    Ok(())
}
