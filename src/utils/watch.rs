use super::build::{process_asset, process_content};
use crate::{config::SiteConfig, run_command};
use anyhow::{Context, Result, anyhow, bail};
use rayon::prelude::*;
use std::{
    env, fs,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

pub fn process_watched_content(files: &[&PathBuf], config: &'static SiteConfig) -> Result<()> {
    let flag = config.get_root().starts_with("./");

    files.par_iter().try_for_each(|path| {
        let path = path.strip_prefix(env::current_dir().unwrap()).unwrap();
        let path = if flag {
            &Path::new("./").join(path)
        } else {
            path
        };

        match process_content(path, config, true) {
            Ok(Some(handle)) => handle.join().ok().context(""),
            Ok(None) => Ok(()),
            Err(e) => Err(e),
        }
    })?;

    if config.build.tailwind.enable {
        let input = config.build.tailwind.input.as_ref().unwrap();
        let output = config.build.output.as_path();
        let relative_asset_path = input
            .strip_prefix(config.build.assets.as_path())?
            .to_str()
            .ok_or(anyhow!("Invalid path"))?;
        let input = input.canonicalize().unwrap();
        let output_path = output.canonicalize().unwrap().join(relative_asset_path);

        run_command!(config.get_root(); &config.build.tailwind.command;
            "-i", input, "-o", output_path, if config.build.minify { "--minify" } else { "" }
        )?;
    }

    Ok(())
}

pub fn process_watched_assets(
    files: &[&PathBuf],
    config: &'static SiteConfig,
    should_wait_until_stable: bool,
) -> Result<()> {
    let flag = config.get_root().starts_with("./");

    files
        .par_iter()
        .filter(|path| path.exists())
        .try_for_each(|path| {
            let path = path.strip_prefix(env::current_dir().unwrap()).unwrap();
            let path = if flag {
                &Path::new("./").join(path)
            } else {
                path
            };
            match process_asset(path, config, should_wait_until_stable, true) {
                Ok(Some(handle)) => handle.join().ok().context(""),
                Ok(None) => Ok(()),
                Err(e) => Err(e),
            }
        })?;

    Ok(())
}

#[rustfmt::skip]
pub fn process_watched_files(files: &[PathBuf], config: &'static SiteConfig) -> Result<()> {
    let posts_files: Vec<_> = files
        .par_iter()
        .filter(|path| path.exists() && path.extension().and_then(|s| s.to_str()) == Some("typ"))
        .collect();

    let flag = config.get_root().starts_with("./");
    let assets_files: Vec<_> = files
        .par_iter()
        .filter(|path|  {
            let path = path.strip_prefix(env::current_dir().unwrap()).unwrap();
            let path = if flag {
                &Path::new("./").join(path)
            } else {
                path
            };
            path.starts_with(&config.build.assets)
        })
        .collect();

    if !posts_files.is_empty() { process_watched_content(&posts_files, config)? }
    if !assets_files.is_empty() { process_watched_assets(&assets_files, config, true)? }

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
