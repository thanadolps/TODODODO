use std::time::{Duration, SystemTime, UNIX_EPOCH};

use jsonwebtoken::{DecodingKey, Validation};
use poem::{error::Unauthorized, http::StatusCode, Error, Request, Result};
use poem_openapi::{auth::Bearer, SecurityScheme};
use serde::{Deserialize, Serialize};

use uuid::Uuid;

/// JWT claims
/// https://www.iana.org/assignments/jwt/jwt.xhtml
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Time after which the JWT expires
    pub exp: usize,
    /// Time at which the JWT was issued
    pub iat: usize,
    /// Subject of the JWT (user id)
    pub sub: Uuid,
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

/// JWT bearer auth scheme.
#[derive(SecurityScheme)]
#[oai(ty = "bearer", bearer_format = "jwt", checker = "verify_bearer")]
pub struct JWTAuth(
    /// [JWT claims](https://auth0.com/docs/secure/tokens/json-web-tokens/json-web-token-claims)
    pub Claims,
);

async fn verify_bearer(req: &Request, bearer: Bearer) -> Result<Claims> {
    // Retrive the injected decoding key, (injected in main.rs with `.data`).
    // if the code panic here, it's most likely because you forgot to inject the decoding key
    // https://docs.rs/poem/latest/poem/web/struct.Data.html#example
    let decode_key = req.data::<DecodingKey>().ok_or(Error::from_string(
        "Decode key not provided",
        StatusCode::INTERNAL_SERVER_ERROR,
    ))?;

    let decoded = jsonwebtoken::decode::<Claims>(&bearer.token, decode_key, &Validation::default())
        .map_err(Unauthorized)?;

    Ok(decoded.claims)
}
