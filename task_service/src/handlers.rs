use crate::{dtos, models};

use poem::error::InternalServerError;
use poem::Result;
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
