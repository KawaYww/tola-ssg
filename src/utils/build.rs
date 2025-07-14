use std::{fs, path::{Path, PathBuf}};
use anyhow::{anyhow, Context, Result};
use crate::{config::SiteConfig, log, run_command};
use crate::utils::watch::wait_until_stable;
use rayon::prelude::*;

pub fn _copy_dir_recursively(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst).context("[Utils] Failed to create destination directory")?;
    }

    for entry in fs::read_dir(src).context("[Utils] Failed to read source directory")? {
        let entry = entry.context("[Utils] Invalid directory entry")?;
        let entry_path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if entry_path.is_dir() {
            _copy_dir_recursively(&entry_path, &dest_path)?;
        } else {
            fs::copy(&entry_path, &dest_path).with_context(|| {
                format!("[Utils] Failed to copy {entry_path:?} to {dest_path:?}")
            })?;

            log!("assets", "{}", dest_path.display());
        }
    }

    Ok(())
}

pub fn process_files<P, F>(dir: &Path, config: &SiteConfig, p: &P, f: &F) -> Result<()>
where
    P: Fn(&PathBuf) -> bool + Sync,
    F: Fn(&Path, &SiteConfig) -> Result<()> + Sync,
{   
    fs::read_dir(dir)?
        .collect::<Vec<_>>()
        .par_iter()
        .flatten()
        .map(|entry| entry.path())
        .try_for_each(|path| {
            if path.is_dir() {
                process_files(&path, config, p, f)
            } else if path.is_file() && p(&path) {
                f(&path, config)
            } else {
                Ok(())
            }
        })
}

pub fn process_post(post_path: &Path, config: &SiteConfig) -> Result<()> {
    let root = &config.get_root();
    
    let content_dir = &config.build.content_dir;
    let output_dir = &config.build.output_dir;

    let relative_post_path = post_path
        .strip_prefix(content_dir)?
        .to_str()
        .ok_or(anyhow!("Invalid path"))?
        .strip_suffix(".typ")
        .ok_or(anyhow!("Not a .typ file"))?;

    let output_path = output_dir.join(relative_post_path);
    fs::create_dir_all(&output_path)?;

    let html_path = if post_path.file_name().is_some_and(|p| p == "home.typ") {
        config.build.output_dir.join("index.html")
    } else {
        output_path.join("index.html")
    };

    run_command!(&config.build.typst_command;
        "compile", "--features", "html", "--format", "html",
        "--font-path", root, "--root", root,
        post_path, &html_path
    )?;

    if config.build.minify {
        let html_content = fs::read_to_string(&html_path)?;
        let minified_content = minify_html::minify(html_content.as_bytes(), &minify_html::Cfg::new());
        let content = String::from_utf8_lossy(&minified_content).to_string();
        fs::write(&html_path, content)?;
    }

    log!("content", "{}", relative_post_path);

    Ok(())
}

pub fn process_asset(asset_path: &Path, config: &SiteConfig, should_wait_until_stable: bool) -> Result<()> {
    let assets_dir = &config.build.assets_dir;
    let output_dir = &config.build.output_dir;

    match asset_path.extension().unwrap_or_default().to_str().unwrap_or_default() {
        "css" if config.tailwind.enable => {
            println!("{asset_path:?}")
            // Command::new(config.tailwind.command[0].as_str());
        },
        _ => (),
    }

    let relative_asset_path = asset_path
        .strip_prefix(assets_dir)?
        .to_str()
        .ok_or(anyhow!("Invalid path"))?;

    PathBuf::from(relative_asset_path).extension().unwrap_or_default();

    let output_path = output_dir.join(relative_asset_path);

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if output_path.exists() {
        fs::remove_file(&output_path)?;
    }

    if should_wait_until_stable {
        wait_until_stable(asset_path, 5)?;
    }
    fs::copy(asset_path, &output_path)?;

    log!("assets", "{}", relative_asset_path);

    Ok(())
}



