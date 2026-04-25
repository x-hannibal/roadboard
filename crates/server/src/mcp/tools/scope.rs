use roadboard_core::{
    project::{ProjectMember, ProjectMemberRepository},
    user::{GrantType, McpToken},
};
use roadboard_storage::SqliteProjectMemberRepository;

use crate::state::AppState;
use super::super::error::ToolError;

pub fn check_scope(token: &McpToken, required: GrantType) -> Result<(), ToolError> {
    if token.scopes.contains(&GrantType::ProjectAdmin) {
        return Ok(());
    }
    if token.scopes.contains(&required) {
        return Ok(());
    }
    let required_str = serde_json::to_value(&required)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    let token_scopes: Vec<String> = token
        .scopes
        .iter()
        .filter_map(|s| serde_json::to_value(s).ok()?.as_str().map(str::to_string))
        .collect();
    Err(ToolError::Forbidden { required_scope: required_str, token_scopes })
}

pub async fn check_project_access(
    state: &AppState,
    token: &McpToken,
    project_id: &str,
    request_id: &str,
) -> Result<ProjectMember, ToolError> {
    SqliteProjectMemberRepository(state.pool.clone())
        .find(project_id, &token.user_id)
        .await
        .map_err(|e| match e {
            roadboard_core::Error::NotFound { .. } => {
                ToolError::ProjectAccessDenied { project_id: project_id.to_string() }
            }
            _ => ToolError::Internal { request_id: request_id.to_string() },
        })
}
