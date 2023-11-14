use crate::{
    dtos,
    models,
};
use gengrpc::performance::{PerformanceClient, StreakDetail};
use poem::error::InternalServerError;
use poem::Result;
use poem_grpc::Request;
use poem_openapi::{
    param::{Path, Query},
    payload::Json,
    ApiResponse, OpenApi,
};

use uuid::Uuid;

#[derive(ApiResponse)]
pub enum OptionalTaskResponse {
    #[oai(status = 200)]
    Ok(Json<dtos::Task>),
    #[oai(status = 404)]
    /// Specified task not found.
    NotFound,
}

#[derive(ApiResponse)]
pub enum OptionalSubtaskResponse {
    #[oai(status = 200)]
    Ok(Json<dtos::Subtask>),
    #[oai(status = 404)]
    /// Specified task not found.
    NotFound,
}


#[derive(ApiResponse)]
pub enum OptionalTaskWithSubtasksResponse {
    #[oai(status = 200)]
    Ok(Json<dtos::TaskWithSubtasks>),
    #[oai(status = 404)]
    /// Specified task not found.
    NotFound,
}


pub struct Api {
    pub pool: sqlx::PgPool,
    pub performance: PerformanceClient,
}

#[OpenApi(tag=super::Tags::Task)]
impl Api {
    #[oai(path = "/task", method = "get")]
    /// List all tasks.
    pub async fn list_tasks(&self, Query(user_id): Query<Option<Uuid>>) -> Result<Json<Vec<dtos::TaskWithSubtasks>>> {
        let tasks = sqlx::query_as!(models::Task, "SELECT * FROM task WHERE (user_id = $1 OR $1 IS NULL) ORDER BY created_at ASC", user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(InternalServerError)?;

            let mut dto_tasks_with_subtasks = Vec::new();

            for task in tasks {
                let subtasks = sqlx::query_as!(
                    models::Subtask,
                    "SELECT * FROM task.subtask WHERE task_id = $1 ORDER BY created_at ASC",
                    task.id
                )
                .fetch_all(&self.pool)
                .await
                .map_err(InternalServerError)?;
        
                let task_with_subtasks = models::TaskWithSubtasks {
                    id: task.id,
                    user_id: task.user_id,
                    community_id: task.community_id,
                    completed: task.completed,
                    deadline: task.deadline,
                    description: task.description.to_string(),
                    title: task.title.to_string(),
                    created_at: task.created_at,
                    subtasks,
                };
        
                dto_tasks_with_subtasks.push(dtos::TaskWithSubtasks::from(task_with_subtasks));
            }
        
            Ok(Json(dto_tasks_with_subtasks))
    }

    #[oai(path = "/task/:id", method = "get")]
    /// Get a task by id.
    pub async fn get_task(&self, Path(id): Path<Uuid>) -> Result<OptionalTaskWithSubtasksResponse> {
        let task = sqlx::query_as!(models::Task, "SELECT * FROM task WHERE id = $1", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(InternalServerError)?;

        match task.map(dtos::Task::from) {
            Some(task) => {
                let subtasks = sqlx::query_as!(
                    models::Subtask,
                    "select * from task.subtask where task_id=$1 ORDER BY created_at ASC",
                    Some(task.id)
                )
                .fetch_all(&self.pool)
                .await
                .map_err(InternalServerError)?;

                let task_with_subtasks = models::TaskWithSubtasks{
                    id: task.id,
                    user_id: task.user_id,
                    community_id: task.community_id,
                    completed: task.completed,
                    deadline: task.deadline,
                    description: task.description.to_string(),
                    title: task.title.to_string(),
                    created_at: task.created_at,
                    subtasks
                };
                
                Ok(OptionalTaskWithSubtasksResponse::Ok(Json(dtos::TaskWithSubtasks::from(task_with_subtasks))))},
            None => Ok(OptionalTaskWithSubtasksResponse::NotFound),
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

    #[oai(path = "/subtask", method = "post")]
    /// Create new Subtask under a Task specified by ID
    pub async fn add_subtask(&self, Json(subtask): Json<dtos::Subtask>) -> Result<Json<dtos::Subtask>> {
        let subtask = sqlx::query_as!(
            models::Subtask,
            "INSERT INTO subtask (title, task_id) VALUES ($1, $2) RETURNING *",
            subtask.title,
            Some(subtask.task_id)
        )
        .fetch_one(&self.pool)
        .await
        .map_err(InternalServerError)?;

        Ok(Json(dtos::Subtask::from(subtask)))
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
            tracing::info!("Adding streak...");
            self.performance.add_streak(Request::new(StreakDetail {
                user_id: completed.unwrap().user_id.to_string()
            })).await.map_err(InternalServerError)?;
        }

        Ok(())
    }

    #[oai(path = "/subtask/:id/check", method = "patch")]
    /// Check Subtask
    pub async fn check_subtask(&self, Path(id): Path<Uuid>) -> Result<()> {
        #[derive(Debug)]
        struct Result {
            completed: Option<bool>
        } 
        let completed = sqlx::query_as!(Result, "
        UPDATE task.subtask SET completed = true WHERE id=$1 RETURNING completed", id)
            .fetch_optional(&self.pool) 
            .await.map_err(InternalServerError)?;  

        tracing::info!("result: {:?}", completed);

        Ok(())
    }

    #[oai(path = "/subtask/:id/uncheck", method = "patch")]
    /// Uncheck Subtask
    pub async fn complete_subtask(&self, Path(id): Path<Uuid>) -> Result<()> {
        #[derive(Debug)]
        struct Result {
            completed: Option<bool>
        } 
        let completed = sqlx::query_as!(Result, "
        UPDATE task.subtask SET completed = false WHERE id=$1 RETURNING completed", id)
            .fetch_optional(&self.pool) 
            .await.map_err(InternalServerError)?;  

        tracing::info!("result: {:?}", completed);

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

    #[oai(path = "/subtask/:id", method = "delete")]
    /// Delete a subtask by id.
    pub async fn delete_subtask(&self, Path(id): Path<Uuid>) -> Result<OptionalSubtaskResponse> {
        let subtask = sqlx::query_as!(
            models::Subtask,
            "DELETE FROM subtask WHERE id = $1 RETURNING *",
            id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(InternalServerError)?;

        match subtask.map(dtos::Subtask::from) {
            Some(subtask) => Ok(OptionalSubtaskResponse::Ok(Json(subtask))),
            None => Ok(OptionalSubtaskResponse::NotFound),
        }
    }
}