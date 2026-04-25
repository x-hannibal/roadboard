pub mod auth;
pub mod milestones;
pub mod projects;
pub mod sprints;
pub mod tasks;
pub mod tokens;
pub mod users;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::Deserialize;
use tower_sessions::Session;

use crate::error::AppError;
use crate::state::AppState;
use roadboard_core::project::ProjectMemberRepository;
use roadboard_storage::SqliteProjectMemberRepository;

pub async fn session_user_id(session: &Session) -> Result<String, AppError> {
    session
        .get::<String>("uid")
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?
        .ok_or(AppError::Unauthenticated { reason: "missing_token" })
}

pub async fn require_project_access(
    state: &AppState,
    user_id: &str,
    project_id: &str,
) -> Result<roadboard_core::project::ProjectMember, AppError> {
    SqliteProjectMemberRepository(state.pool.clone())
        .find(project_id, user_id)
        .await
        .map_err(|e| match e {
            roadboard_core::Error::NotFound { .. } => AppError::ProjectAccessDenied {
                project_id: project_id.to_string(),
            },
            other => AppError::from(other),
        })
}

#[derive(Deserialize, Default)]
pub struct PaginationQuery {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

pub fn parse_limit(limit: Option<i64>) -> i64 {
    limit.map(|l| l.clamp(1, 200)).unwrap_or(50)
}

pub fn decode_cursor(cursor: Option<&str>) -> Result<Option<String>, AppError> {
    let c = match cursor {
        None => return Ok(None),
        Some(c) => c,
    };
    let bytes = URL_SAFE_NO_PAD
        .decode(c)
        .map_err(|_| AppError::InvalidInput("invalid cursor".to_string()))?;
    let json: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| AppError::InvalidInput("invalid cursor".to_string()))?;
    let last_id = json["last_id"]
        .as_str()
        .ok_or_else(|| AppError::InvalidInput("invalid cursor".to_string()))?
        .to_string();
    Ok(Some(last_id))
}

pub fn encode_cursor(last_id: &str) -> String {
    let json = serde_json::json!({ "last_id": last_id });
    URL_SAFE_NO_PAD.encode(json.to_string())
}

pub fn paginate<T: serde::Serialize>(
    items: &[T],
    limit: i64,
    get_id: impl Fn(&T) -> &str,
) -> serde_json::Value {
    let next_cursor = if items.len() == limit as usize {
        items.last().map(|item| encode_cursor(get_id(item)))
    } else {
        None
    };
    serde_json::json!({ "items": items, "next_cursor": next_cursor })
}
