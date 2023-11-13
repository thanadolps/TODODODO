use crate::{dtos::{self}, models};
use std::time::{SystemTime};
use gengrpc::performance::{PerformanceClient, StreakDetail, RoutineDetail, HabitDetail};
use poem::error::InternalServerError;
use poem::Result;
use poem_grpc::Request;
use poem_openapi::{param::{Path, Query}, payload::Json, ApiResponse, OpenApi};
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
pub enum OptionalTaskWithSubtasksResponse {
    #[oai(status = 200)]
    Ok(Json<dtos::TaskWithSubtasks>),
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
pub enum OptionalHabitResponse {
    #[oai(status = 200)]
    Ok(Json<dtos::Habit>),
    #[oai(status = 404)]
    /// Specified task not found.
    NotFound,
}

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
    pub performance: PerformanceClient
}

#[OpenApi]
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
                        completed_at: Some(SystemTime::now().into())
                    })).await.map_err(InternalServerError)?;
                    Ok(OptionalRoutineResponse::Ok(Json(routine)))
                },
                None => Ok(OptionalRoutineResponse::NotFound),
            }
    }

   
    
}