mod helpers;

use roadboard_core::{
    user::{GrantType, McpTokenRepository, NewMcpToken, NewUser, TokenStatus, UserRepository},
};
use roadboard_storage::{SqliteMcpTokenRepository, SqliteUserRepository};

async fn make_user(pool: &sqlx::SqlitePool) -> String {
    let repo = SqliteUserRepository(pool.clone());
    let u = repo
        .create(NewUser {
            username: "tokenuser".to_string(),
            email: "tokenuser@example.com".to_string(),
            display_name: "Token User".to_string(),
            password_hash: "x".to_string(),
        })
        .await
        .unwrap();
    u.id
}

#[tokio::test]
async fn create_returns_plaintext_token() {
    let pool = helpers::setup_db().await;
    let user_id = make_user(&pool).await;
    let repo = SqliteMcpTokenRepository(pool);
    let created = repo
        .create(NewMcpToken {
            user_id: user_id.clone(),
            token_name: "my-token".to_string(),
            scopes: vec![GrantType::ProjectRead],
            expires_at: None,
        })
        .await
        .unwrap();
    assert!(!created.plaintext_token.is_empty());
    assert_eq!(created.token.status, TokenStatus::Active);
}

#[tokio::test]
async fn find_by_hash_returns_token() {
    let pool = helpers::setup_db().await;
    let user_id = make_user(&pool).await;
    let repo = SqliteMcpTokenRepository(pool);
    let created = repo
        .create(NewMcpToken {
            user_id,
            token_name: "findme".to_string(),
            scopes: vec![GrantType::TaskWrite],
            expires_at: None,
        })
        .await
        .unwrap();
    let found = repo.find_by_hash(&created.plaintext_token).await.unwrap();
    assert_eq!(found.id, created.token.id);
}

#[tokio::test]
async fn revoke_changes_status() {
    let pool = helpers::setup_db().await;
    let user_id = make_user(&pool).await;
    let repo = SqliteMcpTokenRepository(pool);
    let created = repo
        .create(NewMcpToken {
            user_id,
            token_name: "revokeme".to_string(),
            scopes: vec![],
            expires_at: None,
        })
        .await
        .unwrap();
    let revoked = repo.revoke(&created.token.id).await.unwrap();
    assert_eq!(revoked.status, TokenStatus::Revoked);
    assert!(revoked.revoked_at.is_some());
}

#[tokio::test]
async fn revoked_token_still_findable() {
    let pool = helpers::setup_db().await;
    let user_id = make_user(&pool).await;
    let repo = SqliteMcpTokenRepository(pool);
    let created = repo
        .create(NewMcpToken {
            user_id,
            token_name: "stillhere".to_string(),
            scopes: vec![],
            expires_at: None,
        })
        .await
        .unwrap();
    repo.revoke(&created.token.id).await.unwrap();
    let found = repo.find_by_id(&created.token.id).await.unwrap();
    assert_eq!(found.status, TokenStatus::Revoked);
}
