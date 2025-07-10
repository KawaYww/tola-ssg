mod builder;
mod cli;
mod initer;
mod deployer;
mod config;
mod server;
mod utils;
mod watcher;

use anyhow::Result;
use builder::build_site;
use clap::Parser;
use cli::Cli;
use config::SiteConfig;
use deployer::deploy_site;
use initer::new_site;
use server::serve_site;
use utils::checker::check_required_command_installed;

#[rustfmt::skip]
#[tokio::main]
async fn main() -> Result<()> {
    let cli: &'static Cli = Box::leak(Box::new(Cli::parse()));
    let config: &'static SiteConfig = {
        let config_file = cli.root.join(&cli.config);
        let config =
            if config_file.exists() { SiteConfig::from_file(&config_file)? }
            else { SiteConfig::default() }
            .update_with_cli(cli);
        Box::leak(Box::new(config))
    };

    check_typst_installed()?;
    utils::check_typst_installed()?;
    
    check_required_command_installed(config)?;
    
    if cli.command_is_init() { new_site(&config.build.root_path)?; }
    if cli.command_is_built() { build_site(config)?; }
    if cli.command_is_deploy() { deploy_site(config)?; }
    if cli.command_is_serve() { serve_site(config).await?; }

    Ok(())
}
