use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use roadboard_core::{
    project::{
        NewProject, NewProjectMember, Project, ProjectMember, ProjectMemberRepository,
        ProjectRepository, ProjectStatus,
    },
    Error as CoreError, Result,
};

use crate::util::{enum_to_str, map_sqlx_err, parse_enum, parse_ts, req};

pub struct SqliteProjectRepository(pub SqlitePool);
pub struct SqliteProjectMemberRepository(pub SqlitePool);

#[allow(clippy::too_many_arguments)]
fn row_to_project(
    id: Option<String>,
    slug: String,
    name: String,
    description: Option<String>,
    status: String,
    owner_user_id: String,
    created_at: String,
    updated_at: String,
) -> Result<Project> {
    Ok(Project {
        id: req(id, "project.id")?,
        slug,
        name,
        description,
        status: parse_enum(&status)?,
        owner_user_id,
        created_at: parse_ts(&created_at)?,
        updated_at: parse_ts(&updated_at)?,
    })
}

fn row_to_member(
    project_id: String,
    user_id: String,
    role: String,
    granted_by_user_id: String,
    created_at: String,
) -> Result<ProjectMember> {
    Ok(ProjectMember {
        project_id,
        user_id,
        role: parse_enum(&role)?,
        granted_by_user_id,
        created_at: parse_ts(&created_at)?,
    })
}

impl ProjectRepository for SqliteProjectRepository {
    async fn create(&self, new: NewProject) -> Result<Project> {
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();
        let status = enum_to_str(&ProjectStatus::Active)?;

        sqlx::query!(
            "INSERT INTO projects (id, slug, name, description, status, owner_user_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            id, new.slug, new.name, new.description, status, new.owner_user_id, now, now
        )
        .execute(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        self.find_by_id(&id).await
    }

    async fn find_by_id(&self, id: &str) -> Result<Project> {
        let r = sqlx::query!(
            "SELECT id, slug, name, description, status, owner_user_id, created_at, updated_at
             FROM projects WHERE id = ?",
            id
        )
        .fetch_optional(&self.0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| CoreError::NotFound { entity_type: "Project", id: id.to_string() })?;

        row_to_project(r.id, r.slug, r.name, r.description, r.status, r.owner_user_id,
                       r.created_at, r.updated_at)
    }

    async fn list_accessible_to_user(
        &self,
        user_id: &str,
        after_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Project>> {
        let rows = sqlx::query!(
            "SELECT DISTINCT p.id, p.slug, p.name, p.description, p.status, p.owner_user_id,
                    p.created_at, p.updated_at
             FROM projects p
             WHERE (
                 p.owner_user_id = ?
                 OR EXISTS (
                     SELECT 1 FROM project_members pm
                     WHERE pm.project_id = p.id AND pm.user_id = ?
                 )
             )
             AND (? IS NULL OR p.id > ?)
             ORDER BY p.id
             LIMIT ?",
            user_id, user_id, after_id, after_id, limit
        )
        .fetch_all(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        rows.into_iter()
            .map(|r| row_to_project(r.id, r.slug, r.name, r.description, r.status,
                                     r.owner_user_id, r.created_at, r.updated_at))
            .collect()
    }
}

impl ProjectMemberRepository for SqliteProjectMemberRepository {
    async fn add(&self, new: NewProjectMember) -> Result<ProjectMember> {
        let now = Utc::now().to_rfc3339();
        let role_str = enum_to_str(&new.role)?;

        sqlx::query!(
            "INSERT INTO project_members (project_id, user_id, role, granted_by_user_id, created_at)
             VALUES (?, ?, ?, ?, ?)",
            new.project_id, new.user_id, role_str, new.granted_by_user_id, now
        )
        .execute(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        self.find(&new.project_id, &new.user_id).await
    }

    async fn find(&self, project_id: &str, user_id: &str) -> Result<ProjectMember> {
        let r = sqlx::query!(
            "SELECT project_id, user_id, role, granted_by_user_id, created_at
             FROM project_members WHERE project_id = ? AND user_id = ?",
            project_id, user_id
        )
        .fetch_optional(&self.0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| CoreError::NotFound {
            entity_type: "ProjectMember",
            id: format!("({}, {})", project_id, user_id),
        })?;

        row_to_member(r.project_id, r.user_id, r.role, r.granted_by_user_id, r.created_at)
    }

    async fn list_for_project(&self, project_id: &str) -> Result<Vec<ProjectMember>> {
        let rows = sqlx::query!(
            "SELECT project_id, user_id, role, granted_by_user_id, created_at
             FROM project_members WHERE project_id = ? ORDER BY created_at",
            project_id
        )
        .fetch_all(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        rows.into_iter()
            .map(|r| row_to_member(r.project_id, r.user_id, r.role, r.granted_by_user_id,
                                    r.created_at))
            .collect()
    }

    async fn remove(&self, project_id: &str, user_id: &str) -> Result<()> {
        sqlx::query!(
            "DELETE FROM project_members WHERE project_id = ? AND user_id = ?",
            project_id, user_id
        )
        .execute(&self.0)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }
}
