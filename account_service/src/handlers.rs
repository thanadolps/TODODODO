use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use poem::{
    error::{InternalServerError, NotFound, NotFoundError, Unauthorized},
    Result,
};
use poem_openapi::{
    payload::Json,
    types::{Email, Password},
    ApiResponse, Object, OpenApi,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::PrimitiveDateTime;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
// JWT claims
// https://www.iana.org/assignments/jwt/jwt.xhtml
struct Claims {
    // Time after which the JWT expires
    exp: usize,
    // Time at which the JWT was issued
    iat: usize,
    // Subject of the JWT (the user)
    sub: Uuid,
}

impl Claims {
    pub fn new(sub: Uuid) -> Self {
        Self {
            exp: (SystemTime::now() + Duration::from_secs(60 * 60 * 24 * 365))
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as usize,
            iat: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as usize,
            sub,
        }
    }
}

#[derive(Object)]
struct AuthRequest {
    email: Email,
    #[oai(validator(max_length = 32))]
    password: Password,
}

#[derive(ApiResponse)]
enum AuthResponse {
    #[oai(status = 200)]
    Ok(Json<String>),
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
    #[oai(status = 200)]
    Ok(Json<String>),
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

#[OpenApi]
impl Api {
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
                let claims = Claims::new(account.id);
                let token = jsonwebtoken::encode(
                    &jsonwebtoken::Header::default(),
                    &claims,
                    &self.encode_key,
                )
                .map_err(InternalServerError)?;
                return Ok(AuthResponse::Ok(Json(token)));
            }
            // Database unique violation when email or username already exists
            Err(sqlx::Error::Database(err))
                if err.kind() == sqlx::error::ErrorKind::UniqueViolation =>
            {
                return Ok(AuthResponse::Conflict);
            }
            Err(err) => return Err(InternalServerError(err)),
        };
    }

    #[oai(path = "/login", method = "post")]
    async fn login(&self, body: Json<AuthRequest>) -> Result<RegisterResponse> {
        let account = sqlx::query!(
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

        let claims = Claims::new(account.id);
        let token =
            jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &self.encode_key)
                .map_err(InternalServerError)?;
        Ok(RegisterResponse::Ok(Json(token)))
    }
}
