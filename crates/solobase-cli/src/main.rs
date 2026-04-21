mod build;
mod cmd;
mod config;
mod serve;
mod skills;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "solobase",
    about = "Build / dev / serve for solobase-browser consumers",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the consumer app. Release profile unless --dev is given.
    Build {
        /// Use the dev profile (skips wasm-opt + content-hashing).
        /// Equivalent to the `dev` subcommand.
        #[arg(long, default_value_t = false)]
        dev: bool,
    },
    /// Alias for `build --dev`.
    Dev,
    /// Build (dev) then serve `dist_dir` over http.
    Serve {
        /// TCP port. Defaults to 8080.
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let cwd = std::env::current_dir()?;
    let (cfg, repo_root) = config::find_and_load(&cwd)?;
    match cli.command {
        Commands::Build { dev } => {
            let profile = if dev {
                build::BuildProfile::Dev
            } else {
                build::BuildProfile::Release
            };
            build::run(&cfg, &repo_root, profile)?;
        }
        Commands::Dev => build::run(&cfg, &repo_root, build::BuildProfile::Dev)?,
        Commands::Serve { port } => serve::run(&cfg, &repo_root, port)?,
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}
