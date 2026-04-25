use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use roadboard_core::user::UserRepository;
use roadboard_storage::SqliteUserRepository;
use serde::Deserialize;
use serde_json::json;
use tower_sessions::Session;

use crate::error::AppError;
use crate::state::AppState;
use super::session_user_id;

#[derive(Deserialize)]
pub struct LoginBody {
    pub username: String,
    pub password: String,
}

pub async fn login(
    session: Session,
    State(state): State<AppState>,
    Json(body): Json<LoginBody>,
) -> Result<Response, AppError> {
    let repo = SqliteUserRepository(state.pool.clone());
    let user = repo
        .find_by_username(&body.username)
        .await
        .map_err(|_| AppError::Unauthenticated { reason: "invalid_format" })?;

    let hash = PasswordHash::new(&user.password_hash)
        .map_err(|_| AppError::Unauthenticated { reason: "invalid_format" })?;
    Argon2::default()
        .verify_password(body.password.as_bytes(), &hash)
        .map_err(|_| AppError::Unauthenticated { reason: "invalid_format" })?;

    session
        .cycle_id()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;
    session
        .insert("uid", user.id.clone())
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;

    let body = json!({
        "user": {
            "id": user.id,
            "username": user.username,
            "display_name": user.display_name
        }
    });
    Ok(Json(body).into_response())
}

pub async fn logout(session: Session) -> Result<StatusCode, AppError> {
    session
        .flush()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn me(
    session: Session,
    State(state): State<AppState>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    let user = SqliteUserRepository(state.pool.clone())
        .find_by_id(&user_id)
        .await
        .map_err(AppError::from)?;

    let body = json!({
        "user": {
            "id": user.id,
            "username": user.username,
            "email": user.email,
            "display_name": user.display_name,
            "status": user.status
        }
    });
    Ok(Json(body).into_response())
}
