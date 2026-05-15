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
    flows::{embed_cloudflare, embed_native, embed_web, sealed_native, sealed_web},
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
    // Parse the synthetic argv `["solobase", "serve"]` rather than
    // hand-constructing a default `Cli`, so clap's verb-level defaults
    // (port, release, run-migrations) and any future flag additions stay
    // in one place — `Cli::default()` would silently bypass them.
    let cli = if std::env::args_os().count() == 1 {
        Cli::try_parse_from(["solobase", "serve"])?
    } else {
        Cli::parse()
    };

    let cwd = std::env::current_dir()?;
    let ctx = ModeContext::scan(&cwd)?;

    // Flow functions take `repo_root` (= `ctx.cwd`) explicitly, and
    // `cli::server::run` plumbs it into `load_dotenv` directly. We no
    // longer mutate the process cwd here — process-global state shouldn't
    // change as a side effect of dispatch (and `set_current_dir` is racy
    // with anything else holding a cwd-relative path).

    match cli.command {
        Command::Build { target, release } => {
            let target = default_target(&ctx, target)?;
            dispatch_build(&ctx, target, release).await
        }
        Command::Serve {
            target,
            release,
            port,
            run_migrations,
        } => {
            let target = default_target(&ctx, target)?;
            dispatch_serve(&ctx, target, release, port, run_migrations).await
        }
        Command::Deploy {
            target,
            release,
            run_migrations,
        } => {
            let target = default_target(&ctx, target)?;
            dispatch_deploy(&ctx, target, release, run_migrations).await
        }
    }
}

async fn dispatch_build(ctx: &ModeContext, target: Target, release: bool) -> anyhow::Result<()> {
    let repo_root = &ctx.cwd;
    match (detect_mode(ctx), target) {
        (Mode::Sealed, Target::Native) => sealed_native::build(repo_root, release).await,
        (Mode::Sealed, Target::Web) => sealed_web::build(repo_root, release).await,
        (Mode::Sealed, Target::Cloudflare) => anyhow::bail!(
            "--target cloudflare requires a Cargo package; sealed mode not yet implemented"
        ),
        (Mode::Embed, Target::Native) => embed_native::build(repo_root, release).await,
        (Mode::Embed, Target::Web) => embed_web::build(repo_root, release).await,
        (Mode::Embed, Target::Cloudflare) => embed_cloudflare::build(repo_root, release).await,
    }
}

async fn dispatch_serve(
    ctx: &ModeContext,
    target: Target,
    release: bool,
    port: Option<u16>,
    run_migrations: bool,
) -> anyhow::Result<()> {
    let repo_root = &ctx.cwd;
    match (detect_mode(ctx), target) {
        (Mode::Sealed, Target::Native) => {
            sealed_native::serve(repo_root, release, port, run_migrations).await
        }
        (Mode::Sealed, Target::Web) => {
            sealed_web::serve(repo_root, release, port, run_migrations).await
        }
        (Mode::Sealed, Target::Cloudflare) => anyhow::bail!(
            "--target cloudflare requires a Cargo package; sealed mode not yet implemented"
        ),
        (Mode::Embed, Target::Native) => {
            embed_native::serve(repo_root, release, port, run_migrations).await
        }
        (Mode::Embed, Target::Web) => {
            embed_web::serve(repo_root, release, port, run_migrations).await
        }
        (Mode::Embed, Target::Cloudflare) => {
            embed_cloudflare::serve(repo_root, release, port, run_migrations).await
        }
    }
}

async fn dispatch_deploy(
    ctx: &ModeContext,
    target: Target,
    release: bool,
    run_migrations: bool,
) -> anyhow::Result<()> {
    let repo_root = &ctx.cwd;
    match (detect_mode(ctx), target) {
        (Mode::Embed, Target::Cloudflare) => {
            embed_cloudflare::deploy(repo_root, release, run_migrations).await
        }
        (Mode::Sealed, Target::Cloudflare) => anyhow::bail!(
            "--target cloudflare requires a Cargo package; sealed mode not yet implemented"
        ),
        _ => anyhow::bail!("solobase deploy is only implemented for --target cloudflare in v1"),
    }
}
