use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use chrono::Utc;
use rand::{thread_rng, Rng};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use uuid::Uuid;

use roadboard_core::{
    user::{
        CreatedMcpToken, GrantType, McpToken, McpTokenRepository, NewMcpToken, NewUser, User,
        UserRepository, UserStatus,
    },
    Error as CoreError, Result,
};

use crate::util::{enum_to_str, map_sqlx_err, parse_enum, parse_ts, parse_ts_opt, req};

pub struct SqliteUserRepository(pub SqlitePool);
pub struct SqliteMcpTokenRepository(pub SqlitePool);

#[allow(clippy::too_many_arguments)]
fn row_to_user(
    id: Option<String>,
    username: String,
    email: String,
    display_name: String,
    password_hash: String,
    status: String,
    created_at: String,
    updated_at: String,
) -> Result<User> {
    Ok(User {
        id: req(id, "user.id")?,
        username,
        email,
        display_name,
        password_hash,
        status: parse_enum(&status)?,
        created_at: parse_ts(&created_at)?,
        updated_at: parse_ts(&updated_at)?,
    })
}

#[allow(clippy::too_many_arguments)]
fn row_to_token(
    id: Option<String>,
    user_id: String,
    token_name: String,
    token_hash: String,
    scopes: String,
    status: String,
    expires_at: Option<String>,
    last_used_at: Option<String>,
    revoked_at: Option<String>,
    created_at: String,
) -> Result<McpToken> {
    let scopes: Vec<GrantType> = serde_json::from_str(&scopes)
        .map_err(|e| CoreError::Storage(format!("invalid scopes JSON: {}", e)))?;
    Ok(McpToken {
        id: req(id, "mcp_token.id")?,
        user_id,
        token_name,
        token_hash,
        scopes,
        status: parse_enum(&status)?,
        expires_at: parse_ts_opt(expires_at)?,
        last_used_at: parse_ts_opt(last_used_at)?,
        revoked_at: parse_ts_opt(revoked_at)?,
        created_at: parse_ts(&created_at)?,
    })
}

