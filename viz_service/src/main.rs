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
use sqlx::{postgres::types::PgInterval, PgPool};
use time::{Date, OffsetDateTime as DateTime};
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
            "INSERT INTO routine_completion VALUES($1,$2,$3)",
            uuid,
            converted_completed_at,
            request.typena,
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
            Some(streak) => Ok(OptionalStreakResponse::Ok(Json(streak))),
            None => Ok(OptionalStreakResponse::NotFound),
        }
    }

    #[oai(path = "/routine", method = "get")]
    pub async fn list_routine_completions(
        &self,
        Query(task_ids): Query<Vec<Uuid>>,
        Query(start_date): Query<Option<DateTime>>,
        Query(end_date): Query<Option<DateTime>>,
    ) -> Result<Json<Vec<dtos::RoutineCompletionResponse>>> {
        let rcs: Vec<models::RoutineCompletion> = sqlx::query_as!(
            models::RoutineCompletion,
            "SELECT * FROM score.routine_completion WHERE (task_id=ANY($1))",
            &task_ids
        )
        .fetch_all(&self.pool)
        .await
        .map_err(InternalServerError)?;

        let mut responses = Vec::new();
        for rc in &rcs {
            let typena = rc.typena.clone();
            let (date_trunc, interval) = match typena.as_str() {
                "daily" => (
                    "day",
                    PgInterval {
                        months: 0,
                        days: 1,
                        microseconds: 0,
                    },
                ),
                "weekly" => (
                    "week",
                    PgInterval {
                        months: 0,
                        days: 7,
                        microseconds: 0,
                    },
                ),
                "monthly" => (
                    "month",
                    PgInterval {
                        months: 1,
                        days: 0,
                        microseconds: 0,
                    },
                ),
                _ => (
                    "day",
                    PgInterval {
                        months: 0,
                        days: 1,
                        microseconds: 0,
                    },
                ),
            };
            let start_date_pg: Option<Date> = start_date.map(|start_date| start_date.date());
            let end_date_pg: Option<Date> = end_date.map(|end_date| end_date.date());
            let bool_arr = sqlx::query_scalar!("SELECT ARRAY(
                SELECT 
                    CASE 
                        WHEN EXISTS (
                            SELECT 1 
                            FROM score.routine_completion rc 
                            WHERE date_trunc($1, rc.completed_at::date) = s.series_date and task_id=$2
                        ) THEN true
                        ELSE false
                    END
                FROM (SELECT date_trunc($1, generate_series) AS series_date
                FROM generate_series($3::date, $4::date, $5::interval) AS generate_series) as s
            ) AS result_array;
            ", date_trunc,rc.task_id, start_date_pg, end_date_pg, interval).fetch_one(&self.pool)
            .await
            .map_err(InternalServerError)?;

            let dates_pg = sqlx::query_scalar!(r#"SELECT date_trunc($1, generate_series) as "dates!" FROM generate_series($2::date, $3::date, $4::interval) AS generate_series;"#, date_trunc, start_date_pg, end_date_pg, interval).fetch_all(&self.pool)
            .await
            .map_err(InternalServerError)?;
            let response = dtos::RoutineCompletionResponse {
                task_id: rc.task_id,
                dates: dates_pg,
                completions: bool_arr.unwrap_or_default(),
                typena: rc.typena.clone(),
            };
            responses.push(response);
        }

        Ok(Json(responses))
    }

    #[oai(path = "/habit", method = "get")]
    pub async fn list_habit_histories(
        &self,
        Query(task_ids): Query<Vec<Uuid>>,
        Query(start_date): Query<Option<DateTime>>,
        Query(end_date): Query<Option<DateTime>>,
    ) -> Result<Json<Vec<dtos::HabitHistoryResponse>>> {
        let habits = sqlx::query!(
            "SELECT task_id, ARRAY_AGG(habit_history.triggered_at) AS dates, 
            ARRAY_AGG(habit_history.positive) AS positives FROM score.habit_history 
            WHERE (triggered_at >= $1 OR $1 IS NULL) AND 
            (triggered_at <= $2 OR $2 IS NULL) AND 
            (task_id=ANY($3)) GROUP BY (task_id)",
            start_date,
            end_date,
            &task_ids
        )
        .fetch_all(&self.pool)
        .await
        .map_err(InternalServerError)?;

        let mut habit_history_response = Vec::new();

        for habit in habits {
            let dates = habit.dates;
            let growth = habit
                .positives
                .unwrap_or_default()
                .iter()
                .scan(1.0, |point, positive| {
                    *point *= if *positive { 1.01 } else { 0.99 };
                    let rounded_point = (*point * 1000_f64).round() / 1000.0;
                    Some(rounded_point)
                })
                .collect();

            let dto_habit_history = dtos::HabitHistoryResponse {
                dates: dates.unwrap_or_default(),
                growth,
                task_id: habit.task_id,
            };
            habit_history_response.push(dto_habit_history)
        }

        Ok(Json(habit_history_response))
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
    tracing_init(env.log_mongo_url).await?;

    // Setup database
    let pool = PgPool::connect(&env.database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    // OpenAPI
    let server_url = if let Some(domain) = env.public_domain {
        if domain.contains("://") {
            domain
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
