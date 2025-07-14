use std::{ffi::OsString, path::Path, process::{Command, Output}};
use anyhow::Result;
use crate::log;

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


pub fn into_arg<S>(arg: S) -> OsString
where S: Into<OsString>,
{
    arg.into()
}

pub fn run_command(root: Option<&Path>, command: &[OsString], args: &[OsString]) -> Result<Output> {
    let command: Vec<OsString> = command.iter().map(into_arg).collect();
    let args: Vec<OsString> = [&command[1..], args].concat();

    let output = if let Some(root) = root {
        Command::new(&command[0]).args(args).current_dir(root).output()?
    } else {
        Command::new(&command[0]).args(args).output()?
    };

    log_for_command(command[0].to_str().unwrap(), &output)?;

    Ok(output)
}

pub fn log_for_command(name: &str, output: &Output) -> Result<()> {
    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed: {}", error_msg);
    }

    let (stdout_msg, stderr_msg) = (String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    for line in stdout_msg.lines().map(|s| s.trim()) {
        log!(name, "{line}");
    }
    for line in stderr_msg.lines().map(|s| s.trim()) {
        // ignore warning from `typst` command, which will appear when enabling experimental `html` feature
        if line.starts_with("warning: html export is under active development and incomplete") {
            break
        }
        log!(name, "{line}");
    }
    Ok(())
}
