//! Clap definitions for the unified solobase CLI.

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "solobase",
    version,
    about = "Solobase — build and run Solobase apps for native or browser targets",
    arg_required_else_help = false
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Build the app for `--target` (defaults per directory contents).
    Build {
        #[arg(long)]
        target: Option<Target>,

        /// Use the release profile. Default is debug for fast iteration.
        #[arg(long)]
        release: bool,
    },
    /// Build the app and run a server.
    Serve {
        #[arg(long)]
        target: Option<Target>,

        #[arg(long)]
        release: bool,

        /// Override the listen port. Native: from .env. Web: defaults to 8080.
        #[arg(long)]
        port: Option<u16>,
    },
    /// Build the app and deploy it to the target's hosting environment.
    /// (v1: only `--target cloudflare` is supported.)
    Deploy {
        #[arg(long)]
        target: Option<Target>,

        #[arg(long)]
        release: bool,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Native,
    Web,
    Cloudflare,
}

impl Default for Cli {
    fn default() -> Self {
        Self {
            command: Command::Serve {
                target: Some(Target::Native),
                release: false,
                port: None,
            },
        }
    }
}
