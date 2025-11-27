//! External command execution utilities.
//!
//! Provides macros and functions for running shell commands with proper
//! output handling and error reporting.

use crate::log;
use anyhow::Result;
use std::{
    ffi::OsString,
    path::Path,
    process::{ChildStdin, Command, Output, Stdio},
};

/// Run an external command with arguments
#[macro_export]
macro_rules! run_command {
    ($command:expr; $($arg:expr),*) => {{
        use $crate::utils::command::{run_command, into_arg};
        use std::ffi::OsString;

        let args: Vec<OsString> = [$(into_arg($arg),)*].into_iter().filter(|a| !a.is_empty()).collect();
        let command: Vec<OsString> = $command.iter().map(into_arg).collect();

        run_command(None, &command, &args)
    }};
    ($root:expr; $command:expr; $($arg:expr),*) => {{
        use $crate::utils::command::{run_command, into_arg};
        use std::ffi::OsString;

        let args: Vec<OsString> = [$(into_arg($arg),)*].into_iter().filter(|a| !a.is_empty()).collect();
        let command: Vec<OsString> = $command.iter().map(into_arg).collect();

        run_command(Some($root), &command, &args)
    }};
}

/// Run an external command and return a handle to its stdin
#[macro_export]
macro_rules! run_command_with_stdin {
    ($command:expr; $($arg:expr),*) => {{
        use $crate::utils::command::{run_command_with_stdin, into_arg};
        use std::ffi::OsString;

        let args: Vec<OsString> = [$(into_arg($arg),)*].into_iter().filter(|a| !a.is_empty()).collect();
        let command: Vec<OsString> = $command.iter().map(into_arg).collect();

        run_command_with_stdin(None, &command, &args)
    }};
    ($root:expr; $command:expr; $($arg:expr),*) => {{
        use $crate::utils::command::{run_command_with_stdin, into_arg};
        use std::ffi::OsString;

        let args: Vec<OsString> = [$(into_arg($arg),)*].into_iter().filter(|a| !a.is_empty()).collect();
        let command: Vec<OsString> = $command.iter().map(into_arg).collect();

        run_command_with_stdin(Some($root), &command, &args)
    }};
}

/// Convert any compatible type to OsString
pub fn into_arg<S: Into<OsString>>(arg: S) -> OsString {
    arg.into()
}

/// Execute a command and capture its output
pub fn run_command(root: Option<&Path>, command: &[OsString], args: &[OsString]) -> Result<Output> {
    let full_args: Vec<_> = [&command[1..], args].concat();
    let cmd_name = command[0].to_str().unwrap();

    let mut cmd = Command::new(cmd_name);
    cmd.args(&full_args);
    if let Some(root) = root {
        cmd.current_dir(root);
    }

    let output = cmd.output()?;
    log_command_output(cmd_name, &output)?;

    Ok(output)
}

/// Execute a command and return a handle to write to its stdin
pub fn run_command_with_stdin(
    root: Option<&Path>,
    command: &[OsString],
    args: &[OsString],
) -> Result<ChildStdin> {
    let full_args: Vec<_> = [&command[1..], args].concat();
    let cmd_name = command[0].to_str().unwrap();

    let mut cmd = Command::new(cmd_name);
    cmd.args(&full_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if let Some(root) = root {
        cmd.current_dir(root);
    }

    let mut child = cmd.spawn()?;
    Ok(child.stdin.take().expect("stdin handle present"))
}

/// Prefixes to ignore in stdout
const IGNORE_STDOUT: &[&str] = &["<!DOCTYPE html>", r#"{"#];

/// Prefixes to ignore in stderr
const IGNORE_STDERR: &[&str] = &[
    "warning: html export is under active development and incomplete",
    "warning: elem was ignored during paged export",
    "≈ tailwindcss v",
];

/// Typst HTML export warning to strip from error output
const TYPST_HTML_WARNING: &str = "warning: html export is under active development and incomplete
 = hint: its behaviour may change at any time
 = hint: do not rely on this feature for production use cases
 = hint: see https://github.com/typst/typst/issues/5512 for more information\n";

/// Log command output, filtering known noise
fn log_command_output(name: &str, output: &Output) -> Result<()> {
    let stdout = std::str::from_utf8(&output.stdout)?.trim();
    let stderr = std::str::from_utf8(&output.stderr)?.trim();

    if !output.status.success() {
        let cleaned_stderr = stderr.trim_start_matches(TYPST_HTML_WARNING);
        eprintln!("{cleaned_stderr}");
        anyhow::bail!("Command `{name}` failed");
    }

    // Log stdout unless it matches ignored prefixes
    if !IGNORE_STDOUT.iter().any(|s| stdout.starts_with(s)) {
        for line in stdout.lines().filter(|s| !s.trim().is_empty()) {
            log!(name; "{line}");
        }
    }

    // Log stderr unless it matches ignored prefixes
    if !IGNORE_STDERR.iter().any(|s| stderr.starts_with(s)) {
        for line in stderr.lines().filter(|s| !s.trim().is_empty()) {
            log!(name; "{line}");
        }
    }

    Ok(())
}
