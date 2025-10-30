// #![allow(unused)]

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

use crate::utils::rss::build_rss;

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
            (true, true) => bail!("The config file exists, please remove the config file manually or init in other path"),
            (false, false) => bail!("the config file didn't exist"),

            (false, true) => config.validate()?,
        }

        Box::leak(Box::new(config))
    };

    let run_build_tasks = || rayon::join(
        || build_site(config, config.build.clear),
        || build_rss(config)
    );

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
        Commands::Init { .. } => new_site(config)?,
        Commands::Build { .. } => {
            let (build_result, rss_result) = run_build_tasks();
            _ = (build_result?, rss_result?);
        },
        Commands::Deploy { .. } => {
            let (build_result, rss_result) = run_build_tasks();
            let (repo, _) = (build_result?, rss_result?);
            deploy_site(repo, config)?;
        },
        Commands::Serve { .. } => {
            let (build_result, rss_result) = run_build_tasks();
            _ = (build_result?, rss_result?);
            tokio::runtime::Runtime::new()?.block_on(serve_site(config))?;
        },
    };

    Ok(())
}
