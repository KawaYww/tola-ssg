use crate::{
    config::SiteConfig,
    log,
    utils::{
        build::{process_asset, process_content, process_files},
        git, rss,
    },
};
use anyhow::{Context, Result, anyhow};
use gix::{Repository, ThreadSafeRepository};
use std::{ffi::OsStr, fs, thread};

#[rustfmt::skip]
pub fn build_site(config: &'static SiteConfig, should_clear: bool) -> Result<ThreadSafeRepository> {
    let output = &config.build.output;
    let content = &config.build.content;
    let assets = &config.build.assets;

    // Clear output directory and create git repo for deploying
    let repo = match (output.exists(), should_clear) {
        (true, true) => {
            fs::remove_dir_all(output)
                .with_context(|| format!("[build] Failed to clear output directory: {}", output.display()))?;
            git::create_repo(output)?
        },
        (true, false) => match git::open_repo(output) {
            Ok(repo) => repo,
            Err(_) => {
                log!("git"; "{output:?} is not a git repo, creating new now");
                git::create_repo(output)?
            }
        },
        (false, _) => git::create_repo(output)?,
    };


    thread::scope(|s| -> Result<()> {
        // process all posts and relative assets
        let posts_handle = s.spawn(||
            process_files(true, content, config, &|path| path.starts_with(content), &|path, config| process_content(path, config, false))
                .context("Failed to compile all posts")
        );

        // process all assets
        let assets_handle = s.spawn(||
            process_files(false, assets, config, &|_| true, &|path, config| process_asset(path, config, false, false))
                .context("Failed to copy all assets")
        );

        // waiting for finishing
        posts_handle.join().map_err(|e| anyhow!("{e:?}"))??;
        assets_handle.join().map_err(|e| anyhow!("{e:?}"))??;

        Ok(())
    })?;

    let file_num = fs::read_dir(&config.build.output)?
        .flatten()
        .filter(|p| p.file_name() != OsStr::new(".git"))
        .count();

    if file_num == 0 {
        log!("warn"; "output directory is empty, maybe you write nothing or just a single post without `typ` extension?")
    } else {
        log!("build"; "successfully generated site in: {}", config.build.output.display());
    }

    Ok(repo)
}
