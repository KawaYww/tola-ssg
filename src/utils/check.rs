use std::process::Command;
use anyhow::{Context, Result};
use crate::config::SiteConfig;

pub fn check_required_command_installed(config: &'static SiteConfig) -> Result<()> {
    check_typst_installed(config)?;
    check_tailwind_installed(config)?;
    Ok(())
}

fn check_typst_installed(config: &SiteConfig) -> Result<()> {
    let command = config.build.typst_command.as_str();
    
    Command::new(command)
        .arg("--version")
        .output()
        .map(|_| ())
        .with_context(|| not_found_message(command))
}

fn check_tailwind_installed(config: &SiteConfig) -> Result<()> {
    if !config.tailwind.enable { return Ok(()) }
    
    let command = config.tailwind.command.as_str();

    Command::new(command)
        .arg("-h")
        .output()
        .map(|_| ())
        .with_context(|| not_found_message(command))
}

fn not_found_message(command: &str) -> String {
    format!("[checker] `{command}` not found. Please install `{command}` first.")
}
