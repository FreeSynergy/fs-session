//! `SessionTracker` — observes window events from the Desktop and updates the session store.

use async_trait::async_trait;

use crate::{error::SessionError, store::SessionStore};

// ── SessionTracker trait ──────────────────────────────────────────────────────

/// Receives window lifecycle events from the Desktop and persists them via a [`SessionStore`].
///
/// The Desktop emits events when windows are opened, minimized, focused, or closed.
/// A `SessionTracker` maps these events to session state changes.
///
/// # Design
///
/// ```text
/// Desktop window event
///     │
///     ▼
/// SessionTracker::on_* method
///     │
///     ▼
/// SessionStore (persistent session state)
/// ```
#[async_trait]
pub trait SessionTracker: Send + Sync {
    /// Called when an application window is first opened.
    async fn on_app_opened(&self, session_id: &str, app_id: &str) -> Result<(), SessionError>;

    /// Called when an application window is minimized to the taskbar.
    async fn on_app_minimized(&self, session_id: &str, app_id: &str) -> Result<(), SessionError>;

    /// Called when an application window receives focus.
    async fn on_app_focused(&self, session_id: &str, app_id: &str) -> Result<(), SessionError>;

    /// Called when an application window is closed by the user.
    async fn on_app_closed(&self, session_id: &str, app_id: &str) -> Result<(), SessionError>;
}

// ── StoreBackedTracker ────────────────────────────────────────────────────────

/// Default `SessionTracker` implementation backed by any [`SessionStore`].
pub struct StoreBackedTracker<S: SessionStore> {
    store: S,
}

impl<S: SessionStore> StoreBackedTracker<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }
}

#[async_trait]
impl<S: SessionStore> SessionTracker for StoreBackedTracker<S> {
    async fn on_app_opened(&self, session_id: &str, app_id: &str) -> Result<(), SessionError> {
        self.store.open_app(session_id, app_id).await
    }

    async fn on_app_minimized(&self, session_id: &str, app_id: &str) -> Result<(), SessionError> {
        self.store.minimize_app(session_id, app_id).await
    }

    async fn on_app_focused(&self, session_id: &str, app_id: &str) -> Result<(), SessionError> {
        self.store.restore_app(session_id, app_id).await
    }

    async fn on_app_closed(&self, session_id: &str, app_id: &str) -> Result<(), SessionError> {
        self.store.close_app(session_id, app_id).await
    }
}
