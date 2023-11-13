pub mod account;
pub mod community;
pub mod community_task;
pub mod invite_code;

#[derive(poem_openapi::Tags)]
pub enum Tags {
    Account,
    Community,
    CommunityTask,
    Invite,
}
