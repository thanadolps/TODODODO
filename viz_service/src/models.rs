use uuid::Uuid;

use sqlx::FromRow;
use time::OffsetDateTime as DateTime;

#[derive(FromRow, Debug)]
pub struct Streak {
    pub user_id: Uuid,
    pub combo: i32,
    pub best_record: i32,
}

#[derive(FromRow, Debug)]
pub struct RoutineCompletion {
    pub task_id: Uuid,
    pub completed_at: Option<DateTime>,
}

#[derive(FromRow, Debug)]
pub struct HabitHistory {
    pub task_id: Uuid,
    pub positive: bool,
    pub triggered_at: Option<DateTime>,
}
