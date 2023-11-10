use color_eyre::eyre::Context;
use gengrpc::performance::{
    HabitDetail, Performance, PerformanceServer, RoutineDetail, StreakDetail,
};
use poem::{listener::TcpListener, Server};
use poem_grpc::{Response, RouteGrpc, Status};
use sqlx::PgPool;
use time::OffsetDateTime;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

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

    async fn complete_routine(
        &self,
        request: poem_grpc::Request<RoutineDetail>,
    ) -> Result<Response<()>, Status> {
        let task_id = &request.task_id;
        let completed_at = request.completed_at.as_ref().unwrap();

        let converted_completed_at =
            OffsetDateTime::from_unix_timestamp(completed_at.seconds).unwrap();

        let uuid = Uuid::parse_str(task_id).unwrap();
        // Add to table RoutineCompletion
        sqlx::query!(
            "INSERT INTO routine_completion VALUES($1,$2)",
            uuid,
            converted_completed_at
        )
        .execute(&self.pool)
        .await
        .unwrap();

        Ok(Response::new(()))
    }

    async fn trigger_habit(
        &self,
        request: poem_grpc::Request<HabitDetail>,
    ) -> Result<Response<()>, Status> {
        let task_id = &request.task_id;
        let positive = request.positive;
        let triggered_at = request.triggered_at.as_ref().unwrap();

        let converted_triggered_at =
            OffsetDateTime::from_unix_timestamp(triggered_at.seconds).unwrap();
        let uuid = Uuid::parse_str(task_id).unwrap();

        sqlx::query!(
            "INSERT INTO habit_history VALUES($1,$2,$3)",
            uuid,
            positive,
            converted_triggered_at
        )
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
                tracing_mongo::MongoLogger::new(&uri, "log", "viz_service")
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

    let service = PerformanceService { pool };
    Server::new(TcpListener::bind(format!("0.0.0.0:{}", env.port)))
        .run(RouteGrpc::new().add_service(PerformanceServer::new(service)))
        .await
        .with_context(|| format!("Fail to start server on port {:?}", env.port))?;

    Ok(())
}
