use std::time::Duration;

use poem::{error::InternalServerError, http::StatusCode, Error, Result};
use poem_openapi::{param::Path, payload::Json, ApiResponse, Object, OpenApi};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{jwt::JWTAuth, models::Community};

// ====================

async fn get_community(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Community>> {
    sqlx::query_as!(Community, "SELECT * FROM community WHERE id = $1", id)
        .fetch_optional(pool)
        .await
}

use thiserror::Error;
#[derive(Error, Debug)]
enum CheckCommunityError {
    #[error("Community not found")]
    NotFound,

    #[error("Not the owner of the community")]
    NotOwner,

    #[error("Database error")]
    Sqlx(#[from] sqlx::Error),
}

async fn check_community(
    pool: &PgPool,
    community_id: Uuid,
    owner_id: Option<Uuid>,
) -> core::result::Result<(), CheckCommunityError> {
    let community = get_community(pool, community_id)
        .await?
        .ok_or(CheckCommunityError::NotFound)?;

    if owner_id.is_some_and(|oid| community.owner_id != oid) {
        return Err(CheckCommunityError::NotOwner);
    }

    Ok(())
}

// ====================

/// Represents a code that when used, join the user to a community
#[derive(Object)]
struct InviteCode {
    id: Uuid,
    expired_at: Option<OffsetDateTime>,
}

/// Configuration when creating invite code
#[derive(Object)]
struct CreateInvite {
    /// Duration (in seconds) before the invite code expires
    expired_in: Option<u64>,
}

// ====================

#[derive(ApiResponse)]
enum CreateResponse {
    /// Created invite code
    #[oai(status = 201)]
    Created(Json<InviteCode>),
    /// Community not found
    #[oai(status = 404)]
    CommunityNotFound,
    /// Not the owner of the community
    #[oai(status = 403)]
    NotOwner,
}

#[derive(ApiResponse)]
enum ListReponse {
    /// List of invite codes
    #[oai(status = 200)]
    Ok(Json<Vec<InviteCode>>),
    /// Community not found
    #[oai(status = 404)]
    CommunityNotFound,
    /// Not the owner of the community
    #[oai(status = 403)]
    NotOwner,
}

#[derive(ApiResponse)]
enum DeleteResponse {
    /// Deleted invite code
    #[oai(status = 200)]
    Ok,
    /// Invite code or community not found
    #[oai(status = 404)]
    NotFound,
    /// Not the owner of the community
    #[oai(status = 403)]
    NotOwner,
}

pub struct Api {
    pub pool: PgPool,
}

#[OpenApi(tag = "super::Tags::Invite")]
impl Api {
    /// Join community with invite code
    #[oai(path = "/community/invite/:code", method = "post")]
    async fn join_community_with_code(
        &self,
        Path(code): Path<Uuid>,
        JWTAuth(claim): JWTAuth,
    ) -> Result<Json<Community>> {
        let user_id = claim.sub;

        // Clean up expired invite codes
        sqlx::query!(
            "
            DELETE FROM invite_code WHERE expired_at < now()
            "
        )
        .execute(&self.pool)
        .await
        .map_err(InternalServerError)?;

        // Join user to community, return joined community
        let community = sqlx::query_as!(
            Community,
            "
            WITH inserted AS (
                INSERT INTO user_join_community (account_id, community_id)
                SELECT 
                    $1 AS account_id, 
                    community_id
                FROM invite_code i
                WHERE i.id = $2
                RETURNING community_id 
            )
            SELECT c.* FROM inserted JOIN community c ON c.id = inserted.community_id;
            ",
            user_id,
            code
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(InternalServerError)?;

        let community = community.ok_or(Error::from_string(
            "The invite code is invalid or expired",
            StatusCode::NOT_FOUND,
        ))?;

        Ok(Json(community))
    }

    /// Create invite code
    #[oai(path = "/community/:id/invite", method = "post")]
    async fn create_invite_code(
        &self,
        Path(id): Path<Uuid>,
        JWTAuth(claim): JWTAuth,
        body: Json<CreateInvite>,
    ) -> Result<CreateResponse> {
        match check_community(&self.pool, id, Some(claim.sub)).await {
            Ok(_) => {}
            Err(CheckCommunityError::NotFound) => return Ok(CreateResponse::CommunityNotFound),
            Err(CheckCommunityError::NotOwner) => return Ok(CreateResponse::NotOwner),
            Err(CheckCommunityError::Sqlx(e)) => return Err(InternalServerError(e)),
        }

        let expired_at = body
            .expired_in
            .map(|expired_in| OffsetDateTime::now_utc() + Duration::from_secs(expired_in));

        let invite_code = sqlx::query_as!(
            InviteCode,
            "
            INSERT INTO invite_code (community_id, expired_at)
            VALUES ($1, $2)
            RETURNING id, expired_at
            ",
            id,
            expired_at
        )
        .fetch_one(&self.pool)
        .await
        .map_err(InternalServerError)?;

        Ok(CreateResponse::Created(Json(invite_code)))
    }

    /// List invite codes
    #[oai(path = "/community/:id/invite", method = "get")]
    async fn list_invite_codes(
        &self,
        Path(id): Path<Uuid>,
        JWTAuth(claim): JWTAuth,
    ) -> Result<ListReponse> {
        match check_community(&self.pool, id, Some(claim.sub)).await {
            Ok(_) => {}
            Err(CheckCommunityError::NotFound) => return Ok(ListReponse::CommunityNotFound),
            Err(CheckCommunityError::NotOwner) => return Ok(ListReponse::NotOwner),
            Err(CheckCommunityError::Sqlx(e)) => return Err(InternalServerError(e)),
        }

        let invite_codes = sqlx::query_as!(
            InviteCode,
            "
            SELECT id, expired_at
            FROM invite_code
            WHERE community_id = $1
            ",
            id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(InternalServerError)?;

        Ok(ListReponse::Ok(Json(invite_codes)))
    }

    /// Delete invite code
    #[oai(path = "/community/:id/invite/:code", method = "delete")]
    async fn delete_invite_code(
        &self,
        Path(id): Path<Uuid>,
        Path(code): Path<Uuid>,
        JWTAuth(claim): JWTAuth,
    ) -> Result<DeleteResponse> {
        match check_community(&self.pool, id, Some(claim.sub)).await {
            Ok(_) => {}
            Err(CheckCommunityError::NotFound) => return Ok(DeleteResponse::NotFound),
            Err(CheckCommunityError::NotOwner) => return Ok(DeleteResponse::NotOwner),
            Err(CheckCommunityError::Sqlx(e)) => return Err(InternalServerError(e)),
        }

        let result = sqlx::query!(
            "
            DELETE FROM invite_code
            WHERE id = $1
            ",
            code
        )
        .execute(&self.pool)
        .await
        .map_err(InternalServerError)?;

        if result.rows_affected() == 0 {
            return Ok(DeleteResponse::NotFound);
        }

        Ok(DeleteResponse::Ok)
    }
}
