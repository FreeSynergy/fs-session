//! Error type for session operations.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("session not found: {id}")]
    NotFound { id: String },

    #[error("program not open in session {session_id}: {program_id}")]
    ProgramNotOpen {
        session_id: String,
        program_id: String,
    },

    #[error("serialisation error: {0}")]
    Json(#[from] serde_json::Error),
}
