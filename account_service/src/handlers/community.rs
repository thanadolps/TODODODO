use poem::{error::InternalServerError, http::StatusCode, Error, Result};
use poem_openapi::{param::Path, payload::Json, Object, OpenApi};
use sqlx::PgPool;
use uuid::Uuid;

use crate::jwt::JWTAuth;

#[derive(Object)]
struct Community {
    #[oai(read_only)]
    id: Uuid,
    name: String,
    description: Option<String>,
    is_private: bool,
    #[oai(read_only)]
    owner_id: Uuid,
}

pub struct Api {
    pub pool: PgPool,
}

#[OpenApi(tag = "super::Tags::Community")]
impl Api {
    /// Search communities
    #[oai(path = "/community", method = "get")]
    async fn search_community(&self) -> Result<Json<Vec<Community>>> {
        // let owner_id = token.0.sub;

        let communities = sqlx::query_as!(
            Community,
            "SELECT * FROM community WHERE is_private = false",
            // owner_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(InternalServerError)?;

        Ok(Json(communities))
    }

    /// Create community
    #[oai(path = "/community", method = "post")]
    async fn create_community(
        &self,
        Json(community): Json<Community>,
        JWTAuth(claims): JWTAuth,
    ) -> Result<Json<Community>> {
        let owner_id = claims.sub;

        // TODO: consider if should also join community?
        let community = sqlx::query_as!(
            Community,
            "INSERT INTO community (name, description, is_private, owner_id) VALUES ($1, $2, $3, $4) RETURNING *",
            community.name,
            community.description,
            community.is_private,
            owner_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(InternalServerError)?;

        Ok(Json(community))
    }

    /// List joined communities
    #[oai(path = "/community/joined", method = "get")]
    async fn joined_community(&self, JWTAuth(claim): JWTAuth) -> Result<Json<Vec<Community>>> {
        let user_id = claim.sub;

        let communities = sqlx::query_as!(
            Community,
            "
            select c.* from user_join_community ujc
            join community c on c.id=ujc.community_id 
            where ujc.account_id = $1
            ",
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(InternalServerError)?;

        Ok(Json(communities))
    }

    /// List owned communities
    #[oai(path = "/community/owned", method = "get")]
    async fn owned_community(&self, JWTAuth(claim): JWTAuth) -> Result<Json<Vec<Community>>> {
        let user_id = claim.sub;

        let communities = sqlx::query_as!(
            Community,
            "select * from community where owner_id = $1",
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(InternalServerError)?;

        Ok(Json(communities))
    }

    /// Join community
    #[oai(path = "/community/:id/member", method = "post")]
    async fn join_community(&self, Path(id): Path<Uuid>, JWTAuth(claim): JWTAuth) -> Result<()> {
        sqlx::query!(
            "
            INSERT INTO user_join_community (account_id, community_id) VALUES ($1, $2)
            ",
            claim.sub,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(InternalServerError)?;

        Ok(())
    }

    /// Leave community
    #[oai(path = "/community/:id/member", method = "delete")]
    async fn leave_community(&self, Path(id): Path<Uuid>, JWTAuth(claim): JWTAuth) -> Result<()> {
        let result = sqlx::query!(
            "
            DELETE FROM user_join_community WHERE account_id = $1 AND community_id = $2
            ",
            claim.sub,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(InternalServerError)?;

        if result.rows_affected() == 0 {
            return Err(Error::from_string(
                "You are not a member of the given community",
                StatusCode::FORBIDDEN,
            ));
        }

        Ok(())
    }
}
