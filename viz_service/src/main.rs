mod dtos;
mod models;

use color_eyre::eyre::Context;
use gengrpc::performance::{
    HabitDetail, Performance, PerformanceServer, RoutineDetail, StreakDetail,
};
use poem::error::InternalServerError;
use poem::{listener::TcpListener, middleware, EndpointExt, Result, Route, Server};
use poem_grpc::{Code, Response, RouteGrpc, Status};
use poem_openapi::{
    param::{Path, Query},
    payload::Json,
    ApiResponse, OpenApi, OpenApiService,
};
use sqlx::PgPool;
use time::OffsetDateTime as DateTime;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use uuid::Uuid;
struct PerformanceGRPCService {
    pool: PgPool,
}

struct PerformanceRESTService {
    pool: PgPool,
}

#[poem::async_trait]
impl Performance for PerformanceGRPCService {
    async fn add_streak(
        &self,
        request: poem_grpc::Request<StreakDetail>,
    ) -> Result<Response<()>, Status> {
        let user_id = &request.user_id;
        let uuid = Uuid::parse_str(user_id)
            .map_err(|err| Status::new(Code::InvalidArgument).with_message(err))?;

        sqlx::query!("
        INSERT INTO performance (user_id, combo, best_record)
            VALUES ($1, 1, 1)
            ON CONFLICT (user_id)
            DO UPDATE SET combo = performance.combo + 1, best_record = GREATEST(performance.combo + 1, performance.best_record);
        ", uuid).execute(&self.pool).await.map_err(Status::from_std_error)?;

        Ok(Response::new(()))
    }

    async fn reset_streak(
        &self,
        request: poem_grpc::Request<StreakDetail>,
    ) -> Result<Response<()>, Status> {
        let user_id = &request.user_id;

        let uuid = Uuid::parse_str(user_id)
            .map_err(|err| Status::new(Code::InvalidArgument).with_message(err))?;

        sqlx::query!("UPDATE performance SET combo = 0 WHERE user_id=$1", uuid)
            .execute(&self.pool)
            .await
            .map_err(Status::from_std_error)?;

        Ok(Response::new(()))
    }

    async fn complete_routine(
        &self,
        request: poem_grpc::Request<RoutineDetail>,
    ) -> Result<Response<()>, Status> {
        let task_id = &request.task_id;
        let completed_at = request
            .completed_at
            .as_ref()
            .ok_or(Status::new(Code::InvalidArgument))?;

        let converted_completed_at =
            DateTime::from_unix_timestamp(completed_at.seconds).map_err(Status::from_std_error)?;

        let uuid = Uuid::parse_str(task_id)
            .map_err(|err| Status::new(Code::InvalidArgument).with_message(err))?;
        // Add to table RoutineCompletion
        sqlx::query!(
            "INSERT INTO routine_completion VALUES($1,$2)",
            uuid,
            converted_completed_at
        )
        .execute(&self.pool)
        .await
        .map_err(Status::from_std_error)?;

        Ok(Response::new(()))
    }

    async fn trigger_habit(
        &self,
        request: poem_grpc::Request<HabitDetail>,
    ) -> Result<Response<()>, Status> {
        let task_id = &request.task_id;
        let positive = request.positive;
        let triggered_at = request
            .triggered_at
            .as_ref()
            .ok_or(Status::new(Code::InvalidArgument))?;

        let converted_triggered_at =
            DateTime::from_unix_timestamp(triggered_at.seconds).map_err(Status::from_std_error)?;
        let uuid = Uuid::parse_str(task_id)
            .map_err(|err| Status::new(Code::InvalidArgument).with_message(err))?;

        sqlx::query!(
            "INSERT INTO habit_history VALUES($1,$2,$3)",
            uuid,
            positive,
            converted_triggered_at
        )
        .execute(&self.pool)
        .await
        .map_err(Status::from_std_error)?;

        Ok(Response::new(()))
    }
}

#[derive(ApiResponse)]
pub enum HelloResponse {
    #[oai(status = 200)]
    Ok(Json<String>),
    #[oai(status = 404)]
    /// Specified task not found.
    NotFound,
}

#[derive(ApiResponse)]
pub enum OptionalStreakResponse {
    #[oai(status = 200)]
    Ok(Json<dtos::Streak>),
    #[oai(status = 404)]
    /// Specified task not found.
    NotFound,
}

#[derive(ApiResponse)]
pub enum OptionalRoutineCompletionResponse {
    #[oai(status = 200)]
    Ok(Json<dtos::RoutineCompletion>),
    #[oai(status = 404)]
    /// Specified task not found.
    NotFound,
}

#[OpenApi]
impl PerformanceRESTService {
    #[oai(path = "/hello", method = "get")]
    pub async fn list_tasks(&self) -> Result<HelloResponse> {
        Ok(HelloResponse::Ok(Json("Hello".to_string())))
    }

