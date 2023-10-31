use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};

use poem::{
    error::{InternalServerError, NotFoundError},
    Result,
};
use poem_openapi::{
    payload::{Json, PlainText},
    types::{Email, Password},
    ApiResponse, Object, OpenApi,
};

use sqlx::PgPool;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::jwt;

#[derive(Object)]
struct AuthRequest {
    email: Email,
    #[oai(validator(max_length = 32))]
    password: Password,
}

#[derive(ApiResponse)]
enum AuthResponse {
    /// Returns JWT token
    #[oai(status = 200)]
    Ok(PlainText<String>),
    #[oai(status = 409)]
    Conflict,
}

#[derive(Object)]
struct RegisterRequest {
    #[oai(validator(max_length = 64))]
    username: String,
    email: Email,
    #[oai(validator(max_length = 32))]
    password: Password,
}

#[derive(ApiResponse)]
enum RegisterResponse {
    /// Returns JWT token
    #[oai(status = 200)]
    Ok(PlainText<String>),
    #[oai(status = 401)]
    Unauthorized,
}

struct Account {
    id: Uuid,
    username: String,
    email: String,
    password_hash: String,
    created_at: PrimitiveDateTime,
    updated_at: PrimitiveDateTime,
}

pub struct Api {
    pub pool: PgPool,
    pub encode_key: jsonwebtoken::EncodingKey,
}

#[OpenApi(tag = "super::Tags::Account")]
impl Api {
    /// Register account
    #[oai(path = "/register", method = "post")]
    async fn register(&self, body: Json<RegisterRequest>) -> Result<AuthResponse> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(body.password.as_bytes(), &salt)
            .expect("to not fail hashing password")
            .to_string();

        match sqlx::query_as!(
            Account,
            "INSERT INTO account (username, email, password_hash) VALUES ($1, $2, $3) RETURNING *",
            body.username,
            body.email.to_string(),
            password_hash
        )
        .fetch_one(&self.pool)
        .await
        {
            Ok(account) => {
                let claims = jwt::Claims::new(account.id);
                let token = jsonwebtoken::encode(
                    &jsonwebtoken::Header::default(),
                    &claims,
                    &self.encode_key,
                )
                .map_err(InternalServerError)?;
                Ok(AuthResponse::Ok(PlainText(token)))
            }
            // Database unique violation when email or username already exists
            Err(sqlx::Error::Database(err))
                if err.kind() == sqlx::error::ErrorKind::UniqueViolation =>
            {
                Ok(AuthResponse::Conflict)
            }
            Err(err) => Err(InternalServerError(err)),
        }
    }

    /// Login to account
    #[oai(path = "/login", method = "post")]
    async fn login(&self, body: Json<AuthRequest>) -> Result<RegisterResponse> {
        let account: Account = sqlx::query_as!(
            Account,
            "SELECT * FROM account WHERE email = $1",
            body.email.to_string()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(InternalServerError)?
        .ok_or(NotFoundError)?;

        if let Err(err) = Argon2::default().verify_password(
            body.password.as_bytes(),
            &PasswordHash::new(&account.password_hash).unwrap(),
        ) {
            tracing::warn!("Failed to verify password: {}", err);
            return Ok(RegisterResponse::Unauthorized);
        }

        let claims = jwt::Claims::new(account.id);
        let token =
            jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &self.encode_key)
                .map_err(InternalServerError)?;
        Ok(RegisterResponse::Ok(PlainText(token)))
    }
}
