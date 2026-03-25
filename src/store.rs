//! `SessionStore` — persistent session storage backed by SQLite.

use crate::{
    error::SessionError,
    models::{ProgramEntry, ProgramState, Session},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, Database, DatabaseConnection,
    EntityTrait, QueryFilter,
};
use tracing::instrument;

// ── Schema ────────────────────────────────────────────────────────────────────

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS sessions (
    id           TEXT PRIMARY KEY NOT NULL,
    user_id      TEXT NOT NULL,
    display_name TEXT NOT NULL,
    started_at   TEXT NOT NULL,
    programs     TEXT NOT NULL DEFAULT '[]'
);

CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
";

// ── SeaORM entity ─────────────────────────────────────────────────────────────

mod entity {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "sessions")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub user_id: String,
        pub display_name: String,
        pub started_at: String,
        /// JSON: `Vec<ProgramEntry>`
        pub programs: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ── Conversion ────────────────────────────────────────────────────────────────

impl TryFrom<entity::Model> for Session {
    type Error = SessionError;

    fn try_from(m: entity::Model) -> Result<Self, Self::Error> {
        use chrono::DateTime;

        let started_at = m.started_at.parse::<DateTime<chrono::Utc>>().map_err(|e| {
            SessionError::Json(
                serde_json::from_str::<serde_json::Value>(&e.to_string()).unwrap_err(),
            )
        })?;

        // Rebuild via private fields using serde round-trip
        let raw = serde_json::json!({
            "id":           m.id,
            "user_id":      m.user_id,
            "display_name": m.display_name,
            "started_at":   started_at,
            "programs":     serde_json::from_str::<serde_json::Value>(&m.programs)?,
        });
        Ok(serde_json::from_value(raw)?)
    }
}

// ── SessionStore ──────────────────────────────────────────────────────────────

/// Persistent store for user sessions.
#[derive(Debug)]
pub struct SessionStore {
    db: DatabaseConnection,
}

impl SessionStore {
    /// Open (or create) the session database. Use `":memory:"` in tests.
    #[instrument(name = "session_store.open")]
    pub async fn open(path: &str) -> Result<Self, SessionError> {
        let url = if path == ":memory:" {
            "sqlite::memory:".to_string()
        } else {
            format!("sqlite://{}?mode=rwc", path)
        };
        let db = Database::connect(&url).await?;
        db.execute_unprepared(SCHEMA).await?;
        Ok(Self { db })
    }

    // ── Session lifecycle ─────────────────────────────────────────────────────

    /// Create a new session for the given user.
    #[instrument(name = "session_store.create", skip(self))]
    pub async fn create(&self, user_id: &str, display_name: &str) -> Result<Session, SessionError> {
        let session = Session::new(user_id, display_name);
        self.persist(&session).await?;
        Ok(session)
    }

    /// Load a session by id.
    pub async fn get(&self, session_id: &str) -> Result<Session, SessionError> {
        entity::Entity::find_by_id(session_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| SessionError::NotFound {
                id: session_id.to_owned(),
            })
            .and_then(Session::try_from)
    }

    /// Load the most recent session for a user, if any.
    pub async fn get_for_user(&self, user_id: &str) -> Result<Option<Session>, SessionError> {
        entity::Entity::find()
            .filter(entity::Column::UserId.eq(user_id))
            .one(&self.db)
            .await?
            .map(Session::try_from)
            .transpose()
    }

    /// Delete a session (logout).
    #[instrument(name = "session_store.close")]
    pub async fn close(&self, session_id: &str) -> Result<(), SessionError> {
        let model = entity::Entity::find_by_id(session_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| SessionError::NotFound {
                id: session_id.to_owned(),
            })?;
        let active: entity::ActiveModel = model.into();
        active.delete(&self.db).await?;
        Ok(())
    }

    // ── Program management ────────────────────────────────────────────────────

    /// Register that a program was opened in this session.
    ///
    /// If the program is already open (e.g. minimized), this is a no-op —
    /// call `restore_program` instead.
    pub async fn open_program(
        &self,
        session_id: &str,
        program_id: &str,
    ) -> Result<(), SessionError> {
        let mut session = self.get(session_id).await?;
        if !session.is_open(program_id) {
            session.add_program(ProgramEntry::new(program_id));
            self.persist(&session).await?;
        }
        Ok(())
    }

    /// Set a program to `Minimized` state.
    pub async fn minimize_program(
        &self,
        session_id: &str,
        program_id: &str,
    ) -> Result<(), SessionError> {
        self.set_program_state(session_id, program_id, ProgramState::Minimized)
            .await
    }

    /// Restore a minimized program to `Open` state.
    pub async fn restore_program(
        &self,
        session_id: &str,
        program_id: &str,
    ) -> Result<(), SessionError> {
        self.set_program_state(session_id, program_id, ProgramState::Open)
            .await
    }

    /// Remove a program from the session (closed by user).
    pub async fn close_program(
        &self,
        session_id: &str,
        program_id: &str,
    ) -> Result<(), SessionError> {
        let mut session = self.get(session_id).await?;
        if !session.remove_program(program_id) {
            return Err(SessionError::ProgramNotOpen {
                session_id: session_id.to_owned(),
                program_id: program_id.to_owned(),
            });
        }
        self.persist(&session).await
    }

    // ── Internal ─────────────────────────────────────────────────────────────

    async fn set_program_state(
        &self,
        session_id: &str,
        program_id: &str,
        state: ProgramState,
    ) -> Result<(), SessionError> {
        let mut session = self.get(session_id).await?;
        if !session.set_program_state(program_id, state) {
            return Err(SessionError::ProgramNotOpen {
                session_id: session_id.to_owned(),
                program_id: program_id.to_owned(),
            });
        }
        self.persist(&session).await
    }

    async fn persist(&self, session: &Session) -> Result<(), SessionError> {
        let model = entity::Entity::find_by_id(session.id())
            .one(&self.db)
            .await?;

        let programs_json = serde_json::to_string(session.programs())?;

        if model.is_some() {
            let mut active = entity::ActiveModel {
                id: Set(session.id().to_owned()),
                user_id: Set(session.user_id().to_owned()),
                display_name: Set(session.display_name().to_owned()),
                started_at: Set(session.started_at().to_rfc3339()),
                programs: Set(programs_json),
            };
            active.programs = Set(serde_json::to_string(session.programs())?);
            active.update(&self.db).await?;
        } else {
            entity::ActiveModel {
                id: Set(session.id().to_owned()),
                user_id: Set(session.user_id().to_owned()),
                display_name: Set(session.display_name().to_owned()),
                started_at: Set(session.started_at().to_rfc3339()),
                programs: Set(programs_json),
            }
            .insert(&self.db)
            .await?;
        }
        Ok(())
    }
}
