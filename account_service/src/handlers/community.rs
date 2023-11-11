use poem::{error::InternalServerError, http::StatusCode, Error, Result};
use poem_openapi::{param::Path, payload::Json, OpenApi};
use sqlx::PgPool;
use tracing::error;
use uuid::Uuid;

use crate::jwt::JWTAuth;
use crate::models::Community;

pub struct Api {
    pub pool: PgPool,
}

#[OpenApi(tag = "super::Tags::Community")]
impl Api {
    /// Search public communities
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

    /// Delete community
    #[oai(path = "/community/:id", method = "delete")]
    async fn delete_community(&self, Path(id): Path<Uuid>, JWTAuth(claims): JWTAuth) -> Result<()> {
        let user_id = claims.sub;

        let community = get_community(&self.pool, id)
            .await
            .map_err(InternalServerError)?
            .ok_or(Error::from_string(
                "The community does not exist",
                StatusCode::NOT_FOUND,
            ))?;

        if community.owner_id != user_id {
            return Err(Error::from_string(
                "You are not the owner of the community",
                StatusCode::FORBIDDEN,
            ));
        }

        let result = sqlx::query!("DELETE FROM community WHERE id = $1", id)
            .execute(&self.pool)
            .await
            .map_err(InternalServerError)?;

        if result.rows_affected() == 0 {
            error!(
                community_id = ?id,
                requester_user_id = ?user_id,
                owner_id = ?community.owner_id,
                "Failed to delete community, community should exist but no row affected"
            );
            return Err(Error::from_status(StatusCode::INTERNAL_SERVER_ERROR));
        }

        Ok(())
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

    /// Join public community
    ///
    /// Join user to a community. If the community is private, the user must be the owner.
    /// (use invite code to join others private community)
    #[oai(path = "/community/:id/member", method = "post")]
    async fn join_community(&self, Path(id): Path<Uuid>, JWTAuth(claim): JWTAuth) -> Result<()> {
        let user_id = claim.sub;

        let community = get_community(&self.pool, id)
            .await
            .map_err(InternalServerError)?;

        match community {
            None => {
                return Err(Error::from_string(
                    "The community does not exist",
                    StatusCode::NOT_FOUND,
                ))
            }
            // reject if community is private and user is not owner
            Some(Community {
                is_private: true,
                owner_id,
                ..
            }) if owner_id != user_id => {
                return Err(Error::from_string(
                    "The community is private (invite-only)",
                    StatusCode::FORBIDDEN,
                ))
            }
            _ => {}
        }

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

async fn get_community(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Community>> {
    sqlx::query_as!(Community, "SELECT * FROM community WHERE id = $1", id)
        .fetch_optional(pool)
        .await
}
