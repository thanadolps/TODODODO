pub mod account;
pub mod community;
pub mod invite_code;

#[derive(poem_openapi::Tags)]
pub enum Tags {
    Account,
    Community,
    Invite,
}
