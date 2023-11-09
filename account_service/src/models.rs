use poem_openapi::Object;
use uuid::Uuid;

#[derive(Object)]
pub struct Community {
    #[oai(read_only)]
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_private: bool,
    #[oai(read_only)]
    pub owner_id: Uuid,
}
