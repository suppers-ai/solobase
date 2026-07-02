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

        /// Apply pending block migrations on startup.
        /// Blocks whose SQL hash has changed will be applied and their blessed_hash updated.
        /// Safe to run on every deploy but slower; omit once schema is stable.
        #[arg(long)]
        run_migrations: bool,
    },
    /// Build the app and deploy it to the target's hosting environment.
    /// (v1: only `--target cloudflare` is supported.)
    ///
    /// Cloudflare deploys are atomic: an unpromoted version is uploaded,
    /// migrations + seeds run against it via an authenticated
    /// `/_deploy/init` call, and only on success is the version promoted
    /// to 100% traffic. There is no `--run-migrations` flag here (unlike
    /// `serve`) — every deploy always runs the init funnel.
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

// No `Default for Cli` impl on purpose: hand-constructing a default
// `Cli` bypasses clap's verb-level flag handling, so any new flag added
// to a verb silently defaults to whatever value happens to be in the
// hand-rolled default. The bare-`solobase` fallback in `main` instead
// reparses the synthetic argv `["solobase", "serve"]`, which keeps clap
// as the single source of truth.
