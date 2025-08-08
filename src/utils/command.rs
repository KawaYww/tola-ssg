use crate::log;
use anyhow::Result;
use std::{
    ffi::OsString,
    path::Path,
    process::{ChildStdin, Command, Output, Stdio},
};

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

pub fn into_arg<S>(arg: S) -> OsString
where
    S: Into<OsString>,
{
    arg.into()
}

pub fn run_command(root: Option<&Path>, command: &[OsString], args: &[OsString]) -> Result<Output> {
    let command: Vec<OsString> = command.iter().map(into_arg).collect();
    let args: Vec<OsString> = [&command[1..], args].concat();
    let command_name = command[0].to_str().unwrap();

    let mut command = Command::new(command_name);
    let command = if let Some(root) = root {
        command.args(args).current_dir(root)
    } else {
        command.args(args)
    };

    let output = command.output()?;

    log_for_command(command_name, &output)?;

    Ok(output)
}

pub fn run_command_with_stdin(
    root: Option<&Path>,
    command: &[OsString],
    args: &[OsString],
) -> Result<ChildStdin> {
    let command: Vec<OsString> = command.iter().map(into_arg).collect();
    let args: Vec<OsString> = [&command[1..], args].concat();
    let command_name = command[0].to_str().unwrap();

    let mut command = Command::new(command_name);
    let command = if let Some(root) = root {
        command.args(args).current_dir(root)
    } else {
        command.args(args)
    };

    let mut output = command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let stdin = output.stdin.take().expect("handle present");

    Ok(stdin)
}

#[rustfmt::skip]
pub fn log_for_command(name: &str, output: &Output) -> Result<()> {
    let (stdout, stderr) = (str::from_utf8(&output.stdout)?.trim(), str::from_utf8(&output.stderr)?.trim());

    if !output.status.success() {
        anyhow::bail!("Command failed: {}", stderr);
    }

    // Configurable ignore prefixes
    let ignore_stdout = ["<!DOCTYPE html>"];
    let ignore_stderr = [
        "warning: html export is under active development and incomplete",
        "≈ tailwindcss v",
    ];

    if !ignore_stdout.iter().any(|s| stdout.starts_with(s)) {
        stdout.lines().filter(|s| !s.trim().is_empty()).for_each(|s| log!(name; "{s}"));
    }

    if !ignore_stderr.iter().any(|s| stderr.starts_with(s)) {
        stderr.lines().filter(|s| !s.trim().is_empty()).for_each(|s| log!(name; "{s}"));
    }

    Ok(())
}
