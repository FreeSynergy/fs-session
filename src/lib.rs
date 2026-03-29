//! `fs-session` — user session management for `FreeSynergy`.
//!
//! A **session** is the runtime context of a logged-in user. It tracks:
//!
//! - Which user is currently active (user id + display name)
//! - Which applications are open and in what window state
//! - When the session started
//!
//! This solves the "minimize problem": when an app is minimized, the
//! session knows it is `Minimized` — not gone. Opening it again restores
//! the existing window instead of launching a new instance.
//!
//! # Design
//!
//! - [`SessionStore`] — trait: interface for all session storage backends
//! - [`SqliteSessionStore`] — concrete SQLite-backed implementation
//! - [`SessionTracker`] — trait: maps Desktop window events to session state
//! - [`StoreBackedTracker`] — default tracker backed by any [`SessionStore`]
//!
//! # Example
//!
//! ```no_run
//! use fs_session::{SessionStore, SqliteSessionStore, SessionError};
//!
//! # async fn example() -> Result<(), SessionError> {
//! let store = SqliteSessionStore::open(":memory:").await?;
//! let session = store.create("user-42", "Alice").await?;
//! store.open_app(session.id(), "fs-store").await?;
//! # Ok(())
//! # }
//! ```

#![deny(clippy::all, clippy::pedantic, warnings)]
#![allow(clippy::module_name_repetitions)]

pub mod error;
pub mod models;
pub mod store;
pub mod tracker;

pub use error::SessionError;
pub use models::{AppSession, AppState, Session};
pub use store::{SessionStore, SqliteSessionStore};
pub use tracker::{SessionTracker, StoreBackedTracker};
