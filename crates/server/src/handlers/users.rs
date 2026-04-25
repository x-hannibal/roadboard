use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    Json,
};
use roadboard_core::user::{UserRepository, UserStatus};
use roadboard_storage::SqliteUserRepository;
use serde::Deserialize;
use serde_json::json;
use tower_sessions::Session;

use crate::error::AppError;
use crate::state::AppState;
use super::session_user_id;

pub async fn get_user(
    session: Session,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let caller_id = session_user_id(&session).await?;
    // Only own user or admin (simplified: own user only for now)
    if caller_id != id {
        return Err(AppError::Forbidden("Access denied".to_string()));
    }
    let user = SqliteUserRepository(state.pool.clone())
        .find_by_id(&id)
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({ "user": user })).into_response())
}

#[derive(Deserialize)]
pub struct UpdateStatusBody {
    pub status: UserStatus,
}

pub async fn update_user_status(
    session: Session,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateStatusBody>,
) -> Result<Response, AppError> {
    let caller_id = session_user_id(&session).await?;
    if caller_id != id {
        return Err(AppError::Forbidden("Access denied".to_string()));
    }
    let user = SqliteUserRepository(state.pool.clone())
        .update_status(&id, body.status)
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({ "user": user })).into_response())
}
