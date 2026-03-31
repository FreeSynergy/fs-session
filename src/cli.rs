// cli.rs — CLI for fs-session.

use clap::{Parser, Subcommand};

/// `FreeSynergy` session manager CLI.
#[derive(Parser)]
#[command(name = "fs-session", version, about = "Manage user sessions")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run as daemon (gRPC server + bus subscriber).
    Daemon,
    /// Print the currently active user session.
    CurrentUser,
    /// List all open sessions.
    List,
    /// Show all open apps for a session.
    OpenApps {
        /// Session ID.
        #[arg(short, long)]
        session_id: String,
    },
    /// Print session info for a user.
    Info {
        /// User ID.
        #[arg(short, long)]
        user_id: String,
    },
}
