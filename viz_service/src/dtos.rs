use poem_openapi::Object;

use structmapper::StructMapper;
use time::OffsetDateTime as DateTime;
use uuid::Uuid;

#[derive(Object, StructMapper)]
#[struct_mapper(from_type = "crate::models::Streak")]
pub struct Streak {
    #[oai(read_only)]
    pub user_id: Uuid,
    pub combo: i32,
    pub best_record: i32,
}

#[derive(Object, StructMapper)]
#[struct_mapper(from_type = "crate::models::RoutineCompletion")]
pub struct RoutineCompletion {
    #[oai(read_only)]
    pub task_id: Uuid,
    #[oai(read_only)]
    pub completed_at: Option<DateTime>,
}
