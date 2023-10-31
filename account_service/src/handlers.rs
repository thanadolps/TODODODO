pub mod account;
pub mod community;

#[derive(poem_openapi::Tags)]
pub enum Tags {
    Account,
    Community,
}
