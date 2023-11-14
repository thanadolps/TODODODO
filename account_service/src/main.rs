mod handlers;
mod jwt;
mod models;

use gengrpc::community_task::CommunityTaskServiceClient;

use color_eyre::eyre::Context;
use jsonwebtoken::{DecodingKey, EncodingKey};
use poem::{listener::TcpListener, middleware, EndpointExt, Route, Server};
use poem_grpc::ClientConfig;
use poem_openapi::OpenApiService;
use serde::Deserialize;
use sqlx::postgres::PgPool;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Deserialize, Debug)]
struct Env {
    port: u16,
    database_url: String,
    jwt_secret: String,
    task_grpc_url: String,
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
    tracing_init(env.log_mongo_url).await?;

    // Setup database
    let pool = PgPool::connect(&env.database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    // JWT
    let encode_key = EncodingKey::from_base64_secret(env.jwt_secret.as_str())?;
    let decode_key = DecodingKey::from_base64_secret(env.jwt_secret.as_str())?;

    // GRPC
    let task_grpc =
        CommunityTaskServiceClient::new(ClientConfig::builder().uri(env.task_grpc_url).build()?);

    // Handler
    let handler = (
        handlers::account::Api {
            pool: pool.clone(),
            encode_key,
        },
        handlers::community::Api { pool: pool.clone() },
        handlers::invite_code::Api { pool: pool.clone() },
        handlers::community_task::Api {
            pool: pool.clone(),
            task_grpc,
        },
    );

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
        OpenApiService::new(handler, "TODODODO - Account and Community Service", "1.0")
            .server(server_url);
    let ui = api_service.openapi_explorer();
    let spec = api_service.spec_endpoint();

    // Route
    let route = Route::new()
        .nest("/", api_service)
        .nest("/docs", ui)
        .nest("/docs-json", spec)
        .with(middleware::Cors::default())
        .with(middleware::CatchPanic::default())
        .with(middleware::Tracing)
        .data(decode_key);

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
