use std::process::Command;

/// Error message shape when a child process exits non-zero.
///
/// ```text
/// error: <step> failed
///   command: <arg0> <arg1> ...
///   exit code: <n>
///   --- stderr ---
///   <child stderr>
/// ```
pub fn format_child_error(
    step: &str,
    cmd: &Command,
    exit_code: Option<i32>,
    stderr: &str,
) -> String {
    use std::fmt::Write;

    let program = cmd.get_program().to_string_lossy();
    let code = exit_code
        .map(|c| c.to_string())
        .unwrap_or_else(|| "<signal>".to_string());

    // Write the command line directly into the output buffer rather than
    // first collecting args into a `Vec<String>` and joining — the args
    // are only ever read once, so there's no reason to allocate twice.
    let mut out = String::with_capacity(128 + stderr.len());
    let _ = write!(&mut out, "error: {step} failed\n  command: {program}");
    for arg in cmd.get_args() {
        let _ = write!(&mut out, " {}", arg.to_string_lossy());
    }
    let _ = write!(
        &mut out,
        "\n  exit code: {code}\n  --- stderr ---\n{stderr}",
    );
    out
}

/// Run `cmd` inheriting stdio, and map non-zero exits to an `anyhow::Error`.
/// The child's stderr streams directly to the parent, so the user sees it
/// live. The error returned uses the same format as `format_child_error`
/// with a `(stderr streamed above)` placeholder instead of a captured tail.
///
/// Dry-run: when `SOLOBASE_CLI_DRY_RUN=1`, the command is NOT spawned;
/// instead a single line of the form
/// `DRY_RUN step="..." cmd=<prog> args=[...]` is printed to stdout and
/// the call returns Ok. Used by golden tests in
/// `tests/dry_run_goldens.rs` to assert flow command construction.
pub fn run(step: &str, mut cmd: Command) -> anyhow::Result<()> {
    if std::env::var("SOLOBASE_CLI_DRY_RUN").as_deref() == Ok("1") {
        let prog = cmd.get_program().to_string_lossy().into_owned();
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        println!("DRY_RUN step={step:?} cmd={prog} args={args:?}");
        return Ok(());
    }
    let status = cmd
        .status()
        .map_err(|e| anyhow::anyhow!("spawn {step}: {e}"))?;
    if status.success() {
        return Ok(());
    }
    Err(anyhow::Error::msg(format_child_error(
        step,
        &cmd,
        status.code(),
        "(stderr streamed above)",
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_error_message_shape() {
        let mut cmd = Command::new("wasm-pack");
        cmd.arg("build").arg("--target").arg("web");
        let msg = format_child_error("wasm-pack build", &cmd, Some(101), "boom\n");
        assert!(msg.contains("error: wasm-pack build failed"));
        assert!(msg.contains("command: wasm-pack build --target web"));
        assert!(msg.contains("exit code: 101"));
        assert!(msg.contains("--- stderr ---"));
        assert!(msg.contains("boom"));
    }

    #[test]
    fn format_error_unknown_exit_code() {
        let cmd = Command::new("sleep");
        let msg = format_child_error("sleep", &cmd, None, "");
        assert!(msg.contains("exit code: <signal>"));
    }
}
