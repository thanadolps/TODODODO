use crate::{
    dtos,
    models,
};
use gengrpc::performance::{PerformanceClient, RoutineDetail};
use poem::error::InternalServerError;
use poem::Result;
use poem_grpc::Request;
use poem_openapi::{
    param::{Path, Query},
    payload::Json,
    ApiResponse, OpenApi,
};
use std::time::SystemTime;
use uuid::Uuid;

#[derive(ApiResponse)]
pub enum OptionalRoutineResponse {
    #[oai(status = 200)]
    Ok(Json<dtos::Routine>),
    #[oai(status = 404)]
    /// Specified task not found.
    NotFound,
}


pub struct Api {
    pub pool: sqlx::PgPool,
    pub performance: PerformanceClient,
}

#[OpenApi(tag=super::Tags::Routine)]
impl Api {
    #[oai(path = "/routine", method = "get")]
    /// List all routines.
    pub async fn list_routines(&self, Query(user_id): Query<Option<Uuid>>) -> Result<Json<Vec<dtos::Routine>>> {
        let routines = sqlx::query_as!(models::Routine, "SELECT * FROM routine WHERE (user_id = $1 OR $1 IS NULL) ORDER BY created_at ASC", user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(InternalServerError)?;

        let dto_routines = routines.into_iter().map(dtos::Routine::from).collect();
        Ok(Json(dto_routines))
    }

    #[oai(path = "/routine/:id", method = "get")]
    /// Get a routine by id.
    pub async fn get_routine(&self, Path(id): Path<Uuid>) -> Result<OptionalRoutineResponse> {
        let routine = sqlx::query_as!(models::Routine, "SELECT * FROM routine WHERE id = $1", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(InternalServerError)?;

        match routine.map(dtos::Routine::from) {
            Some(routine) => Ok(OptionalRoutineResponse::Ok(Json(routine))),
            None => Ok(OptionalRoutineResponse::NotFound),
        }
    }

    #[oai(path = "/routine", method = "post")]
    /// Add a new routine.
    pub async fn add_routine(&self, Json(routine): Json<dtos::Routine>) -> Result<Json<dtos::Routine>> {

        let routine = sqlx::query_as!(
            models::Routine,
            "INSERT INTO routine (title, description, typena, user_id) VALUES ($1, $2, $3,$4) RETURNING *",
            routine.title,
            routine.description, 
            routine.typena,
            routine.user_id 
        )
        .fetch_one(&self.pool)
        .await
        .map_err(InternalServerError)?;

        Ok(Json(dtos::Routine::from(routine)))
    }


    #[oai(path = "/routine/:id", method = "put")]
    /// Update a routine by id.
    pub async fn update_routine(
        &self,
        Path(id): Path<Uuid>,
        Json(routine): Json<dtos::Routine>,
    ) -> Result<OptionalRoutineResponse> {
        let routine = sqlx::query_as!(
            models::Routine,
            "UPDATE routine SET title = $1, description = $2, typena = $3 WHERE id = $4 RETURNING *",
            routine.title,
            routine.description,
            routine.typena,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(InternalServerError)?;

        match routine.map(dtos::Routine::from) {
            Some(routine) => Ok(OptionalRoutineResponse::Ok(Json(routine))),
            None => Ok(OptionalRoutineResponse::NotFound),
        } 
    }

    #[oai(path = "/routine/:id", method = "delete")]
    /// Delete a routine by id.
    pub async fn delete_routine(&self, Path(id): Path<Uuid>) -> Result<OptionalRoutineResponse> {
        let routine = sqlx::query_as!(
            models::Routine,
            "DELETE FROM routine WHERE id = $1 RETURNING *",
            id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(InternalServerError)?;

        match routine.map(dtos::Routine::from) {
            Some(routine) => Ok(OptionalRoutineResponse::Ok(Json(routine))),
            None => Ok(OptionalRoutineResponse::NotFound),
        }
    }

    #[oai(path = "/rountine/:id/complete", method = "patch")]
    /// Complete routine
    pub async fn complete_routine(&self, Path(id): Path<Uuid>) -> Result<OptionalRoutineResponse> {

        // Tell Database
        let routine = sqlx::query_as!(models::Routine, "
        UPDATE routine SET completed = true WHERE id=$1 RETURNING *
        ", id)
            .fetch_optional(&self.pool) 
            .await.map_err(InternalServerError)?;  

            match routine.map(dtos::Routine::from) {
                Some(routine) => {
        
                    tracing::info!("Completing routine...");
                    self.performance.complete_routine(Request::new(RoutineDetail {
                        task_id: id.to_string(),
                        completed_at: Some(SystemTime::now().into()),
                        typena: routine.typena.clone(),
                    })).await.map_err(InternalServerError)?;
                    Ok(OptionalRoutineResponse::Ok(Json(routine)))
                },
                None => Ok(OptionalRoutineResponse::NotFound),
            }
    }
}