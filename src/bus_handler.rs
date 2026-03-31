// bus_handler.rs — SessionBusHandler: bridges fs-bus session::* events into
// the SessionStore.
//
// Topic patterns handled:
//   session::user::login    → create or resume session
//   session::user::logout   → close session
//   session::app::opened    → register app window in session
//   session::app::closed    → remove app window from session

use std::sync::Arc;

use async_trait::async_trait;
use fs_bus::{
    topics::{SESSION_APP_CLOSED, SESSION_APP_OPENED, SESSION_USER_LOGIN, SESSION_USER_LOGOUT},
    BusError, Event, TopicHandler,
};
use tracing::{info, instrument, warn};

use crate::store::SessionStore;

// ── Payload types ─────────────────────────────────────────────────────────────

/// Payload of `session::user::login`.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UserLoginPayload {
    pub user_id: String,
    pub username: String,
    pub session_id: String,
}

/// Payload of `session::user::logout`.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UserLogoutPayload {
    pub user_id: String,
    pub session_id: String,
}

/// Payload of `session::app::opened`.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AppOpenedPayload {
    pub session_id: String,
    pub app_id: String,
}

/// Payload of `session::app::closed`.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AppClosedPayload {
    pub session_id: String,
    pub app_id: String,
}

// ── SessionBusHandler ─────────────────────────────────────────────────────────

/// Subscribes to `session::#` bus events and keeps the session store in sync.
pub struct SessionBusHandler<S> {
    store: Arc<S>,
}

impl<S: SessionStore> SessionBusHandler<S> {
    /// Wrap `store` in a bus handler.
    #[must_use]
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl<S: SessionStore + Send + Sync + 'static> TopicHandler for SessionBusHandler<S> {
    fn topic_pattern(&self) -> &'static str {
        "session::#"
    }

    #[instrument(
        name = "session.bus_handler",
        skip(self, event),
        fields(topic = event.topic())
    )]
    async fn handle(&self, event: &Event) -> Result<(), BusError> {
        match event.topic() {
            SESSION_USER_LOGIN => {
                let p: UserLoginPayload = match event.parse_payload() {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("session::user::login: bad payload: {e}");
                        return Ok(());
                    }
                };
                match self.store.create(&p.user_id, &p.username).await {
                    Ok(s) => info!("session created: {} for {}", s.id(), p.user_id),
                    Err(e) => warn!("session create failed: {e}"),
                }
            }
            SESSION_USER_LOGOUT => {
                let p: UserLogoutPayload = match event.parse_payload() {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("session::user::logout: bad payload: {e}");
                        return Ok(());
                    }
                };
                match self.store.close(&p.session_id).await {
                    Ok(()) => info!("session closed: {}", p.session_id),
                    Err(e) => warn!("session close failed: {e}"),
                }
            }
            SESSION_APP_OPENED => {
                let p: AppOpenedPayload = match event.parse_payload() {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("session::app::opened: bad payload: {e}");
                        return Ok(());
                    }
                };
                match self.store.open_app(&p.session_id, &p.app_id).await {
                    Ok(()) => info!("app opened: {} in {}", p.app_id, p.session_id),
                    Err(e) => warn!("open_app failed: {e}"),
                }
            }
            SESSION_APP_CLOSED => {
                let p: AppClosedPayload = match event.parse_payload() {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("session::app::closed: bad payload: {e}");
                        return Ok(());
                    }
                };
                match self.store.close_app(&p.session_id, &p.app_id).await {
                    Ok(()) => info!("app closed: {} in {}", p.app_id, p.session_id),
                    Err(e) => warn!("close_app failed: {e}"),
                }
            }
            other => {
                warn!("SessionBusHandler: unhandled topic '{other}'");
            }
        }
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::SqliteSessionStore;
    use fs_bus::{BusMessage, MessageBus};

    async fn store() -> Arc<SqliteSessionStore> {
        Arc::new(SqliteSessionStore::open(":memory:").await.unwrap())
    }

    #[tokio::test]
    async fn topic_pattern_is_session_namespace() {
        let s = store().await;
        let h = SessionBusHandler::new(s);
        assert_eq!(h.topic_pattern(), "session::#");
    }

    #[tokio::test]
    async fn login_event_creates_session() {
        let s = store().await;
        let mut bus = MessageBus::new();
        bus.add_handler(Arc::new(SessionBusHandler::new(Arc::clone(&s))));

        let payload = UserLoginPayload {
            user_id: "user-1".into(),
            username: "Alice".into(),
            session_id: "sess-abc".into(),
        };
        let ev = Event::new(SESSION_USER_LOGIN, "test", payload).unwrap();
        bus.publish(BusMessage::fire(ev)).await;

        let active = s.active_user().await.unwrap();
        assert!(active.is_some());
        assert_eq!(active.unwrap().user_id(), "user-1");
    }

    #[tokio::test]
    async fn app_opened_event_tracked() {
        let s = store().await;
        let sess = s.create("user-2", "Bob").await.unwrap();
        let mut bus = MessageBus::new();
        bus.add_handler(Arc::new(SessionBusHandler::new(Arc::clone(&s))));

        let payload = AppOpenedPayload {
            session_id: sess.id().to_owned(),
            app_id: "fs-store".into(),
        };
        let ev = Event::new(SESSION_APP_OPENED, "test", payload).unwrap();
        bus.publish(BusMessage::fire(ev)).await;

        let session = s.get(sess.id()).await.unwrap();
        assert_eq!(session.apps().len(), 1);
        assert_eq!(session.apps()[0].app_id, "fs-store");
    }

    #[tokio::test]
    async fn app_closed_event_removes_app() {
        let s = store().await;
        let sess = s.create("user-3", "Carol").await.unwrap();
        s.open_app(sess.id(), "fs-store").await.unwrap();
        let mut bus = MessageBus::new();
        bus.add_handler(Arc::new(SessionBusHandler::new(Arc::clone(&s))));

        let payload = AppClosedPayload {
            session_id: sess.id().to_owned(),
            app_id: "fs-store".into(),
        };
        let ev = Event::new(SESSION_APP_CLOSED, "test", payload).unwrap();
        bus.publish(BusMessage::fire(ev)).await;

        let session = s.get(sess.id()).await.unwrap();
        assert!(session.apps().is_empty());
    }
}
