mod handlers;
mod jwt;
mod models;

use color_eyre::eyre::Context;
use jsonwebtoken::{DecodingKey, EncodingKey};
use poem::{listener::TcpListener, middleware, EndpointExt, Route, Server};
use poem_openapi::OpenApiService;
use serde::Deserialize;
use sqlx::postgres::PgPool;

#[derive(Deserialize, Debug)]
struct Env {
    port: u16,
    database_url: String,
    jwt_secret: String,
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

    // JWT
    let encode_key = EncodingKey::from_base64_secret(env.jwt_secret.as_str())?;
    let decode_key = DecodingKey::from_base64_secret(env.jwt_secret.as_str())?;

    // Handler
    let handler = (
        handlers::account::Api {
            pool: pool.clone(),
            encode_key,
        },
        handlers::community::Api { pool: pool.clone() },
        handlers::invite_code::Api { pool },
    );

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
        .with(middleware::CatchPanic::default())
        .data(decode_key);

    // Start server
    let ip = format!("0.0.0.0:{}", env.port);
    Server::new(TcpListener::bind(ip)).run(route).await?;
    Ok(())
}
