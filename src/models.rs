//! Domain models for session management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// ── AppState ──────────────────────────────────────────────────────────────────

/// Window state of an open application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AppState {
    #[default]
    Open,
    Minimized,
    Focused,
}

impl fmt::Display for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => write!(f, "Open"),
            Self::Minimized => write!(f, "Minimized"),
            Self::Focused => write!(f, "Focused"),
        }
    }
}

// ── AppSession ────────────────────────────────────────────────────────────────

/// One open application within a user session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSession {
    /// Application id, e.g. `"fs-store"`.
    pub app_id: String,
    /// Current window state.
    pub state: AppState,
    /// When the application was opened.
    pub opened_at: DateTime<Utc>,
}

impl AppSession {
    pub fn new(app_id: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
            state: AppState::Open,
            opened_at: Utc::now(),
        }
    }

    #[must_use]
    pub fn is_minimized(&self) -> bool {
        self.state == AppState::Minimized
    }
}

// ── Session ───────────────────────────────────────────────────────────────────

/// An active user session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    id: String,
    user_id: String,
    display_name: String,
    started_at: DateTime<Utc>,
    apps: Vec<AppSession>,
}

impl Session {
    pub fn new(user_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.into(),
            display_name: display_name.into(),
            started_at: Utc::now(),
            apps: Vec::new(),
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    #[must_use]
    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    #[must_use]
    pub fn apps(&self) -> &[AppSession] {
        &self.apps
    }

    /// Find an open app by id.
    #[must_use]
    pub fn app(&self, app_id: &str) -> Option<&AppSession> {
        self.apps.iter().find(|a| a.app_id == app_id)
    }

    /// Whether the app is currently open (any state).
    #[must_use]
    pub fn is_open(&self, app_id: &str) -> bool {
        self.app(app_id).is_some()
    }

    // ── Internal mutators (used by SessionStore impls) ────────────────────────

    pub(crate) fn add_app(&mut self, entry: AppSession) {
        self.apps.push(entry);
    }

    pub(crate) fn set_app_state(&mut self, app_id: &str, state: AppState) -> bool {
        match self.apps.iter_mut().find(|a| a.app_id == app_id) {
            Some(a) => {
                a.state = state;
                true
            }
            None => false,
        }
    }

    pub(crate) fn remove_app(&mut self, app_id: &str) -> bool {
        let before = self.apps.len();
        self.apps.retain(|a| a.app_id != app_id);
        self.apps.len() < before
    }
}
