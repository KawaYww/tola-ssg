use crate::{
    config::SiteConfig,
    log,
    utils::{
        build::{process_asset, process_content, process_files},
        git,
    },
};
use anyhow::{Context, Result};
use gix::ThreadSafeRepository;
use std::{ffi::OsStr, fs};

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

    let (posts_result, assets_result) = rayon::join(
        || process_files(&crate::utils::build::CONTENT_CACHE, content, config, &|path| path.starts_with(content), &|path, config| process_content(path, config, false))
            .context("Failed to compile all posts"),
        || process_files(&crate::utils::build::ASSETS_CACHE, assets, config, &|_| true, &|path, config| process_asset(path, config, false, false))
            .context("Failed to copy all assets")
    );
    _ = (posts_result?, assets_result?);

    let file_num = fs::read_dir(&config.build.output)?
        .flatten()
        .filter(|p| p.file_name() != OsStr::new(".git"))
        .count();

    if file_num == 0 {
        log!("warn"; "Output directory is empty, maybe you write nothing or just a single post without `typ` extension?")
    } else {
        log!("build"; "Successfully generated site in: {}", config.build.output.display());
    }

    Ok(repo)
}
