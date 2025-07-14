use crate::{config::SiteConfig, log, utils::{build::{process_post, process_asset, process_files}, git}};
use anyhow::{anyhow, Context, Result};
use gix::Repository;
use std::{ffi::OsStr, fs, thread};

#[rustfmt::skip]
pub fn build_site(config: &'static SiteConfig, should_clear: bool) -> Result<Repository> {
    let output_dir = &config.build.output_dir;
    let content_dir = &config.build.content_dir;
    let assets_dir = &config.build.assets_dir;

    // Clear output directory and create git repo for deploying
    let repo = match (output_dir.exists(), should_clear) {
        (true, true) => {
            fs::remove_dir_all(output_dir)
                .with_context(|| format!("[builder] Failed to clear output directory: {}", output_dir.display()))?;
            git::create_repo(output_dir)?
        },
        (true, false) => git::open_repo(output_dir)?,

        (false, _) => git::create_repo(output_dir)?,
    };


    let repo = thread::scope(|s| -> Result<Repository> {
        // process all posts
        let posts_handle = s.spawn(|| 
            process_files(content_dir,  config, &|path| path.extension().is_some_and(|ext| ext == "typ"), &process_post)
                .context("Failed to compile all posts")
        );

        // process all assets
        let assets_handle = s.spawn(|| 
            process_files(assets_dir,  config, &|_| true, &|path, config| process_asset(path, config, false))
                .context("Failed to copy all assets")
        );

        // waiting for finishing
        posts_handle.join().map_err(|e| anyhow!("{e:?}"))??;
        assets_handle.join().map_err(|e| anyhow!("{e:?}"))??;

        Ok(repo)
    })?;

    let output_dir = fs::read_dir(&config.build.output_dir)?;
    let file_num = output_dir.into_iter()
        .filter_map(|p| p.ok())
        .filter(|p| p.file_name() != OsStr::new(".git"))
        .count();
    if file_num == 0 {
        log!("warn", "output directory is empty, maybe you write nothing or just a single post without `typ` extension?")
    } else {
        log!("build", "successfully generated site in: {}", config.build.output_dir.display());
    }

    Ok(repo)
}
