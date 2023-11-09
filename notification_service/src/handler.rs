use poem::{error::InternalServerError, Result};
use poem_openapi::{param::Path, payload::Json, ApiResponse, OpenApi};
use tracing::{info, warn};
use url::Url;
use uuid::Uuid;

pub struct Api {
    pub pool: sqlx::PgPool,
}

#[derive(ApiResponse)]
enum GetResponse {
    #[oai(status = 200)]
    Ok(Json<Option<Url>>),
    #[oai(status = 404)]
    NotFound,
}

#[derive(ApiResponse)]
enum DeleteResponse {
    #[oai(status = 200)]
    Ok,
    #[oai(status = 404)]
    NotFound,
}

#[OpenApi]
impl Api {
    /// Get webhook url for a user
    #[oai(path = "/webhook/:user_id", method = "get")]
    async fn get_webhook(&self, Path(user_id): Path<Uuid>) -> Result<GetResponse> {
        match sqlx::query_scalar!("SELECT url FROM webhook WHERE user_id = $1", user_id)
            .fetch_optional(&self.pool)
            .await
        {
            Err(err) => Err(InternalServerError(err)),
            Ok(None) => Ok(GetResponse::NotFound),
            // webhook is null
            Ok(Some(None)) => Ok(GetResponse::Ok(Json(None))),
            // webhook is not null
            Ok(Some(Some(url))) => {
                let url = Url::parse(&url).map_err(|err| InternalServerError(err))?;
                Ok(GetResponse::Ok(Json(Some(url))))
            }
        }
    }

    /// Set webhook url for a user
    #[oai(path = "/webhook/:user_id", method = "post")]
    async fn set_webhook(
        &self,
        Path(user_id): Path<Uuid>,
        Json(body): Json<Url>,
    ) -> Result<Json<Url>> {
        info!(?user_id, ?body, "Setting webhook");

        // Warn if likly not discord webhook
        if let Some(host) = body.host() {
            use url::Host;
            match host {
                Host::Domain("discord.com") => {}
                Host::Domain(_) => warn!(?body, "received webhook url with unknown domain"),
                _ => warn!(?body, "received webhook url with no domain"),
            }
        } else {
            warn!(?body, "received webhook url without hostname")
        }

        // Query
        let url = sqlx::query_scalar!(
            "INSERT INTO webhook (user_id, url) VALUES ($1, $2) ON CONFLICT (user_id) DO UPDATE SET url = $2 RETURNING url",
            user_id,
            body.to_string()
        ).fetch_one(&self.pool).await.map_err(InternalServerError)?;
        let url = url.expect("url must exist after just inserting it");
        let url = Url::parse(&url).expect("data from this url columns must be valid url");

        Ok(Json(url))
    }

    /// Remove webhook url for a user
    #[oai(path = "/webhook/:user_id", method = "delete")]
    async fn delete_webhook(&self, Path(user_id): Path<Uuid>) -> Result<DeleteResponse> {
        info!(?user_id, "Deleting webhook");

        let result = sqlx::query!("DELETE FROM webhook WHERE user_id = $1", user_id)
            .execute(&self.pool)
            .await
            .map_err(InternalServerError)?;

        if result.rows_affected() == 0 {
            return Ok(DeleteResponse::NotFound);
        }

        Ok(DeleteResponse::Ok)
    }
}
