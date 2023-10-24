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
