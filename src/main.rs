#![allow(unused)]

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
use rayon::ThreadPoolBuilder;
use serve::serve_site;
use std::path::Path;

#[rustfmt::skip]
fn main() -> Result<()> {
    let cli: &'static Cli = Box::leak(Box::new(Cli::parse()));

    let config: &'static SiteConfig = {
        let root = cli.root.as_deref().unwrap_or(Path::new("./"));
        let config_file = root.join(&cli.config);
        let mut config =
            if config_file.exists() { SiteConfig::from_path(&config_file)? }
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

    let run_rss_task = || {
        if config.build.rss.enable && !cli.is_init() {
            let rss_xml = crate::utils::rss::RSSChannel::new(config)?;
            rss_xml.write_to_file(config)?;
        }
        Ok::<(), anyhow::Error>(())
    };

    // fn handle_error<T, BODY>(body: BODY) -> T
    // where
    //     BODY: FnOnce() -> Result<T> + Send + 'static,
    // {
    //     match body() {
    //         Ok(t) => t,
    //         Err(err) => {
    //             eprintln!("Error: {}", err);
    //             std::process::exit(1);
    //         }
    //     }
    // }

    match cli.command {
        Commands::Init { .. } => {
            new_site(config)?;
        },
        Commands::Build { .. } => {
            let (build_handle, rss_handle) = rayon::join(
                || build_site(config, config.build.clear),
                run_rss_task
            );
            build_handle?;
            rss_handle?;
        },
        Commands::Deploy { .. } => {
            let (repo, rss_handle) = rayon::join(
                || build_site(config, config.deploy.force),
                run_rss_task
            );
            rss_handle?;
            let repo = repo?;
            deploy_site(repo, config)?;
        },
        Commands::Serve { .. } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(serve_site(config))?;
        },
    };

    Ok(())
}
