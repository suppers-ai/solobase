//! Writes framework static assets to a target directory, then runs the
//! bundler to content-hash assets and render templates. Invoked from
//! consumer Makefiles after `wasm-pack build`.
//!
//! Usage: `export-assets <pkg-dir> [--repo-dir <path>] [--dev]`

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

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
    solobase_browser::tools::bundle::run(&cli.pkg_dir, &repo, cli.dev)?;

    Ok(())
}
