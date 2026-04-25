use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::NaiveDate;
use roadboard_core::planning::{MilestoneRepository, MilestoneStatus, NewMilestone};
use roadboard_storage::SqliteMilestoneRepository;
use serde::Deserialize;
use serde_json::json;
use tower_sessions::Session;

use crate::error::AppError;
use crate::state::AppState;
use super::{decode_cursor, paginate, parse_limit, require_project_access, session_user_id};

#[derive(Deserialize)]
pub struct MilestoneListQuery {
    pub status: Option<MilestoneStatus>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

pub async fn list_milestones(
    session: Session,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(q): Query<MilestoneListQuery>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    require_project_access(&state, &user_id, &project_id).await?;
    let limit = parse_limit(q.limit);
    let after = decode_cursor(q.cursor.as_deref())?;

    let ms = SqliteMilestoneRepository(state.pool.clone())
        .list_for_project(&project_id, q.status, after.as_deref(), limit)
        .await
        .map_err(AppError::from)?;

    let page = paginate(&ms, limit, |m| m.id.as_str());
    Ok(Json(page).into_response())
}

#[derive(Deserialize)]
pub struct CreateMilestoneBody {
    pub title: String,
    pub description: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub status: Option<MilestoneStatus>,
    pub order_index: Option<i64>,
}

pub async fn create_milestone(
    session: Session,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(body): Json<CreateMilestoneBody>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    require_project_access(&state, &user_id, &project_id).await?;

    let milestone = SqliteMilestoneRepository(state.pool.clone())
        .create(NewMilestone {
            project_id,
            title: body.title,
            description: body.description,
            due_date: body.due_date,
            status: body.status.unwrap_or(MilestoneStatus::Planned),
            order_index: body.order_index.unwrap_or(0),
            created_by_user_id: user_id.clone(),
            updated_by_user_id: user_id,
        })
        .await
        .map_err(AppError::from)?;

    Ok((StatusCode::CREATED, Json(json!({ "milestone": milestone }))).into_response())
}

pub async fn get_milestone(
    session: Session,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let user_id = session_user_id(&session).await?;
    let ms = SqliteMilestoneRepository(state.pool.clone())
        .find_by_id(&id)
        .await
        .map_err(AppError::from)?;
    require_project_access(&state, &user_id, &ms.project_id).await?;
    Ok(Json(json!({ "milestone": ms })).into_response())
}
