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
