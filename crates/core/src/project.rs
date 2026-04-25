use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Active,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectRole {
    Viewer,
    Contributor,
    Admin,
    Owner,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub owner_user_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct NewProject {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub owner_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMember {
    pub project_id: String,
    pub user_id: String,
    pub role: ProjectRole,
    pub granted_by_user_id: String,
    pub created_at: DateTime<Utc>,
}

pub struct NewProjectMember {
    pub project_id: String,
    pub user_id: String,
    pub role: ProjectRole,
    pub granted_by_user_id: String,
}

pub trait ProjectRepository {
    async fn create(&self, new: NewProject) -> Result<Project>;
    async fn find_by_id(&self, id: &str) -> Result<Project>;
    async fn list_accessible_to_user(
        &self,
        user_id: &str,
        after_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Project>>;
}

pub trait ProjectMemberRepository {
    async fn add(&self, new: NewProjectMember) -> Result<ProjectMember>;
    async fn find(&self, project_id: &str, user_id: &str) -> Result<ProjectMember>;
    async fn list_for_project(&self, project_id: &str) -> Result<Vec<ProjectMember>>;
    async fn remove(&self, project_id: &str, user_id: &str) -> Result<()>;
}
