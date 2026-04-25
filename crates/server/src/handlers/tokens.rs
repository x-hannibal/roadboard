use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use roadboard_core::user::{GrantType, McpTokenRepository, NewMcpToken};
use roadboard_storage::SqliteMcpTokenRepository;
use serde::Deserialize;
use serde_json::json;
use tower_sessions::Session;

use crate::error::AppError;
use crate::state::AppState;
use super::session_user_id;

pub async fn list_tokens(
    session: Session,
    State(state): State<AppState>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    let tokens = SqliteMcpTokenRepository(state.pool.clone())
        .list_for_user(&user_id)
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({ "tokens": tokens })).into_response())
}

#[derive(Deserialize)]
pub struct CreateTokenBody {
    pub token_name: String,
    pub scopes: Vec<GrantType>,
    pub expires_at: Option<DateTime<Utc>>,
}

pub async fn create_token(
    session: Session,
    State(state): State<AppState>,
    Json(body): Json<CreateTokenBody>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    let created = SqliteMcpTokenRepository(state.pool.clone())
        .create(NewMcpToken {
            user_id,
            token_name: body.token_name,
            scopes: body.scopes,
            expires_at: body.expires_at,
        })
        .await
        .map_err(AppError::from)?;

    let body = json!({
        "token": created.token,
        "plaintext_token": created.plaintext_token
    });
    Ok((StatusCode::CREATED, Json(body)).into_response())
}

pub async fn revoke_token(
    session: Session,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let user_id = session_user_id(&session).await?;
    let repo = SqliteMcpTokenRepository(state.pool.clone());
    let token = repo.find_by_id(&id).await.map_err(AppError::from)?;
    if token.user_id != user_id {
        return Err(AppError::Forbidden("Access denied".to_string()));
    }
    repo.revoke(&id).await.map_err(AppError::from)?;
    Ok(StatusCode::NO_CONTENT)
}
