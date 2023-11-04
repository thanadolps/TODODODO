use uuid::Uuid;

use sqlx::FromRow;
use time::OffsetDateTime as DateTime;

#[derive(FromRow, Debug)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub deadline: Option<DateTime>,
    pub completed: bool,

    pub user_id: Uuid,
    pub community_id: Option<Uuid>,
}
#[derive(FromRow, Debug)]
pub struct Habit {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub score: i32,
    pub user_id: Uuid,
}

#[derive(FromRow, Debug)]
pub struct Routine {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub checktime: Option<DateTime>,
    pub typena: String,
    pub user_id: Uuid,
}
