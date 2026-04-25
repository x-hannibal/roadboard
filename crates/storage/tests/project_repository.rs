mod helpers;

use roadboard_core::{
    project::{NewProject, NewProjectMember, ProjectMemberRepository, ProjectRepository, ProjectRole},
    user::{NewUser, UserRepository},
};
use roadboard_storage::{SqliteProjectMemberRepository, SqliteProjectRepository, SqliteUserRepository};

async fn make_user(pool: &sqlx::SqlitePool, suffix: &str) -> String {
    let repo = SqliteUserRepository(pool.clone());
    repo.create(NewUser {
        username: format!("u_{suffix}"),
        email: format!("u_{suffix}@test.com"),
        display_name: suffix.to_string(),
        password_hash: "x".to_string(),
    })
    .await
    .unwrap()
    .id
}

#[tokio::test]
async fn create_and_find_project() {
    let pool = helpers::setup_db().await;
    let owner = make_user(&pool, "owner1").await;
    let repo = SqliteProjectRepository(pool);
    let proj = repo
        .create(NewProject {
            slug: "test-proj".to_string(),
            name: "Test".to_string(),
            description: None,
            owner_user_id: owner.clone(),
        })
        .await
        .unwrap();
    let found = repo.find_by_id(&proj.id).await.unwrap();
    assert_eq!(found.id, proj.id);
    assert_eq!(found.owner_user_id, owner);
}

#[tokio::test]
async fn list_accessible_includes_member_not_outsider() {
    let pool = helpers::setup_db().await;
    let owner = make_user(&pool, "owner2").await;
    let member = make_user(&pool, "member2").await;
    let outsider = make_user(&pool, "outsider2").await;

    let proj_repo = SqliteProjectRepository(pool.clone());
    let mem_repo = SqliteProjectMemberRepository(pool.clone());

    let proj = proj_repo
        .create(NewProject {
            slug: "proj2".to_string(),
            name: "Proj2".to_string(),
            description: None,
            owner_user_id: owner.clone(),
        })
        .await
        .unwrap();

    mem_repo
        .add(NewProjectMember {
            project_id: proj.id.clone(),
            user_id: member.clone(),
            role: ProjectRole::Contributor,
            granted_by_user_id: owner.clone(),
        })
        .await
        .unwrap();

    let owner_list = proj_repo.list_accessible_to_user(&owner, None, 10).await.unwrap();
    let member_list = proj_repo.list_accessible_to_user(&member, None, 10).await.unwrap();
    let outsider_list = proj_repo.list_accessible_to_user(&outsider, None, 10).await.unwrap();

    assert!(owner_list.iter().any(|p| p.id == proj.id));
    assert!(member_list.iter().any(|p| p.id == proj.id));
    assert!(!outsider_list.iter().any(|p| p.id == proj.id));
}