    #[oai(path = "/streak/:id", method = "get")]
    pub async fn get_streak(&self, Path(id): Path<Uuid>) -> Result<OptionalStreakResponse> {
        let streak = sqlx::query_as!(
            models::Streak,
            "SELECT * FROM score.performance WHERE user_id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(InternalServerError)?;

        match streak.map(dtos::Streak::from) {
            Some(streak) => Ok(OptionalStreakResponse::Ok(Json(dtos::Streak::from(streak)))),
            None => Ok(OptionalStreakResponse::NotFound),
        }
    }

    #[oai(path = "/routine/:task_id", method = "get")]
    pub async fn get_routine_completion(
        &self,
        Path(task_id): Path<Uuid>,
        Query(start_date): Query<Option<DateTime>>,
        Query(end_date): Query<Option<DateTime>>,
    ) -> Result<Json<Vec<dtos::RoutineCompletion>>> {
        let rc = sqlx::query_as!(
            models::RoutineCompletion,
            "SELECT * FROM score.routine_completion WHERE (completed_at >= $1 OR $1 IS NULL) AND (completed_at <= $2 OR $2 IS NULL) AND (task_id=$3)",
            start_date, end_date, task_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(InternalServerError)?;

        let dto_routine_completions = rc.into_iter().map(dtos::RoutineCompletion::from).collect();
        Ok(Json(dto_routine_completions))
    }

    #[oai(path = "/habit/:task_id", method = "get")]
    pub async fn get_habit_history(
        &self,
        Path(task_id): Path<Uuid>,
        Query(start_date): Query<Option<DateTime>>,
        Query(end_date): Query<Option<DateTime>>,
    ) -> Result<Json<dtos::HabitHistoryResponse>> {
        let hhs = sqlx::query!(
            "SELECT * FROM score.habit_history WHERE (triggered_at >= $1 OR $1 IS NULL) AND (triggered_at <= $2 OR $2 IS NULL) AND (task_id=$3)",
            start_date, end_date, task_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(InternalServerError)?;

        let dates = hhs.iter().map(|hh| hh.triggered_at).collect();
        let growth = hhs
            .iter()
            .scan(1.0, |point, hh| {
                *point *= if hh.positive { 1.01 } else { 0.99 };
                let rounded_point = (*point * 1000_f64).round() / 1000.0;
                Some(rounded_point)
            })
            .collect();

        let dto_habit_history = dtos::HabitHistoryResponse {
            dates: dates,
            growth: growth,
            task_id,
        };

        Ok(Json(dto_habit_history))
    }
}

#[derive(serde::Deserialize, Debug)]
struct Env {
    port: u16,
    database_url: String,
    log_mongo_url: Option<String>,
    public_domain: Option<String>,
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

    let server_url = if let Some(domain) = env.public_domain {
        if domain.contains("://") {
            format!("{}:{}/api", domain, env.port)
        } else {
            format!("https://{}:{}/api", domain, env.port)
        }
    } else {
        format!("http://localhost:{}/api", env.port)
    };

    let api_service = OpenApiService::new(
        PerformanceRESTService { pool: pool.clone() },
        "TODODODO - Performance Service",
        "1.0",
    )
    .server(server_url);
    let ui = api_service.openapi_explorer();
    let spec = api_service.spec_endpoint();

    // Service & Route
    let service = PerformanceGRPCService { pool };
    let route_grpc = RouteGrpc::new().add_service(PerformanceServer::new(service));
    let route = Route::new()
        .nest("/", route_grpc)
        .nest("/api", api_service)
        .nest("/docs", ui)
        .nest("/docs-json", spec)
        .with(middleware::Cors::new())
        .with(middleware::CatchPanic::default())
        .with(middleware::Tracing);

    Server::new(TcpListener::bind(format!("0.0.0.0:{}", env.port)))
        .run(route)
        .await
        .with_context(|| format!("Fail to start server on port {:?}", env.port))?;

    Ok(())
}
