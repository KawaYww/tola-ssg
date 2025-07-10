use std::{fs, path::Path};
use anyhow::Result;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use crate::{config::SiteConfig, utils::git};

// default config path
const CONFIG_PATH: &str = "tola.toml";

// default site structure
const DIRS: &[&str] = &[
    "content",
    "assets",
    "assets/images",
    "assets/iconfonts",
    "assets/fonts",
    "assets/scripts",
    "assets/styles",
    "templates",
    "utils",
];

pub fn new_site(root: &Path) -> Result<()> {
    let repo = git::create_repo(root)?;       

    init_default_config(root)?;
    init_site_structure(root)?;

    git::commit_all(&repo, "initial commit")?;
  
    Ok(())
}


fn init_default_config(root: &Path) -> Result<()> {
    let default_site_config = SiteConfig::default();   
    let content = toml::to_string_pretty(&default_site_config)?;
    let config_path = root.join(CONFIG_PATH);
    fs::write(config_path, content)?;

    Ok(())
}

fn init_site_structure(root: &Path) -> Result<()> {
    DIRS.par_iter().try_for_each(move |path| {
        let path = root.join(path);
        fs::create_dir_all(&path)
    })?;
    Ok(())
}

