use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    Unauthenticated { reason: &'static str },
    Forbidden(String),
    ProjectAccessDenied { project_id: String },
    NotFound { entity_type: &'static str, entity_id: String },
    Conflict(String),
    InvalidInput(String),
    Internal(anyhow::Error),
}

impl From<roadboard_core::Error> for AppError {
    fn from(e: roadboard_core::Error) -> Self {
        match e {
            roadboard_core::Error::NotFound { entity_type, id } => {
                AppError::NotFound { entity_type, entity_id: id }
            }
            roadboard_core::Error::Conflict(msg) => AppError::Conflict(msg),
            roadboard_core::Error::InvalidInput(msg) => AppError::InvalidInput(msg),
            roadboard_core::Error::Storage(msg) => {
                AppError::Internal(anyhow::anyhow!("{}", msg))
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message, details) = match &self {
            AppError::Unauthenticated { reason } => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHENTICATED",
                "Authentication required".to_string(),
                Some(json!({ "reason": reason })),
            ),
            AppError::Forbidden(msg) => {
                (StatusCode::FORBIDDEN, "FORBIDDEN", msg.clone(), None)
            }
            AppError::ProjectAccessDenied { project_id } => (
                StatusCode::FORBIDDEN,
                "PROJECT_ACCESS_DENIED",
                "Access to project denied".to_string(),
                Some(json!({ "project_id": project_id })),
            ),
            AppError::NotFound { entity_type, entity_id } => (
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                "Entity not found".to_string(),
                Some(json!({ "entity_type": entity_type, "entity_id": entity_id })),
            ),
            AppError::Conflict(msg) => {
                (StatusCode::CONFLICT, "CONFLICT", msg.clone(), None)
            }
            AppError::InvalidInput(msg) => {
                (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg.clone(), None)
            }
            AppError::Internal(e) => {
                tracing::error!("internal error: {e:?}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An internal error occurred".to_string(),
                    None,
                )
            }
        };

        let body = match details {
            Some(d) => json!({ "error": { "code": code, "message": message, "details": d } }),
            None => json!({ "error": { "code": code, "message": message } }),
        };

        (status, Json(body)).into_response()
    }
}
