//! Integration tests for fs-session.

use fs_session::{ProgramState, SessionStore};

async fn store() -> SessionStore {
    SessionStore::open(":memory:").await.expect("open failed")
}

#[tokio::test]
async fn create_and_retrieve_session() {
    let store = store().await;
    let session = store.create("user-1", "Alice").await.unwrap();

    let loaded = store.get(session.id()).await.unwrap();
    assert_eq!(loaded.user_id(), "user-1");
    assert_eq!(loaded.display_name(), "Alice");
    assert!(loaded.programs().is_empty());
}

#[tokio::test]
async fn open_program_appears_in_session() {
    let store = store().await;
    let session = store.create("user-1", "Alice").await.unwrap();

    store.open_program(session.id(), "fs-store").await.unwrap();

    let loaded = store.get(session.id()).await.unwrap();
    assert!(loaded.is_open("fs-store"));
    assert_eq!(loaded.programs().len(), 1);
}

#[tokio::test]
async fn open_program_twice_is_idempotent() {
    let store = store().await;
    let session = store.create("user-1", "Alice").await.unwrap();

    store.open_program(session.id(), "fs-store").await.unwrap();
    store.open_program(session.id(), "fs-store").await.unwrap();

    let loaded = store.get(session.id()).await.unwrap();
    assert_eq!(
        loaded.programs().len(),
        1,
        "should not create duplicate entries"
    );
}

#[tokio::test]
async fn minimize_and_restore_program() {
    let store = store().await;
    let session = store.create("user-1", "Alice").await.unwrap();
    store.open_program(session.id(), "fs-store").await.unwrap();

    // Minimize
    store
        .minimize_program(session.id(), "fs-store")
        .await
        .unwrap();
    let loaded = store.get(session.id()).await.unwrap();
    assert_eq!(
        loaded.program("fs-store").unwrap().state,
        ProgramState::Minimized
    );

    // Restore — must not create a new instance
    store
        .restore_program(session.id(), "fs-store")
        .await
        .unwrap();
    let loaded = store.get(session.id()).await.unwrap();
    assert_eq!(
        loaded.program("fs-store").unwrap().state,
        ProgramState::Open
    );
    assert_eq!(
        loaded.programs().len(),
        1,
        "restore must not duplicate the entry"
    );
}

#[tokio::test]
async fn close_program_removes_entry() {
    let store = store().await;
    let session = store.create("user-1", "Alice").await.unwrap();
    store.open_program(session.id(), "fs-store").await.unwrap();

    store.close_program(session.id(), "fs-store").await.unwrap();

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
