use std::time::Duration;

use gengrpc::community_task::{
    AddCommunityTaskRequest, CommunityTaskService, CommunityTaskServiceServer,
};
use poem_grpc::{Code, Response, Status};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

pub fn task_service_server(pool: PgPool) -> CommunityTaskServiceServer<TaskService> {
    CommunityTaskServiceServer::new(TaskService { pool })
}

pub struct TaskService {
    pool: PgPool,
}

#[poem::async_trait]
impl CommunityTaskService for TaskService {
    async fn add_community_task(
        &self,
        request: poem_grpc::Request<AddCommunityTaskRequest>,
    ) -> Result<Response<()>, Status> {
        // Parse and convert request
        let community_id = request
            .community_id
            .parse::<Uuid>()
            .map_err(|err| Status::new(Code::InvalidArgument).with_message(err))?;

        let member_uuids = request
            .members
            .iter()
            .map(|m| m.parse())
            .collect::<Result<Vec<Uuid>, _>>()
            .map_err(|err| Status::new(Code::InvalidArgument).with_message(err))?;

        let deadline = match &request.deadline {
            Some(d) => Some(
                OffsetDateTime::from_unix_timestamp(d.seconds)
                    .map_err(|err| Status::new(Code::InvalidArgument).with_message(err))?
                    + Duration::from_nanos(d.nanos.max(0) as u64),
            ),
            None => None,
        };

        // Insert into database
        // insert task for all member, then insert all subtasks for each inserted task
        sqlx::query!(
            "
            WITH task AS (
                INSERT INTO task (title, description, deadline, community_id, user_id)
                SELECT $1, $2, $3, $4, * FROM UNNEST($5::uuid[])
                RETURNING id
            )
            INSERT INTO subtask (title, task_id)
            SELECT st.*, task.id FROM UNNEST($6::text[]) as st CROSS JOIN task;
        ",
            request.title,
            request.description,
            deadline,
            community_id,
            &member_uuids,
            &request.subtasks
        )
        .execute(&self.pool)
        .await
        .map_err(Status::from_std_error)?;

        Ok(Response::new(()))
    }
}
