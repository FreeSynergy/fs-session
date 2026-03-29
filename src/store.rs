//! `SessionStore` trait and `SqliteSessionStore` — SQLite-backed implementation.

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, Database, DatabaseConnection,
    EntityTrait, QueryFilter,
};
use tracing::instrument;

use crate::{
    error::SessionError,
    models::{AppSession, AppState, Session},
};

// ── SessionStore trait ────────────────────────────────────────────────────────

/// Interface for session storage.
///
/// Concrete implementations: [`SqliteSessionStore`].
/// Consumer code must depend on this trait only — never on the concrete type.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Create a new session for the given user.
    async fn create(&self, user_id: &str, display_name: &str) -> Result<Session, SessionError>;

    /// Load a session by id.
    async fn get(&self, session_id: &str) -> Result<Session, SessionError>;

    /// Load the most recent session for a user, if any.
    async fn get_for_user(&self, user_id: &str) -> Result<Option<Session>, SessionError>;

    /// All active sessions.
    async fn list(&self) -> Result<Vec<Session>, SessionError>;

    /// The most recently started session across all users, if any.
    ///
    /// Useful for single-user systems where there is exactly one active session.
    async fn active_user(&self) -> Result<Option<Session>, SessionError>;

    /// Delete a session (logout).
    async fn close(&self, session_id: &str) -> Result<(), SessionError>;

    /// Register that an application was opened in this session.
    ///
    /// If the app is already open (e.g. minimized), this is a no-op —
    /// call `restore_app` instead.
    async fn open_app(&self, session_id: &str, app_id: &str) -> Result<(), SessionError>;

    /// Set an application to `Minimized` state.
    async fn minimize_app(&self, session_id: &str, app_id: &str) -> Result<(), SessionError>;

    /// Restore a minimized application to `Open` state.
    async fn restore_app(&self, session_id: &str, app_id: &str) -> Result<(), SessionError>;

    /// Remove an application from the session (closed by user).
    async fn close_app(&self, session_id: &str, app_id: &str) -> Result<(), SessionError>;
}

// ── Schema ────────────────────────────────────────────────────────────────────

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS sessions (
    id           TEXT PRIMARY KEY NOT NULL,
    user_id      TEXT NOT NULL,
    display_name TEXT NOT NULL,
    started_at   TEXT NOT NULL,
    apps         TEXT NOT NULL DEFAULT '[]'
);

CREATE INDEX IF NOT EXISTS idx_sessions_user    ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_started ON sessions(started_at);
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
        /// JSON: `Vec<AppSession>`
        pub apps: String,
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

        let started_at = m
            .started_at
            .parse::<DateTime<chrono::Utc>>()
            .map_err(|_| SessionError::Parse(m.started_at.clone()))?;

        let raw = serde_json::json!({
            "id":           m.id,
            "user_id":      m.user_id,
            "display_name": m.display_name,
            "started_at":   started_at,
            "apps":         serde_json::from_str::<serde_json::Value>(&m.apps)?,
        });
        Ok(serde_json::from_value(raw)?)
    }
}

// ── SqliteSessionStore ────────────────────────────────────────────────────────

/// SQLite-backed session store. Implements [`SessionStore`].
#[derive(Debug)]
pub struct SqliteSessionStore {
    db: DatabaseConnection,
}

