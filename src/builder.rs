use crate::{config::SiteConfig, log, utils::{self, compile_post, copy_asset}};
use anyhow::{anyhow, Context, Result};
use std::{fs, thread};

pub fn build_site(config: &'static SiteConfig) -> Result<()> {
    // Clear output directory
    if config.build.output_dir.exists() {
        fs::remove_dir_all(&config.build.output_dir).with_context(|| {
            format!(
                "[Builder] Failed to clear output directory: {}",
                config.build.output_dir.display()
            )
        })?;
    }

    thread::scope(|s| {
        // Process all posts
        let posts_handle = s.spawn(|| {
            utils::process_files(&config.build.content_dir,  config, &|path| path.extension().is_some_and(|ext| ext == "typ"), &compile_post)
                .context("Failed to compile all posts")
        });

        // Copy assets
        let assets_handle = s.spawn(|| {
            utils::process_files(&config.build.assets_dir,  config, &|_| true, &|path, config| copy_asset(path, config, false))
                .context("Failed to copy all assets")
        });

        // waiting for finishing
        posts_handle.join().map_err(|e| anyhow!("{e:?}"))??;
        assets_handle.join().map_err(|e| anyhow!("{e:?}"))??;

        log!(
            "builder",
            "Successfully generated site in: {}",
            config.build.output_dir.display()
        );

        Ok(())
    })
}
