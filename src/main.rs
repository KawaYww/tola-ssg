mod build;
mod cli;
mod config;
mod deploy;
mod init;
mod serve;
mod utils;
mod watch;

use anyhow::{Result, bail};
use build::build_site;
use clap::Parser;
use cli::{Cli, Commands};
use config::SiteConfig;
use deploy::deploy_site;
use init::new_site;
use serve::serve_site;
use std::path::Path;

#[rustfmt::skip]
#[tokio::main]
async fn main() -> Result<()> {
    let cli: &'static Cli = Box::leak(Box::new(Cli::parse()));

    let config: &'static SiteConfig = {
        let root = cli.root.as_deref().unwrap_or(Path::new("./"));
        let config = root.join(&cli.config);
        let mut config =
            if config.exists() { SiteConfig::from_file(&config)? }
            else { SiteConfig::default() };
        config.update_with_cli(cli);

        let config_exists = config.get_root().join(cli.config.as_path()).exists();
        match (cli.is_init(), config_exists) {
            (true, false) => (),
            (true, true) => bail!("the config file exists, please remove the config file manually or init in other path"),
            (false, false) => bail!("the config file didn't exist"),
            (false, true) => config.validate()?,
        }

        Box::leak(Box::new(config))
    };

    match cli.command {
        Commands::Init { .. } => new_site(config)?,
        Commands::Build { .. } => { build_site(config, config.build.clear)?; },
        Commands::Deploy { .. } => deploy_site(config)?,
        Commands::Serve { .. } => serve_site(config).await?
    };

    Ok(())
}
