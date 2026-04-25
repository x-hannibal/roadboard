mod helpers;

use roadboard_core::{
    user::{NewUser, UserRepository, UserStatus},
    Error,
};
use roadboard_storage::SqliteUserRepository;

fn new_user(suffix: &str) -> NewUser {
    NewUser {
        username: format!("user_{suffix}"),
        email: format!("user_{suffix}@example.com"),
        display_name: format!("User {suffix}"),
        password_hash: "secret".to_string(),
    }
}

#[tokio::test]
async fn create_and_find_by_id() {
    let pool = helpers::setup_db().await;
    let repo = SqliteUserRepository(pool);
    let user = repo.create(new_user("a")).await.unwrap();
    assert_eq!(user.username, "user_a");
    let found = repo.find_by_id(&user.id).await.unwrap();
    assert_eq!(found.id, user.id);
}

#[tokio::test]
async fn find_by_username() {
    let pool = helpers::setup_db().await;
    let repo = SqliteUserRepository(pool);
    let user = repo.create(new_user("b")).await.unwrap();
    let found = repo.find_by_username("user_b").await.unwrap();
    assert_eq!(found.id, user.id);
}

#[tokio::test]
async fn find_by_email() {
    let pool = helpers::setup_db().await;
    let repo = SqliteUserRepository(pool);
    let user = repo.create(new_user("c")).await.unwrap();
    let found = repo.find_by_email("user_c@example.com").await.unwrap();
    assert_eq!(found.id, user.id);
}

#[tokio::test]
async fn duplicate_username_returns_conflict() {
    let pool = helpers::setup_db().await;
    let repo = SqliteUserRepository(pool);
    repo.create(new_user("d")).await.unwrap();
    let err = repo.create(new_user("d")).await.unwrap_err();
    assert!(matches!(err, Error::Conflict(_)));
}

#[tokio::test]
async fn update_status() {
    let pool = helpers::setup_db().await;
    let repo = SqliteUserRepository(pool);
    let user = repo.create(new_user("e")).await.unwrap();
    let updated = repo.update_status(&user.id, UserStatus::Disabled).await.unwrap();
    assert_eq!(updated.status, UserStatus::Disabled);
}
