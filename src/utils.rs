use crate::{
    config::SiteConfig, log
};
use anyhow::{bail, Context, Result};
use crossterm::{
    cursor::MoveTo,
    terminal::{Clear, ClearType},
};
use minify_html::{Cfg, minify};
use rayon::prelude::*;
use std::{
    env,
    fs::{self, create_dir_all},
    io::stdout,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};

pub fn check_typst_installed() -> Result<()> {
    Command::new("typst")
        .arg("--version")
        .output()
        .map(|_| ())
        .context("[Utils] Typst not found. Please install Typst first.")
}

pub fn _clear_screen() -> Result<()> {
    crossterm::execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))
        .context("[Utils] Failed to clear screen")
}

pub fn _copy_dir_recursively(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        create_dir_all(dst).context("[Utils] Failed to create destination directory")?;
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
        .try_for_each(|entry| {
            let path = entry.path();
            if path.is_dir() {
                process_files(&path, config, p, f)
            } else if path.is_file() && p(&path) {
                f(&path, config)
            } else {
                Ok(())
            }
        })
}

pub fn process_posts_in_parallel(files: &[&Path], config: &SiteConfig) -> Result<()> {
    files
        .par_iter()
        .try_for_each(|path| compile_post(path, config))
}

pub fn copy_assets_in_parallel(files: &[&Path], config: &SiteConfig, should_wait_until_stable: bool) -> Result<()> {
    files.par_iter().try_for_each(|path| copy_asset(path, config, should_wait_until_stable))
}

pub fn process_watched_files(files: &[PathBuf], config: &SiteConfig) -> Result<()> {
    let posts_files: Vec<_> = files
        .par_iter()
        .filter(|p| p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("typ"))
        .map(|p| p.as_path())
        .collect();

    // println!("Before");

    let assets_files: Vec<_> = files
        .par_iter()
        // .inspect(|x| println!("{:?}", x))
        .filter(|p| {
            p.is_file()
                && p.strip_prefix(env::current_dir().unwrap())
                    .unwrap()
                    .starts_with(&config.build.assets_dir)
        })
        .map(|p| p.as_path())
        .collect();

    // println!("{:?}", assets_files);

    if !posts_files.is_empty() {
        process_posts_in_parallel(&posts_files, config)?;
    }

    if !assets_files.is_empty() {
        copy_assets_in_parallel(&assets_files, config, true)?;
    }

    Ok(())
}

pub fn compile_post(path: &Path, config: &SiteConfig) -> Result<()> {
    let content_dir = &config.build.content_dir;
    let output_dir = &config.build.output_dir;

    let relative_path = path
        .strip_prefix(content_dir)?
        .to_str()
        .ok_or(anyhow::anyhow!("Invalid path"))?
        .strip_suffix(".typ")
        .ok_or(anyhow::anyhow!("Not a .typ file"))?;

    let output_path = output_dir.join(relative_path);
    create_dir_all(&output_path)?;

    let html_path = if path.file_name().is_some_and(|p| p == "home.typ") {
        config.build.output_dir.join("index.html")
    } else {
        output_path.join("index.html")
    };

    let output = Command::new("typst")
        .args(["compile", "--features", "html", "--format", "html"])
        .arg("--font-path")
        .arg(&config.build.root_path)
        .arg("--root")
        .arg(&config.build.root_path)
        .arg(path)
        .arg(&html_path)
        .output()?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to compile {}: {}", path.display(), error_msg);
    }

    if config.build.minify {
        let html_content = fs::read_to_string(&html_path)?;
        let minified_content = minify(html_content.as_bytes(), &Cfg::new());
        let content = String::from_utf8_lossy(&minified_content).to_string();
        fs::write(&html_path, content)?;
    }


    Ok(())
}

pub fn copy_asset(path: &Path, config: &SiteConfig, should_wait_until_stable: bool) -> Result<()> {
    let assets_dir = &config.build.assets_dir;
    let output_dir = &config.build.output_dir;

    let relative_path = path
        .strip_prefix(assets_dir)?
        .to_str()
        .ok_or(anyhow::anyhow!("Invalid path"))?;

    let output_path = output_dir.join(relative_path);

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if output_path.exists() {
        fs::remove_file(&output_path)?;
    }

    if should_wait_until_stable {
        wait_until_stable(path, 5)?;
    }
    fs::copy(path, &output_path)?;


    Ok(())
}

fn wait_until_stable(path: &Path, max_retries: usize) -> Result<()> {
    let mut last_size = fs::metadata(path)?.len();
    let mut retries = 0;
    let timeout = Duration::from_millis(50);
    
    while retries < max_retries {
        thread::sleep(timeout);
        let current_size = fs::metadata(path)?.len();
        if current_size == last_size {
            return Ok(());
        }
        last_size = current_size;
        retries += 1;
    }

    bail!("File did not stabilize after {} retries", max_retries);
}