impl SqliteSessionStore {
    /// Open (or create) the session database at the given file path.
    ///
    /// Use `":memory:"` in tests.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] if the connection or schema setup fails.
    #[instrument(name = "sqlite_session_store.open")]
    pub async fn open(path: &str) -> Result<Self, SessionError> {
        let url = if path == ":memory:" {
            "sqlite::memory:".to_string()
        } else {
            format!("sqlite://{path}?mode=rwc")
        };
        let db = Database::connect(&url).await?;
        db.execute_unprepared(SCHEMA).await?;
        Ok(Self { db })
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    async fn load(&self, session_id: &str) -> Result<Session, SessionError> {
        entity::Entity::find_by_id(session_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| SessionError::NotFound {
                id: session_id.to_owned(),
            })
            .and_then(Session::try_from)
    }

    async fn persist(&self, session: &Session) -> Result<(), SessionError> {
        let exists = entity::Entity::find_by_id(session.id())
            .one(&self.db)
            .await?
            .is_some();

        let apps_json = serde_json::to_string(session.apps())?;

        if exists {
            entity::ActiveModel {
                id: Set(session.id().to_owned()),
                user_id: Set(session.user_id().to_owned()),
                display_name: Set(session.display_name().to_owned()),
                started_at: Set(session.started_at().to_rfc3339()),
                apps: Set(apps_json),
            }
            .update(&self.db)
            .await?;
        } else {
            entity::ActiveModel {
                id: Set(session.id().to_owned()),
                user_id: Set(session.user_id().to_owned()),
                display_name: Set(session.display_name().to_owned()),
                started_at: Set(session.started_at().to_rfc3339()),
                apps: Set(apps_json),
            }
            .insert(&self.db)
            .await?;
        }
        Ok(())
    }

    async fn set_app_state(
        &self,
        session_id: &str,
        app_id: &str,
        state: AppState,
    ) -> Result<(), SessionError> {
        let mut session = self.load(session_id).await?;
        if !session.set_app_state(app_id, state) {
            return Err(SessionError::AppNotOpen {
                session_id: session_id.to_owned(),
                app_id: app_id.to_owned(),
            });
        }
        self.persist(&session).await
    }
}

#[async_trait]
impl SessionStore for SqliteSessionStore {
    #[instrument(name = "session_store.create", skip(self))]
    async fn create(&self, user_id: &str, display_name: &str) -> Result<Session, SessionError> {
        let session = Session::new(user_id, display_name);
        self.persist(&session).await?;
        Ok(session)
    }

    async fn get(&self, session_id: &str) -> Result<Session, SessionError> {
        self.load(session_id).await
    }

    async fn get_for_user(&self, user_id: &str) -> Result<Option<Session>, SessionError> {
        entity::Entity::find()
            .filter(entity::Column::UserId.eq(user_id))
            .one(&self.db)
            .await?
            .map(Session::try_from)
            .transpose()
    }

    async fn list(&self) -> Result<Vec<Session>, SessionError> {
        entity::Entity::find()
            .all(&self.db)
            .await?
            .into_iter()
            .map(Session::try_from)
            .collect()
    }

    async fn active_user(&self) -> Result<Option<Session>, SessionError> {
        use sea_orm::{Order, QueryOrder};
        entity::Entity::find()
            .order_by(entity::Column::StartedAt, Order::Desc)
            .one(&self.db)
            .await?
            .map(Session::try_from)
            .transpose()
    }

    #[instrument(name = "session_store.close", skip(self))]
    async fn close(&self, session_id: &str) -> Result<(), SessionError> {
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

    async fn open_app(&self, session_id: &str, app_id: &str) -> Result<(), SessionError> {
        let mut session = self.load(session_id).await?;
        if !session.is_open(app_id) {
            session.add_app(AppSession::new(app_id));
            self.persist(&session).await?;
        }
        Ok(())
    }

    async fn minimize_app(&self, session_id: &str, app_id: &str) -> Result<(), SessionError> {
        self.set_app_state(session_id, app_id, AppState::Minimized)
            .await
    }

    async fn restore_app(&self, session_id: &str, app_id: &str) -> Result<(), SessionError> {
        self.set_app_state(session_id, app_id, AppState::Open).await
    }

    async fn close_app(&self, session_id: &str, app_id: &str) -> Result<(), SessionError> {
        let mut session = self.load(session_id).await?;
        if !session.remove_app(app_id) {
            return Err(SessionError::AppNotOpen {
                session_id: session_id.to_owned(),
                app_id: app_id.to_owned(),
            });
        }
        self.persist(&session).await
    }
}
