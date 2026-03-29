//! Integration tests for fs-session.

use fs_session::{AppState, SessionStore, SqliteSessionStore};

async fn store() -> SqliteSessionStore {
    SqliteSessionStore::open(":memory:")
        .await
        .expect("open failed")
}

#[tokio::test]
async fn create_and_retrieve_session() {
    let store = store().await;
    let session = store.create("user-1", "Alice").await.unwrap();

    let loaded = store.get(session.id()).await.unwrap();
    assert_eq!(loaded.user_id(), "user-1");
    assert_eq!(loaded.display_name(), "Alice");
    assert!(loaded.apps().is_empty());
}

#[tokio::test]
async fn open_app_appears_in_session() {
    let store = store().await;
    let session = store.create("user-1", "Alice").await.unwrap();

    store.open_app(session.id(), "fs-store").await.unwrap();

    let loaded = store.get(session.id()).await.unwrap();
    assert!(loaded.is_open("fs-store"));
    assert_eq!(loaded.apps().len(), 1);
}

#[tokio::test]
async fn open_app_twice_is_idempotent() {
    let store = store().await;
    let session = store.create("user-1", "Alice").await.unwrap();

    store.open_app(session.id(), "fs-store").await.unwrap();
    store.open_app(session.id(), "fs-store").await.unwrap();

    let loaded = store.get(session.id()).await.unwrap();
    assert_eq!(
        loaded.apps().len(),
        1,
        "should not create duplicate entries"
    );
}

#[tokio::test]
async fn minimize_and_restore_app() {
    let store = store().await;
    let session = store.create("user-1", "Alice").await.unwrap();
    store.open_app(session.id(), "fs-store").await.unwrap();

    store.minimize_app(session.id(), "fs-store").await.unwrap();
    let loaded = store.get(session.id()).await.unwrap();
    assert_eq!(loaded.app("fs-store").unwrap().state, AppState::Minimized);

    store.restore_app(session.id(), "fs-store").await.unwrap();
    let loaded = store.get(session.id()).await.unwrap();
    assert_eq!(loaded.app("fs-store").unwrap().state, AppState::Open);
    assert_eq!(
        loaded.apps().len(),
        1,
        "restore must not duplicate the entry"
    );
}

#[tokio::test]
async fn close_app_removes_entry() {
    let store = store().await;
    let session = store.create("user-1", "Alice").await.unwrap();
    store.open_app(session.id(), "fs-store").await.unwrap();

    store.close_app(session.id(), "fs-store").await.unwrap();

    let loaded = store.get(session.id()).await.unwrap();
    assert!(!loaded.is_open("fs-store"));
}

#[tokio::test]
async fn close_session_removes_it() {
    let store = store().await;
    let session = store.create("user-1", "Alice").await.unwrap();
    let id = session.id().to_string();

    store.close(&id).await.unwrap();

    let result = store.get(&id).await;
    assert!(matches!(
        result,
        Err(fs_session::SessionError::NotFound { .. })
    ));
}

#[tokio::test]
async fn get_for_user_returns_session() {
    let store = store().await;
    store.create("user-42", "Bob").await.unwrap();

    let session = store.get_for_user("user-42").await.unwrap();
    assert!(session.is_some());
    assert_eq!(session.unwrap().user_id(), "user-42");
}

#[tokio::test]
async fn list_returns_all_sessions() {
    let store = store().await;
    store.create("user-1", "Alice").await.unwrap();
    store.create("user-2", "Bob").await.unwrap();

    let sessions = store.list().await.unwrap();
    assert_eq!(sessions.len(), 2);
}

#[tokio::test]
async fn active_user_returns_most_recent() {
    let store = store().await;
    store.create("user-1", "Alice").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    store.create("user-2", "Bob").await.unwrap();

    let active = store.active_user().await.unwrap();
    assert!(active.is_some());
    assert_eq!(active.unwrap().user_id(), "user-2");
}
