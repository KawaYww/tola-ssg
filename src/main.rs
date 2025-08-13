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

    let run_rss_task = || {
        if config.build.rss.enable && !cli.is_init() {
            let rss_xml = crate::utils::rss::RSSChannel::new(config)?;
            rss_xml.write_to_file(config)?;
        }
        Ok::<(), anyhow::Error>(())
    };

    fn run_with_local_pool<F, R>(thread_percantage: f32, f: F) -> R
    where
        F: FnOnce() -> R + Send,
        R: Send,
    {
        let max_threads = rayon::current_num_threads();
        let threads = (max_threads as f32 * thread_percantage).ceil() as usize;
        ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(f)
    }

    match cli.command {
        Commands::Init { .. } => {
            new_site(config)?;
        },
        Commands::Build { .. } => {
            std::thread::scope(|s| -> Result<()> {
                let build_task = s.spawn(|| run_with_local_pool(0.8, || build_site(config, config.build.clear)));
                let rss_task = s.spawn(|| run_with_local_pool(0.2, run_rss_task));
                build_task.join().unwrap();
                rss_task.join().unwrap();
                Ok(())
            })?;
        },
        Commands::Deploy { .. } => {
            std::thread::scope(|s| -> Result<()> {
                let build_task = s.spawn(|| build_site(config, config.build.clear));
                let rss_task = s.spawn(run_rss_task);
                let repo = build_task.join().unwrap()?;
                rss_task.join().unwrap();
                deploy_site(repo, config);
                Ok(())
            })?;
        },
        Commands::Serve { .. } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(serve_site(config))?;
        },
    };

    Ok(())
}