impl UserRepository for SqliteUserRepository {
    async fn create(&self, new: NewUser) -> Result<User> {
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();

        let salt = SaltString::generate(&mut thread_rng());
        let hash = Argon2::default()
            .hash_password(new.password_hash.as_bytes(), &salt)
            .map_err(|e| CoreError::Storage(e.to_string()))?
            .to_string();

        sqlx::query!(
            "INSERT INTO users (id, username, email, display_name, password_hash, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, 'active', ?, ?)",
            id, new.username, new.email, new.display_name, hash, now, now
        )
        .execute(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        self.find_by_id(&id).await
    }

    async fn find_by_id(&self, id: &str) -> Result<User> {
        let r = sqlx::query!(
            "SELECT id, username, email, display_name, password_hash, status, created_at, updated_at
             FROM users WHERE id = ?",
            id
        )
        .fetch_optional(&self.0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| CoreError::NotFound { entity_type: "User", id: id.to_string() })?;

        row_to_user(r.id, r.username, r.email, r.display_name, r.password_hash,
                    r.status, r.created_at, r.updated_at)
    }

    async fn find_by_username(&self, username: &str) -> Result<User> {
        let r = sqlx::query!(
            "SELECT id, username, email, display_name, password_hash, status, created_at, updated_at
             FROM users WHERE username = ?",
            username
        )
        .fetch_optional(&self.0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| CoreError::NotFound { entity_type: "User", id: username.to_string() })?;

        row_to_user(r.id, r.username, r.email, r.display_name, r.password_hash,
                    r.status, r.created_at, r.updated_at)
    }

    async fn find_by_email(&self, email: &str) -> Result<User> {
        let r = sqlx::query!(
            "SELECT id, username, email, display_name, password_hash, status, created_at, updated_at
             FROM users WHERE email = ?",
            email
        )
        .fetch_optional(&self.0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| CoreError::NotFound { entity_type: "User", id: email.to_string() })?;

        row_to_user(r.id, r.username, r.email, r.display_name, r.password_hash,
                    r.status, r.created_at, r.updated_at)
    }

    async fn update_status(&self, id: &str, status: UserStatus) -> Result<User> {
        let now = Utc::now().to_rfc3339();
        let status_str = enum_to_str(&status)?;

        sqlx::query!(
            "UPDATE users SET status = ?, updated_at = ? WHERE id = ?",
            status_str, now, id
        )
        .execute(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        self.find_by_id(id).await
    }
}

impl McpTokenRepository for SqliteMcpTokenRepository {
    async fn create(&self, new: NewMcpToken) -> Result<CreatedMcpToken> {
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();

        // Generate token before any .await — ThreadRng is !Send and must not cross await points
        let (plaintext_token, token_hash) = {
            let mut rng = thread_rng();
            let plaintext_bytes: [u8; 32] = rng.gen();
            let pt = hex::encode(plaintext_bytes);
            let h = hex::encode(Sha256::digest(pt.as_bytes()));
            (pt, h)
        };

        let scopes_json = serde_json::to_string(&new.scopes)
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        let expires_at = new.expires_at.map(|dt| dt.to_rfc3339());

        sqlx::query!(
            "INSERT INTO mcp_tokens (id, user_id, token_name, token_hash, scopes, status, expires_at, created_at)
             VALUES (?, ?, ?, ?, ?, 'active', ?, ?)",
            id, new.user_id, new.token_name, token_hash, scopes_json, expires_at, now
        )
        .execute(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        let token = self.find_by_id(&id).await?;
        Ok(CreatedMcpToken { token, plaintext_token })
    }

    async fn find_by_id(&self, id: &str) -> Result<McpToken> {
        let r = sqlx::query!(
            "SELECT id, user_id, token_name, token_hash, scopes, status,
                    expires_at, last_used_at, revoked_at, created_at
             FROM mcp_tokens WHERE id = ?",
            id
        )
        .fetch_optional(&self.0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| CoreError::NotFound { entity_type: "McpToken", id: id.to_string() })?;

        row_to_token(r.id, r.user_id, r.token_name, r.token_hash, r.scopes, r.status,
                     r.expires_at, r.last_used_at, r.revoked_at, r.created_at)
    }

    async fn find_by_hash(&self, plaintext_token: &str) -> Result<McpToken> {
        let hash = hex::encode(Sha256::digest(plaintext_token.as_bytes()));
        let r = sqlx::query!(
            "SELECT id, user_id, token_name, token_hash, scopes, status,
                    expires_at, last_used_at, revoked_at, created_at
             FROM mcp_tokens WHERE token_hash = ?",
            hash
        )
        .fetch_optional(&self.0)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| CoreError::NotFound {
            entity_type: "McpToken",
            id: "by_hash".to_string(),
        })?;

        row_to_token(r.id, r.user_id, r.token_name, r.token_hash, r.scopes, r.status,
                     r.expires_at, r.last_used_at, r.revoked_at, r.created_at)
    }

    async fn list_for_user(&self, user_id: &str) -> Result<Vec<McpToken>> {
        let rows = sqlx::query!(
            "SELECT id, user_id, token_name, token_hash, scopes, status,
                    expires_at, last_used_at, revoked_at, created_at
             FROM mcp_tokens WHERE user_id = ? ORDER BY created_at",
            user_id
        )
        .fetch_all(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        rows.into_iter()
            .map(|r| row_to_token(r.id, r.user_id, r.token_name, r.token_hash, r.scopes,
                                   r.status, r.expires_at, r.last_used_at, r.revoked_at,
                                   r.created_at))
            .collect()
    }

    async fn revoke(&self, id: &str) -> Result<McpToken> {
        let now = Utc::now().to_rfc3339();
        sqlx::query!(
            "UPDATE mcp_tokens SET status = 'revoked', revoked_at = ? WHERE id = ?",
            now, id
        )
        .execute(&self.0)
        .await
        .map_err(map_sqlx_err)?;

        self.find_by_id(id).await
    }

    async fn touch_last_used(&self, id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query!(
            "UPDATE mcp_tokens SET last_used_at = ? WHERE id = ?",
            now, id
        )
        .execute(&self.0)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }
}
