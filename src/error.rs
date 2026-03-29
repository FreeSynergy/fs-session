//! Error type for session operations.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("session not found: {id}")]
    NotFound { id: String },

    #[error("app not open in session {session_id}: {app_id}")]
    AppNotOpen { session_id: String, app_id: String },

    #[error("serialisation error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("failed to parse timestamp: {0}")]
    Parse(String),
}
