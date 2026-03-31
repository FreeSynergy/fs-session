// grpc.rs — gRPC service implementation for fs-session.
//
// Wraps Arc<dyn SessionStore> and exposes it via the SessionService proto.
// Routes:
//   CurrentUser / OpenApps / SessionInfo / Health

use std::sync::Arc;

use tonic::{Request, Response, Status};
use tracing::instrument;

use crate::{
    models::{AppSession, Session},
    store::SessionStore,
};

// Include the generated tonic code.
pub mod proto {
    #![allow(clippy::all, clippy::pedantic, warnings)]
    tonic::include_proto!("session");
}

pub use proto::session_service_server::{SessionService, SessionServiceServer};
pub use proto::{
    AppSessionProto, CurrentUserRequest, CurrentUserResponse, HealthRequest, HealthResponse,
    OpenAppsRequest, OpenAppsResponse, SessionInfoRequest, SessionInfoResponse, SessionProto,
};

// ── Conversions ───────────────────────────────────────────────────────────────

fn session_to_proto(s: &Session) -> SessionProto {
    SessionProto {
        id: s.id().to_owned(),
        user_id: s.user_id().to_owned(),
        user_name: s.display_name().to_owned(),
        started_at: s.started_at().to_rfc3339(),
        apps: s.apps().iter().map(app_to_proto).collect(),
    }
}

fn app_to_proto(a: &AppSession) -> AppSessionProto {
    AppSessionProto {
        app_id: a.app_id.clone(),
        state: a.state.to_string().to_lowercase(),
        opened_at: a.opened_at.to_rfc3339(),
    }
}

// ── GrpcSession ───────────────────────────────────────────────────────────────

/// gRPC service wrapper around a shared [`SessionStore`].
pub struct GrpcSession<S> {
    store: Arc<S>,
}

impl<S: SessionStore> GrpcSession<S> {
    /// Wrap `store` in a gRPC service.
    #[must_use]
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }
}

#[tonic::async_trait]
impl<S: SessionStore + Send + Sync + 'static> SessionService for GrpcSession<S> {
    #[instrument(name = "grpc.session.current_user", skip(self))]
    async fn current_user(
        &self,
        _req: Request<CurrentUserRequest>,
    ) -> Result<Response<CurrentUserResponse>, Status> {
        let session = self
            .store
            .active_user()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(CurrentUserResponse {
            session: session.as_ref().map(session_to_proto),
        }))
    }

    #[instrument(name = "grpc.session.open_apps", skip(self))]
    async fn open_apps(
        &self,
        req: Request<OpenAppsRequest>,
    ) -> Result<Response<OpenAppsResponse>, Status> {
        let session_id = req.into_inner().session_id;
        let session = self
            .store
            .get(&session_id)
            .await
            .map_err(|e| Status::not_found(e.to_string()))?;
        Ok(Response::new(OpenAppsResponse {
            apps: session.apps().iter().map(app_to_proto).collect(),
        }))
    }

    #[instrument(name = "grpc.session.session_info", skip(self))]
    async fn session_info(
        &self,
        req: Request<SessionInfoRequest>,
    ) -> Result<Response<SessionInfoResponse>, Status> {
        let user_id = req.into_inner().user_id;
        let session = self
            .store
            .get_for_user(&user_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(SessionInfoResponse {
            session: session.as_ref().map(session_to_proto),
        }))
    }

    async fn health(
        &self,
        _req: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        Ok(Response::new(HealthResponse {
            ok: true,
            version: env!("CARGO_PKG_VERSION").to_owned(),
        }))
    }
}
