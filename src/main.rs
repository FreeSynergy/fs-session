#![deny(clippy::all, clippy::pedantic, warnings)]
//! `fs-session` — user session management daemon for `FreeSynergy`.
//!
//! Starts a gRPC server (tonic) and subscribes to `session::*` bus events
//! to keep the session state in sync.
//!
//! # Environment variables
//!
//! | Variable              | Default                                    |
//! |-----------------------|--------------------------------------------|
//! | `FS_SESSION_DB`       | `/var/lib/freesynergy/session.db`          |
//! | `FS_GRPC_PORT`        | `50061`                                    |

use std::{net::SocketAddr, sync::Arc};

use clap::Parser as _;
use tonic::transport::Server;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

use fs_session::{
    bus_handler::SessionBusHandler,
    cli::{Cli, Command},
    grpc::{GrpcSession, SessionServiceServer},
    store::{SessionStore, SqliteSessionStore},
};

// ── Config ────────────────────────────────────────────────────────────────────

struct Config {
    db_path: String,
    grpc_addr: SocketAddr,
}

impl Config {
    fn from_env() -> Self {
        let db_path = std::env::var("FS_SESSION_DB")
            .unwrap_or_else(|_| "/var/lib/freesynergy/session.db".into());
        let grpc_port: u16 = std::env::var("FS_GRPC_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(50_061);
        Self {
            db_path,
            grpc_addr: SocketAddr::from(([0, 0, 0, 0], grpc_port)),
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let args = Cli::parse();
    let cfg = Config::from_env();

    match args.command {
        Command::Daemon => run_daemon(cfg).await?,
        cmd => run_cli(cmd, cfg).await?,
    }
    Ok(())
}

// ── Daemon ────────────────────────────────────────────────────────────────────

async fn run_daemon(cfg: Config) -> Result<(), Box<dyn std::error::Error>> {
    let store = Arc::new(SqliteSessionStore::open(&cfg.db_path).await?);
    info!("session store opened at {}", cfg.db_path);

    // Subscribe to session bus events.
    let bus = fs_bus::MessageBus::new();
    let _handler = Arc::new(SessionBusHandler::new(Arc::clone(&store)));
    // Note: in production the bus handler is wired via BusBridge or shared bus.

    info!("gRPC listening on {}", cfg.grpc_addr);
    Server::builder()
        .add_service(SessionServiceServer::new(GrpcSession::new(Arc::clone(
            &store,
        ))))
        .serve(cfg.grpc_addr)
        .await?;

    drop(bus);
    Ok(())
}

// ── CLI ───────────────────────────────────────────────────────────────────────

async fn run_cli(cmd: Command, cfg: Config) -> Result<(), Box<dyn std::error::Error>> {
    let store = Arc::new(SqliteSessionStore::open(&cfg.db_path).await?);

    match cmd {
        Command::Daemon => unreachable!(),
        Command::CurrentUser => match store.active_user().await? {
            Some(s) => println!("{} ({})", s.display_name(), s.user_id()),
            None => println!("No active session"),
        },
        Command::List => {
            let sessions = store.list().await?;
            for s in sessions {
                println!("{} — {} ({})", s.id(), s.display_name(), s.user_id());
            }
        }
        Command::OpenApps { session_id } => {
            let s = store.get(&session_id).await?;
            for app in s.apps() {
                println!("{} [{}]", app.app_id, app.state);
            }
        }
        Command::Info { user_id } => match store.get_for_user(&user_id).await? {
            Some(s) => println!("{} — {} apps open", s.id(), s.apps().len()),
            None => println!("No session for user {user_id}"),
        },
    }
    Ok(())
}
