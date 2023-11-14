use crate::{
    dtos,
    models,
};
use gengrpc::performance::{HabitDetail, PerformanceClient, StreakDetail};
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
pub enum OptionalHabitResponse {
    #[oai(status = 200)]
    Ok(Json<dtos::Habit>),
    #[oai(status = 404)]
    /// Specified task not found.
    NotFound,
}


pub struct Api {
    pub pool: sqlx::PgPool,
    pub performance: PerformanceClient,
}

#[OpenApi(tag=super::Tags::Habit)]
impl Api {
    #[oai(path = "/habit", method = "get")]
    /// List all habits.
    pub async fn list_habits(&self, Query(user_id): Query<Option<Uuid>>) -> Result<Json<Vec<dtos::Habit>>> {
        let habits = sqlx::query_as!(models::Habit, "SELECT * FROM habit WHERE (user_id = $1 OR $1 IS NULL) ORDER BY created_at ASC", user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(InternalServerError)?;

        let dto_habits = habits.into_iter().map(dtos::Habit::from).collect();
        Ok(Json(dto_habits))
    }

    #[oai(path = "/habit/:id", method = "get")]
    /// Get a habit by id.
    pub async fn get_habit(&self, Path(id): Path<Uuid>) -> Result<OptionalHabitResponse> {
        let habit = sqlx::query_as!(models::Habit, "SELECT * FROM habit WHERE id = $1", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(InternalServerError)?;

        match habit.map(dtos::Habit::from) {
            Some(habit) => Ok(OptionalHabitResponse::Ok(Json(habit))),
            None => Ok(OptionalHabitResponse::NotFound),
        }
    }

    #[oai(path = "/habit/:id/increasescore", method = "put")]
    /// Increase score to habit by id.
    pub async fn increasescore(&self, Path(id): Path<Uuid>) -> Result<OptionalHabitResponse> {
        let habit = sqlx::query_as!(models::Habit, "UPDATE habit SET score = score + 1 WHERE id = $1 RETURNING *", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(InternalServerError)?;

        match habit.map(dtos::Habit::from) {
            Some(habit) => {
                tracing::info!("Increasing habit...");
                    self.performance.trigger_habit(Request::new(HabitDetail {
                        task_id: id.to_string(),
                        positive: true,
                        triggered_at: Some(SystemTime::now().into())
                    })).await.map_err(InternalServerError)?;

                    tracing::info!("Adding combo...");
                    struct Result {
                        user_id: Uuid,
                    } 
            
                    let result: Option<Result> = sqlx::query_as!(Result, "
                    SELECT user_id FROM habit WHERE id=$1
                    ", id)
                        .fetch_optional(&self.pool) 
                        .await.map_err(InternalServerError)?;  
                    self.performance.add_streak(Request::new(StreakDetail {
                        user_id: result.unwrap().user_id.to_string()
                    })).await.map_err(InternalServerError)?;
                
                    Ok(OptionalHabitResponse::Ok(Json(habit)))},
            None => Ok(OptionalHabitResponse::NotFound),
        }
    }

    #[oai(path = "/habit/:id/decreasescore", method = "put")]
    /// Decrease score to habit by id.
    pub async fn decreasescore(&self, Path(id): Path<Uuid>) -> Result<OptionalHabitResponse> {
        let habit = sqlx::query_as!(models::Habit, "UPDATE habit SET score = score - 1 WHERE id = $1 RETURNING *", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(InternalServerError)?;

        match habit.map(dtos::Habit::from) {
            Some(habit) => {
                tracing::info!("Decreasing habit...");
                    self.performance.trigger_habit(Request::new(HabitDetail {
                        task_id: id.to_string(),
                        positive: false,
                        triggered_at: Some(SystemTime::now().into())
                    })).await.map_err(InternalServerError)?;

                    tracing::info!("Adding combo...");
                    struct Result {
                        user_id: Uuid,
                    } 
            
                    let result: Option<Result> = sqlx::query_as!(Result, "
                    SELECT user_id FROM habit WHERE id=$1
                    ", id)
                        .fetch_optional(&self.pool) 
                        .await.map_err(InternalServerError)?;  
                    self.performance.reset_streak(Request::new(StreakDetail {
                        user_id: result.unwrap().user_id.to_string()
                    })).await.map_err(InternalServerError)?;

                    Ok(OptionalHabitResponse::Ok(Json(habit)))},
            None => Ok(OptionalHabitResponse::NotFound),
        }
    }

   

    #[oai(path = "/habit", method = "post")]
    /// Add a new habit.
    pub async fn add_habit(&self, Json(habit): Json<dtos::Habit>) -> Result<Json<dtos::Habit>> {
        let habit = sqlx::query_as!(
            models::Habit,
            "INSERT INTO habit (title, description, score, user_id) VALUES ($1, $2, $3, $4) RETURNING *",
            habit.title,
            habit.description, 
            0,
            habit.user_id 
        )
        .fetch_one(&self.pool)
        .await
        .map_err(InternalServerError)?;

        Ok(Json(dtos::Habit::from(habit)))
    }


    #[oai(path = "/habit/:id", method = "put")]
    /// Update a habit by id.
    pub async fn update_habit(
        &self,
        Path(id): Path<Uuid>,
        Json(habit): Json<dtos::Habit>,
    ) -> Result<OptionalHabitResponse> {
        let habit = sqlx::query_as!(
            models::Habit,
            "UPDATE habit SET title = $1, description = $2, score = $3 WHERE id = $4 RETURNING *",
            habit.title,
            habit.description,
            habit.score,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(InternalServerError)?;

        match habit.map(dtos::Habit::from) {
            Some(habit) => Ok(OptionalHabitResponse::Ok(Json(habit))),
            None => Ok(OptionalHabitResponse::NotFound),
        } 
    }

    #[oai(path = "/habit/:id", method = "delete")]
    /// Delete a habit by id.
    pub async fn delete_habit(&self, Path(id): Path<Uuid>) -> Result<OptionalHabitResponse> {
        let habit = sqlx::query_as!(
            models::Habit,
            "DELETE FROM habit WHERE id = $1 RETURNING *",
            id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(InternalServerError)?;

        match habit.map(dtos::Habit::from) {
            Some(habit) => Ok(OptionalHabitResponse::Ok(Json(habit))),
            None => Ok(OptionalHabitResponse::NotFound),
        }
    }
}