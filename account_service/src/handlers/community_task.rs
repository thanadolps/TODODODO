use std::time::SystemTime;

use gengrpc::community_task::{AddCommunityTaskRequest, CommunityTaskServiceClient};
use poem::{error::InternalServerError, http::StatusCode, Result};
use poem_grpc::{Request};
use poem_openapi::{param::Path, payload::Json, ApiResponse, Object, OpenApi};
use sqlx::PgPool;
use time::OffsetDateTime;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Object)]
struct CommunityTask {
    #[oai(read_only)]
    id: Uuid,
    #[oai(read_only)]
    community_id: Uuid,
    title: String,
    description: String,
    deadline: Option<OffsetDateTime>,
    subtasks: Vec<String>,
}

// ====================

#[derive(ApiResponse)]
enum GetResponse {
    #[oai(status = 200)]
    Ok(Json<CommunityTask>),
    #[oai(status = 404)]
    NotFound,
}

#[derive(ApiResponse)]
enum DeleteResponse {
    #[oai(status = 204)]
    Deleted,
    #[oai(status = 404)]
    NotFound,
}

pub struct Api {
    pub pool: PgPool,
    pub task_grpc: CommunityTaskServiceClient,
}

#[OpenApi(tag = "super::Tags::CommunityTask")]
impl Api {
    #[oai(path = "/community/:cid/task", method = "get")]
    /// List all tasks in a community
    async fn list_community_tasks(
        &self,
        Path(cid): Path<Uuid>,
    ) -> Result<Json<Vec<CommunityTask>>> {
        let tasks = sqlx::query_as!(
            CommunityTask,
            r#"
            SELECT *
            FROM community_task
            WHERE community_id = $1
            "#,
            cid
        )
        .fetch_all(&self.pool)
        .await
        .map_err(InternalServerError)?;

        Ok(Json(tasks))
    }

    #[oai(path = "/community/:cid/task/:id", method = "get")]
    /// Get task in community by id
    async fn get_community_task(
        &self,
        Path(cid): Path<Uuid>,
        Path(id): Path<Uuid>,
    ) -> Result<GetResponse> {
        let task = sqlx::query_as!(
            CommunityTask,
            r#"
            SELECT *
            FROM community_task
            WHERE id = $1 AND community_id = $2
            "#,
            id,
            cid
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(InternalServerError)?;

        Ok(match task {
            Some(task) => GetResponse::Ok(Json(task)),
            None => GetResponse::NotFound,
        })
    }

    #[oai(path = "/community/:cid/task", method = "post")]
    /// Create task in community
    ///
    /// All joined member in community will automatically recieved this task
    async fn create_community_task(
        &self,
        Path(cid): Path<Uuid>,
        Json(ct): Json<CommunityTask>,
    ) -> Result<Json<CommunityTask>> {
        // TODO: who have permission to create task?

        // Create the task in transaction
        let mut tx = self.pool.begin().await.map_err(InternalServerError)?;
        let task = sqlx::query_as!(
            CommunityTask,
            r#"
            INSERT INTO community_task (community_id, title, description, deadline, subtasks)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
            cid,
            ct.title,
            ct.description,
            ct.deadline,
            &ct.subtasks
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(InternalServerError)?;

        // Get all member in community
        let members = sqlx::query_scalar!(
            r#"
            SELECT account_id
            FROM user_join_community
            WHERE community_id = $1
            "#,
            cid
        )
        .fetch_all(&self.pool)
        .await
        .map_err(InternalServerError)?;

        // Create task for each member
        let result = self
            .task_grpc
            .add_community_task(Request::new(AddCommunityTaskRequest {
                community_id: cid.to_string(),
                members: members.iter().map(|m| m.to_string()).collect(),

                title: task.title.clone(),
                description: task.description.clone(),
                deadline: task.deadline.map(|d| SystemTime::from(d).into()),
                subtasks: task.subtasks.clone(),
            }))
            .await;
        match result {
            Err(status) => {
                // Rollback transaction if failed to create task for joined member
                error!(error_status = ?status, members=?members, "Failed to create task for joined member, undo creating community task");
                tx.rollback().await.map_err(InternalServerError)?;
                return Err(poem::Error::from_string(
                    "Failed to create task for joined member",
                    StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }
            Ok(_) => {
                // Commit transaction if success
                info!("Created task for joined member");
                tx.commit().await.map_err(InternalServerError)?;
            }
        }

        Ok(Json(task))
    }

    #[oai(path = "/community/task/:id", method = "delete")]
    /// Delete a task in community
    async fn delete_community_task(&self, Path(id): Path<Uuid>) -> Result<DeleteResponse> {
        let result = sqlx::query!(
            r#"
            DELETE FROM community_task
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(InternalServerError)?;

        if result.rows_affected() == 0 {
            Ok(DeleteResponse::NotFound)
        } else {
            Ok(DeleteResponse::Deleted)
        }
    }
}
