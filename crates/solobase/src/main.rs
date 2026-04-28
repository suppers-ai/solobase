//! Solobase — unified CLI dispatcher.
//!
//! Bare `solobase` boots the native server (preserves the prior bin UX
//! and `examples/run-tests.sh`). All other invocations parse a verb
//! (`build`/`serve`) + flags and dispatch to one of the four flow
//! handlers based on (mode × target). Mode is auto-detected from the
//! presence of a `Cargo.toml` walking up from the cwd; target defaults
//! follow the crate-types in that Cargo.toml when in embed mode.

use std::process::ExitCode;

use clap::Parser;

use solobase::cli::{
    cli_args::{Cli, Command, Target},
    flows::{embed_native, embed_web, sealed_native, sealed_web},
    mode::{default_target, detect_mode, Mode, ModeContext},
};

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e:#}");
            ExitCode::from(1)
        }
    }
}

async fn run() -> anyhow::Result<()> {
    // Bare `solobase` (no args) defaults to `serve --target native`.
    let cli = if std::env::args_os().count() == 1 {
        Cli::default()
    } else {
        Cli::parse()
    };

    let cwd = std::env::current_dir()?;
    let ctx = ModeContext::scan(&cwd)?;

    // The native server-boot helper (`cli::server::run`) reads `.env` from
    // the current working directory, so anchor cwd to ctx.cwd before
    // dispatch. Flow functions take `repo_root` explicitly to avoid
    // depending on cwd internally.
    std::env::set_current_dir(&ctx.cwd)?;

    match cli.command {
        Command::Build { target, release } => {
            let target = default_target(&ctx, target)?;
            dispatch_build(&ctx, target, release).await
        }
        Command::Serve { target, release, port } => {
            let target = default_target(&ctx, target)?;
            dispatch_serve(&ctx, target, release, port).await
        }
    }
}

async fn dispatch_build(ctx: &ModeContext, target: Target, release: bool) -> anyhow::Result<()> {
    let repo_root = &ctx.cwd;
    match (detect_mode(ctx), target) {
        (Mode::Sealed, Target::Native) => sealed_native::build(repo_root, release).await,
        (Mode::Sealed, Target::Web) => sealed_web::build(repo_root, release).await,
        (Mode::Embed, Target::Native) => embed_native::build(repo_root, release).await,
        (Mode::Embed, Target::Web) => embed_web::build(repo_root, release).await,
    }
}

async fn dispatch_serve(
    ctx: &ModeContext,
    target: Target,
    release: bool,
    port: Option<u16>,
) -> anyhow::Result<()> {
    let repo_root = &ctx.cwd;
    match (detect_mode(ctx), target) {
        (Mode::Sealed, Target::Native) => sealed_native::serve(repo_root, release, port).await,
        (Mode::Sealed, Target::Web) => sealed_web::serve(repo_root, release, port).await,
        (Mode::Embed, Target::Native) => embed_native::serve(repo_root, release, port).await,
        (Mode::Embed, Target::Web) => embed_web::serve(repo_root, release, port).await,
    }
}
