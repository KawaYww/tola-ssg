use std::{env, fs, path::{Path, PathBuf}, thread, time::Duration};
use anyhow::{bail, Result};
use crate::config::SiteConfig;
use super::build::{process_post, process_asset};
use rayon::prelude::*;

pub fn process_posts_in_parallel(files: &[&PathBuf], config: &SiteConfig) -> Result<()> {
    files.par_iter().try_for_each(|path| process_post(path, config))
}

pub fn process_assets_in_parallel(files: &[&PathBuf], config: &SiteConfig, should_wait_until_stable: bool) -> Result<()> {
    files.par_iter().try_for_each(|path| process_asset(path, config, should_wait_until_stable))
}

#[rustfmt::skip]
pub fn process_watched_files(files: &[PathBuf], config: &SiteConfig) -> Result<()> {
    let posts_files: Vec<_> = files
        .par_iter()
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("typ"))
        .collect();

    let assets_files: Vec<_> = files
        .par_iter()
        .filter(|path|  path
            .strip_prefix(env::current_dir().unwrap())
            .unwrap()
            .starts_with(&config.build.assets)
        )
        .collect();

    if !posts_files.is_empty() { process_posts_in_parallel(&posts_files, config)? }
    if !assets_files.is_empty() { process_assets_in_parallel(&assets_files, config, true)? }

    Ok(())
}

#[rustfmt::skip]
pub fn wait_until_stable(path: &Path, max_retries: usize) -> Result<()> {
    let mut last_size = fs::metadata(path)?.len();
    let mut retries = 0;
    let timeout = Duration::from_millis(50);
    
    while retries < max_retries {
        thread::sleep(timeout);
        let current_size = fs::metadata(path)?.len();
        if current_size == last_size { return Ok(()) }
        last_size = current_size;
        retries += 1;
    }

    bail!("File did not stabilize after {} retries", max_retries);
}

