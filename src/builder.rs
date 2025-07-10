use crate::{config::SiteConfig, log, utils::{builder::{compile_post, copy_asset, process_files}, git}};
use anyhow::{anyhow, Context, Result};
use std::{fs, thread};

#[rustfmt::skip]
pub fn build_site(config: &'static SiteConfig) -> Result<()> {
    let output_dir = &config.build.output_dir;
    let content_dir = &config.build.content_dir;
    let assets_dir = &config.build.assets_dir;

    // Clear output directory and create git repo for deploying
    if output_dir.exists() {
        fs::remove_dir_all(output_dir)
            .with_context(|| format!("[Builder] Failed to clear output directory: {}", output_dir.display()))?;

        git::create_repo(output_dir)?;
    }

    // Process files in parallel
    thread::scope(|s| {
        // Process all posts
        let posts_handle = s.spawn(|| 
            process_files(content_dir,  config, &|path| path.extension().is_some_and(|ext| ext == "typ"), &compile_post)
                .context("Failed to compile all posts")
        );

        // Copy assets
        let assets_handle = s.spawn(|| 
            process_files(assets_dir,  config, &|_| true, &|path, config| copy_asset(path, config, false))
                .context("Failed to copy all assets")
        );

        // waiting for finishing
        posts_handle.join().map_err(|e| anyhow!("{e:?}"))??;
        assets_handle.join().map_err(|e| anyhow!("{e:?}"))??;

        log!("builder",
            "Successfully generated site in: {}", config.build.output_dir.display()
        );

        Ok(())
    })
}
