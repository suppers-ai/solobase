//! Verifies the clap CLI accepts the verbs and flags from the unified
//! design (build/serve, --target native|web, --release, --port).

use clap::Parser;
use solobase::cli::cli_args::{Cli, Command, Target};

#[test]
fn parses_build_with_target_native() {
    let cli = Cli::parse_from(["solobase", "build", "--target", "native"]);
    if let Command::Build { target, release } = cli.command {
        assert_eq!(target, Some(Target::Native));
        assert!(!release);
    } else {
        panic!("expected Build");
    }
}

#[test]
fn parses_build_with_release_flag() {
    let cli = Cli::parse_from(["solobase", "build", "--release"]);
    if let Command::Build { release, .. } = cli.command {
        assert!(release);
    } else {
        panic!("expected Build");
    }
}

#[test]
fn parses_serve_with_port() {
    let cli = Cli::parse_from(["solobase", "serve", "--port", "9090"]);
    if let Command::Serve { port, .. } = cli.command {
        assert_eq!(port, Some(9090));
    } else {
        panic!("expected Serve");
    }
}

#[test]
fn bare_solobase_uses_serve_native_default() {
    // The actual fallback lives in main(); here we assert the Default impl.
    let cli = Cli::default();
    if let Command::Serve {
        target,
        release,
        port,
    } = cli.command
    {
        assert_eq!(target, Some(Target::Native));
        assert!(!release);
        assert_eq!(port, None);
    } else {
        panic!("expected Serve");
    }
}
