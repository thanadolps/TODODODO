use uuid::Uuid;

use sqlx::FromRow;
use time::OffsetDateTime as DateTime;

#[derive(FromRow, Debug)]
pub struct Streak {
    pub user_id: Uuid,
    pub combo: i32,
    pub best_record: i32,
}
