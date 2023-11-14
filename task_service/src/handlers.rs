mod habit;
mod routine;
mod task;

use gengrpc::performance::PerformanceClient;
use poem_openapi::OpenApi;

#[derive(poem_openapi::Tags)]
enum Tags {
    Task,
    Habit,
    Routine,
}

pub fn handlers(pool: &sqlx::PgPool, performance: &PerformanceClient) -> impl OpenApi {
    (
        task::Api {
            pool: pool.clone(),
            performance: performance.clone(),
        },
        habit::Api {
            pool: pool.clone(),
            performance: performance.clone(),
        },
        routine::Api {
            pool: pool.clone(),
            performance: performance.clone(),
        },
    )
}
