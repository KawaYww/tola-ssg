use std::{fs, path::{Path, PathBuf}};
use anyhow::{Context, Result};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use crate::{config::SiteConfig, utils::git};

// default ignored path
const IGNORES: &[&str] = &[
    ".gitignore",
    ".ignore",
];

// default config path
const CONFIG: &str = "tola.toml";

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

pub fn new_site(config: &'static SiteConfig) -> Result<()> {
    let root = &config.get_root();
   
    let repo = git::create_repo(root)?;       
    init_default_config(root)?;
    init_site_structure(root)?;
    init_ignore_files(root, &[config.build.output.as_path()])?;
    git::commit_all(&repo, "initial commit")?;
  
    Ok(())
}


fn init_default_config(root: &Path) -> Result<()> {
    let default_site_config = SiteConfig::default();   
    let content = toml::to_string_pretty(&default_site_config)?;
    let config_path = root.join(CONFIG);
    fs::write(config_path, content)?;

    Ok(())
}

fn init_site_structure(root: &Path) -> Result<()> {
    DIRS.par_iter().try_for_each(|path| {
        let path = root.join(path);
        fs::create_dir_all(&path)
    })?;
    Ok(())
}

fn init_ignore_files(root: &Path, ignore_paths: &[&Path]) -> Result<()> {
    let ignore_paths = ignore_paths.iter().try_fold(String::new(),|sum, path| -> Result<String> {
        let path = path.strip_prefix(root)
            .with_context(|| format!("Failed to strip suffix: path: {path:?}, root: {root:?}"))?;
        let path = PathBuf::from("/").join(path);
        let path = path.to_str()
            .with_context(|| format!("Failed to convert this path({path:?}) to str"))?;
        Ok(sum + path + "\n")
    })?;
    
    IGNORES.par_iter().try_for_each(|path| {
        let path = root.join(path);
        fs::write(path, ignore_paths.as_str())
    })?;
    
    Ok(())
}
