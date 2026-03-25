//! Domain models for session management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// ── ProgramState ──────────────────────────────────────────────────────────────

/// Window state of an open program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProgramState {
    #[default]
    Open,
    Minimized,
    Focused,
}

impl fmt::Display for ProgramState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => write!(f, "Open"),
            Self::Minimized => write!(f, "Minimized"),
            Self::Focused => write!(f, "Focused"),
        }
    }
}

// ── ProgramEntry ──────────────────────────────────────────────────────────────

/// One open program within a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramEntry {
    /// Program id, e.g. `"fs-store"`.
    pub program_id: String,
    /// Current window state.
    pub state: ProgramState,
    /// When the program was opened.
    pub opened_at: DateTime<Utc>,
}

impl ProgramEntry {
    pub fn new(program_id: impl Into<String>) -> Self {
        Self {
            program_id: program_id.into(),
            state: ProgramState::Open,
            opened_at: Utc::now(),
        }
    }

    pub fn is_minimized(&self) -> bool {
        self.state == ProgramState::Minimized
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
    programs: Vec<ProgramEntry>,
}

impl Session {
    pub fn new(user_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.into(),
            display_name: display_name.into(),
            started_at: Utc::now(),
            programs: Vec::new(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    pub fn programs(&self) -> &[ProgramEntry] {
        &self.programs
    }

    /// Find an open program by id.
    pub fn program(&self, program_id: &str) -> Option<&ProgramEntry> {
        self.programs.iter().find(|p| p.program_id == program_id)
    }

    /// Whether the program is currently open (any state).
    pub fn is_open(&self, program_id: &str) -> bool {
        self.program(program_id).is_some()
    }

    // ── Internal mutators (used by SessionStore) ──────────────────────────────

    pub(crate) fn add_program(&mut self, entry: ProgramEntry) {
        self.programs.push(entry);
    }

    pub(crate) fn set_program_state(&mut self, program_id: &str, state: ProgramState) -> bool {
        match self
            .programs
            .iter_mut()
            .find(|p| p.program_id == program_id)
        {
            Some(p) => {
                p.state = state;
                true
            }
            None => false,
        }
    }

    pub(crate) fn remove_program(&mut self, program_id: &str) -> bool {
        let before = self.programs.len();
        self.programs.retain(|p| p.program_id != program_id);
        self.programs.len() < before
    }
}
