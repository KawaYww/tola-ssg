mod build;
mod cli;
mod init;
mod deploy;
mod config;
mod serve;
mod utils;
mod watch;

use anyhow::Result;
use build::build_site;
use clap::Parser;
use cli::{Cli, Commands};
use config::SiteConfig;
use deploy::deploy_site;
use init::new_site;
use serve::serve_site;
use utils::check::check_required_command_installed;

#[rustfmt::skip]
#[tokio::main]
async fn main() -> Result<()> {
    let cli: &'static Cli = Box::leak(Box::new(Cli::parse()));
    let config: &'static SiteConfig = {
        let root = cli.root.clone().unwrap_or("./".into());
        let config_file = root.join(&cli.config);
        let mut config =
            if config_file.exists() { SiteConfig::from_file(&config_file)? }
            else { SiteConfig::default() };
        config.update_with_cli(cli);
        Box::leak(Box::new(config))
    };

    config.validate(cli)?;
   
    check_required_command_installed(config)?;
       
    match cli.command {
        Commands::Init { .. } => new_site(config)?,
        Commands::Build { .. } => { build_site(config, false)?; },
        Commands::Deploy { .. } => deploy_site(config)?,
        Commands::Serve { .. } => serve_site(config).await?
    };

    Ok(())
}
