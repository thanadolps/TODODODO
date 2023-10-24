use color_eyre::eyre::Context;
use gengrpc::performance::{Performance, PerformanceServer, StreakDetail};
use poem::{listener::TcpListener, Server};
use poem_grpc::{Response, RouteGrpc, Status};
use sqlx::PgPool;
use uuid::Uuid;

struct PerformanceService {
    pool: PgPool,
}

#[poem::async_trait]
impl Performance for PerformanceService {
    async fn add_streak(
        &self,
        request: poem_grpc::Request<StreakDetail>,
    ) -> Result<Response<()>, Status> {
        let user_id = &request.user_id;

        let uuid = Uuid::parse_str(user_id).unwrap();

        sqlx::query!("
        INSERT INTO performance (user_id, combo, best_record)
            VALUES ($1, 1, 1)
            ON CONFLICT (user_id)
            DO UPDATE SET combo = performance.combo + 1, best_record = GREATEST(performance.combo + 1, performance.best_record);
        ", uuid).execute(&self.pool).await.unwrap();

        Ok(Response::new(()))
    }

    async fn reset_streak(
        &self,
        request: poem_grpc::Request<StreakDetail>,
    ) -> Result<Response<()>, Status> {
        let user_id = &request.user_id;

        let uuid = Uuid::parse_str(user_id).unwrap();

        sqlx::query!("UPDATE performance SET combo = 0 WHERE user_id=$1", uuid)
            .execute(&self.pool)
            .await
            .unwrap();

        Ok(Response::new(()))
    }
}

#[derive(serde::Deserialize, Debug)]
struct Env {
    port: u16,
    database_url: String,
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

    let service = PerformanceService { pool };
    Server::new(TcpListener::bind(format!("127.0.0.1:{}", env.port)))
        .run(RouteGrpc::new().add_service(PerformanceServer::new(service)))
        .await?;

    Ok(())
}
