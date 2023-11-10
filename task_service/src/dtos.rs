use poem_openapi::Object;

use structmapper::StructMapper;
use time::OffsetDateTime as DateTime;
use uuid::Uuid;

#[derive(Object, StructMapper)]
#[struct_mapper(from_type = "crate::models::Task")]
pub struct Task {
    #[oai(read_only)]
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub deadline: Option<DateTime>,
    pub completed: bool,

    pub user_id: Uuid,
    pub community_id: Option<Uuid>,
}

#[derive(Object, StructMapper)]
#[struct_mapper(from_type = "crate::models::Subtask")]
pub struct Subtask {
    #[oai(read_only)]
    pub id: Uuid,
    pub title: String,
    #[oai(read_only)]
    pub completed: bool,
    pub task_id: Uuid,
}

#[derive(Object, StructMapper)]
#[struct_mapper(from_type = "crate::models::Habit")]
pub struct Habit {
    #[oai(read_only)]
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub score: i32,
    pub user_id: Uuid,
}

#[derive(Object, StructMapper)]
#[struct_mapper(from_type = "crate::models::Routine")]
pub struct Routine {
    #[oai(read_only)]
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub checktime: Option<DateTime>,
    pub typena: String,
    pub user_id: Uuid,
    pub completed: bool,
}
