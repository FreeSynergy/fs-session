//! `fs-session` — user session management for FreeSynergy.
//!
//! A **session** is the runtime context of a logged-in user. It tracks:
//!
//! - Which user is currently active (user id + display name)
//! - Which programs are open and in what window state
//! - When the session started
//!
//! This solves the "minimize problem": when a program is minimized, the
//! session knows it is `Minimized` — not gone. Opening it again restores
//! the existing window instead of launching a new instance.
//!
//! # Example
//!
//! ```no_run
//! use fs_session::{SessionStore, SessionError};
//!
//! # async fn example() -> Result<(), SessionError> {
//! let store = SessionStore::open(":memory:").await?;
//! let session = store.create("user-42", "Alice").await?;
//! store.open_program(session.id(), "fs-store").await?;
//! # Ok(())
//! # }
//! ```

#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod error;
pub mod models;
pub mod store;

pub use error::SessionError;
pub use models::{ProgramEntry, ProgramState, Session};
pub use store::SessionStore;
