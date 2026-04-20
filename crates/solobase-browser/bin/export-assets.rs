//! Writes framework static assets to a target directory, then runs the
//! bundler to content-hash assets and render templates. Invoked from
//! consumer Makefiles after `wasm-pack build`.
//!
//! Usage: `export-assets <pkg-dir> [--repo-dir <path>] [--dev]
//!             [--app-name <name>] [--app-title <title>]
//!             [--boot-redirect <url>]`

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use solobase_browser::tools::bundle::AppConfig;

#[derive(Parser)]
#[command(name = "export-assets")]
#[command(about = "Write solobase-browser assets + run the bundler post-processor")]
struct Cli {
    /// Path to the `pkg/` directory produced by wasm-pack.
    pkg_dir: PathBuf,

    /// Repo root (used to read `git rev-parse` for the build id).
    /// Defaults to `pkg_dir`'s parent.
    #[arg(long)]
    repo_dir: Option<PathBuf>,

    /// Skip asset hashing; render templates with canonical filenames.
    #[arg(long)]
    dev: bool,

    /// Log prefix shown in sw.js / loader.js console messages.
    /// Defaults to the discovered wasm-pack base name (e.g. `solobase_web`).
    #[arg(long)]
    app_name: Option<String>,

    /// Title rendered into `<title>` and `<h1>` in index.html.
    /// Defaults to the base name with underscores replaced by spaces.
    #[arg(long)]
    app_title: Option<String>,

    /// URL the loader navigates to after the Service Worker activates.
    /// Defaults to `/`.
    #[arg(long)]
    boot_redirect: Option<String>,

    /// Additional URL path prefixes that the Service Worker's fetch handler
    /// should bypass (let the origin serve directly). Comma-separated. Each
    /// entry is matched with `url.pathname.startsWith(...)`, so exact-match
    /// paths like `/ai-bridge.js` work too (no path has that as a prefix
    /// child). Use this for consumer-specific static assets referenced from
    /// the UI (branded CSS, extra JS modules) that the WASM runtime's router
    /// doesn't know about.
    #[arg(long, value_delimiter = ',')]
    extra_bypass_prefix: Vec<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // 1. Write static assets into pkg_dir.
    solobase_browser::assets::write_to(&cli.pkg_dir)?;

    // 2. Run the bundler to content-hash assets + render templates.
    let repo = cli
        .repo_dir
        .clone()
        .or_else(|| cli.pkg_dir.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| cli.pkg_dir.clone());

    let app = AppConfig {
        app_name: cli.app_name,
        app_title: cli.app_title,
        boot_redirect: cli.boot_redirect,
        extra_bypass_prefix: cli.extra_bypass_prefix,
    };

    solobase_browser::tools::bundle::run(&cli.pkg_dir, &repo, cli.dev, app)?;

    Ok(())
}
