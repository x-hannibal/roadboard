use serde_json::{json, Value};
use std::collections::HashMap;

pub enum ToolError {
    Forbidden { required_scope: String, token_scopes: Vec<String> },
    ProjectAccessDenied { project_id: String },
    NotFound { entity_type: &'static str, entity_id: String },
    ValidationError { fields: HashMap<String, String> },
    InvalidEnum { field: String, allowed: Vec<&'static str>, received: String },
    InvalidReference { field: &'static str, value: String },
    InvalidCursor,
    Conflict { constraint: String, existing_id: Option<String> },
    TaskAlreadyInSprint,
    TaskProjectMismatch,
    Internal { request_id: String },
}

impl ToolError {
    pub fn into_result_value(self) -> Value {
        match self {
            Self::Forbidden { required_scope, token_scopes } => json!({
                "error": {
                    "code": "FORBIDDEN",
                    "message": "Token scope insufficient for this operation",
                    "details": { "required_scope": required_scope, "token_scopes": token_scopes }
                }
            }),
            Self::ProjectAccessDenied { project_id } => json!({
                "error": {
                    "code": "PROJECT_ACCESS_DENIED",
                    "message": "Caller is not a member of this project",
                    "details": { "project_id": project_id }
                }
            }),
            Self::NotFound { entity_type, entity_id } => json!({
                "error": {
                    "code": "NOT_FOUND",
                    "message": "Entity does not exist or is not accessible",
                    "details": { "entity_type": entity_type, "entity_id": entity_id }
                }
            }),
            Self::ValidationError { fields } => json!({
                "error": {
                    "code": "VALIDATION_ERROR",
                    "message": "Input failed validation",
                    "details": { "fields": fields }
                }
            }),
            Self::InvalidEnum { field, allowed, received } => json!({
                "error": {
                    "code": "INVALID_ENUM",
                    "message": "Enum field received an invalid value",
                    "details": { "field": field, "allowed": allowed, "received": received }
                }
            }),
            Self::InvalidReference { field, value } => json!({
                "error": {
                    "code": "INVALID_REFERENCE",
                    "message": "A referenced entity does not exist",
                    "details": { "field": field, "value": value }
                }
            }),
            Self::InvalidCursor => json!({
                "error": { "code": "INVALID_CURSOR", "message": "Pagination cursor is malformed", "details": {} }
            }),
            Self::Conflict { constraint, existing_id } => json!({
                "error": {
                    "code": "CONFLICT",
                    "message": "Operation would violate a uniqueness constraint",
                    "details": { "constraint": constraint, "existing_id": existing_id }
                }
            }),
            Self::TaskAlreadyInSprint => json!({
                "error": {
                    "code": "TASK_ALREADY_IN_SPRINT",
                    "message": "An active link already exists between this sprint and task",
                    "details": {}
                }
            }),
            Self::TaskProjectMismatch => json!({
                "error": {
                    "code": "TASK_PROJECT_MISMATCH",
                    "message": "Task and sprint belong to different projects",
                    "details": {}
                }
            }),
            Self::Internal { request_id } => json!({
                "error": {
                    "code": "INTERNAL_ERROR",
                    "message": "Unexpected server error",
                    "details": { "request_id": request_id }
                }
            }),
        }
    }

    pub fn from_core(e: roadboard_core::Error, request_id: &str) -> Self {
        match e {
            roadboard_core::Error::NotFound { entity_type, id } => {
                Self::NotFound { entity_type, entity_id: id }
            }
            roadboard_core::Error::Conflict(msg) => {
                Self::Conflict { constraint: msg, existing_id: None }
            }
            _ => Self::Internal { request_id: request_id.to_string() },
        }
    }
}
