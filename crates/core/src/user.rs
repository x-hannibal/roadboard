use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    Active,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub password_hash: String,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct NewUser {
    pub username: String,
    pub email: String,
    pub display_name: String,
    /// Plaintext password — storage layer hashes with argon2 before persisting.
    pub password_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GrantType {
    #[serde(rename = "project.read")]
    ProjectRead,
    #[serde(rename = "project.write")]
    ProjectWrite,
    #[serde(rename = "task.write")]
    TaskWrite,
    #[serde(rename = "memory.write")]
    MemoryWrite,
    #[serde(rename = "decision.write")]
    DecisionWrite,
    #[serde(rename = "codeflow.read")]
    CodeflowRead,
    #[serde(rename = "codeflow.write")]
    CodeflowWrite,
    #[serde(rename = "project.admin")]
    ProjectAdmin,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenStatus {
    Active,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToken {
    pub id: String,
    pub user_id: String,
    pub token_name: String,
    pub token_hash: String,
    pub scopes: Vec<GrantType>,
    pub status: TokenStatus,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub struct NewMcpToken {
    pub user_id: String,
    pub token_name: String,
    pub scopes: Vec<GrantType>,
    pub expires_at: Option<DateTime<Utc>>,
}

pub struct CreatedMcpToken {
    pub token: McpToken,
    pub plaintext_token: String,
}

pub trait UserRepository {
    async fn create(&self, new: NewUser) -> Result<User>;
    async fn find_by_id(&self, id: &str) -> Result<User>;
    async fn find_by_username(&self, username: &str) -> Result<User>;
    async fn find_by_email(&self, email: &str) -> Result<User>;
    async fn update_status(&self, id: &str, status: UserStatus) -> Result<User>;
}

pub trait McpTokenRepository {
    async fn create(&self, new: NewMcpToken) -> Result<CreatedMcpToken>;
    async fn find_by_id(&self, id: &str) -> Result<McpToken>;
    async fn find_by_hash(&self, plaintext_token: &str) -> Result<McpToken>;
    async fn list_for_user(&self, user_id: &str) -> Result<Vec<McpToken>>;
    async fn revoke(&self, id: &str) -> Result<McpToken>;
    async fn touch_last_used(&self, id: &str) -> Result<()>;
}
