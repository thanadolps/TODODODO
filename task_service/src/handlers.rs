use crate::{dtos, models};

use gengrpc::performance::{PerformanceClient, StreakDetail};
use poem::error::InternalServerError;
use poem::Result;
use poem_grpc::Request;
use poem_openapi::{param::Path, payload::Json, ApiResponse, OpenApi};
use uuid::Uuid;

#[derive(ApiResponse)]
pub enum OptionalTaskResponse {
    #[oai(status = 200)]
    Ok(Json<dtos::Task>),
    #[oai(status = 404)]
    /// Specified task not found.
    NotFound,
}

pub struct Api {
    pub pool: sqlx::PgPool,
    pub performance: PerformanceClient
}

#[OpenApi]
impl Api {
    #[oai(path = "/task", method = "get")]
    /// List all tasks.
    pub async fn list_tasks(&self) -> Result<Json<Vec<dtos::Task>>> {
        let tasks = sqlx::query_as!(models::Task, "SELECT * FROM task")
            .fetch_all(&self.pool)
            .await
            .map_err(InternalServerError)?;

        let dto_tasks = tasks.into_iter().map(dtos::Task::from).collect();
        Ok(Json(dto_tasks))
    }

    #[oai(path = "/task/:id", method = "get")]
    /// Get a task by id.
    pub async fn get_task(&self, Path(id): Path<Uuid>) -> Result<OptionalTaskResponse> {
        let task = sqlx::query_as!(models::Task, "SELECT * FROM task WHERE id = $1", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(InternalServerError)?;

        match task.map(dtos::Task::from) {
            Some(task) => Ok(OptionalTaskResponse::Ok(Json(task))),
            None => Ok(OptionalTaskResponse::NotFound),
        }
    }

    #[oai(path = "/task", method = "post")]
    /// Add a new task.
    pub async fn add_task(&self, Json(task): Json<dtos::Task>) -> Result<Json<dtos::Task>> {
        let task = sqlx::query_as!(
            models::Task,
            "INSERT INTO task (title, description, deadline, user_id) VALUES ($1, $2, $3, $4) RETURNING *",
            task.title,
            task.description, 
            task.deadline,
            task.user_id 
        )
        .fetch_one(&self.pool)
        .await
        .map_err(InternalServerError)?;

        Ok(Json(dtos::Task::from(task)))
    }

    #[oai(path = "/task/:id", method = "put")]
    /// Update a task by id.
    pub async fn update_task(
        &self,
        Path(id): Path<Uuid>,
        Json(task): Json<dtos::Task>,
    ) -> Result<OptionalTaskResponse> {
        let task = sqlx::query_as!(
            models::Task,
            "UPDATE task SET title = $1, description = $2, deadline = $3 WHERE id = $4 RETURNING *",
            task.title,
            task.description,
            task.deadline,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(InternalServerError)?;

        match task.map(dtos::Task::from) {
            Some(task) => Ok(OptionalTaskResponse::Ok(Json(task))),
            None => Ok(OptionalTaskResponse::NotFound),
        } 
    }

    #[oai(path = "/task/:id/complete", method = "patch")]
    /// Complete Task
    pub async fn complete_task(&self, Path(id): Path<Uuid>) -> Result<()> {
        #[derive(Debug)]
        struct Result {
            user_id: Uuid,
            completed: Option<bool>
        } 

        let completed: Option<Result> = sqlx::query_as!(Result, "
        UPDATE task.task SET completed = true WHERE id=$1 RETURNING user_id, (
            select completed from task.task WHERE id=$1
        ) as completed;
        ", id)
            .fetch_optional(&self.pool) 
            .await.map_err(InternalServerError)?;  

        tracing::info!("result: {:?}", completed);

        if matches!(completed, Some(Result {completed: Some(false), ..})) {
            tracing::info!("Adding streak");
            self.performance.add_streak(Request::new(StreakDetail {
                user_id: completed.unwrap().user_id.to_string()
            })).await.map_err(InternalServerError)?;
        }

        Ok(())
    }

    #[oai(path = "/task/:id", method = "delete")]
    /// Delete a task by id.
    pub async fn delete_task(&self, Path(id): Path<Uuid>) -> Result<OptionalTaskResponse> {
        let task = sqlx::query_as!(
            models::Task,
            "DELETE FROM task WHERE id = $1 RETURNING *",
            id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(InternalServerError)?;

        match task.map(dtos::Task::from) {
            Some(task) => Ok(OptionalTaskResponse::Ok(Json(task))),
            None => Ok(OptionalTaskResponse::NotFound),
        }
    }
}
