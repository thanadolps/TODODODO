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
    #[oai(read_only)]
    pub completed: bool,

    pub user_id: Uuid,
    pub community_id: Option<Uuid>,
}

#[derive(Object, StructMapper)]
#[struct_mapper(from_type = "crate::models::TaskWithSubtasks")]
pub struct TaskWithSubtasks {
    #[oai(read_only)]
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub deadline: Option<DateTime>,
    #[oai(read_only)]
    pub completed: bool,

    pub user_id: Uuid,
    pub community_id: Option<Uuid>,
    pub subtasks: Vec<Subtask>,
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
    #[oai(read_only)]
    pub created_at: Option<DateTime>,
}

#[derive(Object, StructMapper)]
#[struct_mapper(from_type = "crate::models::Habit")]
pub struct Habit {
    #[oai(read_only)]
    pub id: Uuid,
    pub title: String,
    pub description: String,
    #[oai(read_only)]
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
    #[oai(read_only)]
    pub checktime: Option<DateTime>,
    pub typena: String,
    pub user_id: Uuid,
    #[oai(read_only)]
    pub completed: bool,
}
