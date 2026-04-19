use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "solobase-web-bundle")]
#[command(about = "Content-hash solobase-web/pkg assets and render SW/HTML templates")]
struct Cli {
    /// Path to the `pkg/` directory produced by wasm-pack.
    pkg_dir: PathBuf,

    /// Repo root (used to read `git rev-parse` for the build id). Defaults to `pkg_dir`'s parent.
    #[arg(long)]
    repo_dir: Option<PathBuf>,

    /// Skip hashing; render templates with canonical filenames for fast local iteration.
    #[arg(long)]
    dev: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo = cli
        .repo_dir
        .clone()
        .or_else(|| cli.pkg_dir.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| cli.pkg_dir.clone());
    solobase_web_bundle::run(&cli.pkg_dir, &repo, cli.dev)
}
