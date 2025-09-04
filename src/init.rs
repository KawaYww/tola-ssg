use crate::{config::SiteConfig, utils::git};
use anyhow::{Context, Result, bail};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{fs, path::Path};

// default ignored path
const IGNORE_FILES: &[&str] = &[".gitignore", ".ignore"];

// default config path
const CONFIG: &str = "tola.toml";

// default site structure
const DIRS: &[&str] = &[
    "content",
    "assets/images",
    "assets/iconfonts",
    "assets/fonts",
    "assets/scripts",
    "assets/styles",
    "templates",
    "utils",
];

pub fn new_site(config: &'static SiteConfig) -> Result<()> {
    let root = config.get_root();

    let repo = git::create_repo(root)?;
    init_default_config(root)?;
    init_site_structure(root)?;
    init_ignore_files(
        root,
        &[config.build.output.as_path(), Path::new("/assets/images/")],
    )?;
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
        if path.exists() {
            bail!(
                "there already has path `{}` when you init site",
                path.display()
            )
        } else {
            fs::create_dir_all(&path).context("")
        }
    })?;
    Ok(())
}

#[rustfmt::skip]
pub fn init_ignore_files(root: &Path, paths_should_ignore: &[&Path]) -> Result<()> {
    // println!("root: {:?}, {:?}", root, paths_should_ignore);

    let paths_should_ignore = paths_should_ignore.iter()
        .try_fold(String::new(), |sum, path| -> Result<String> {
            // let path = path.strip_prefix(root).with_context(|| format!("Failed to strip suffix: path: {path:?}, root: {root:?}"))?;
            // let path = PathBuf::from("/").join(path);
            let path = path.to_str().with_context(|| format!("Failed to convert this path({path:?}) to str"))?;
            Ok(sum + path + "\n")
        })?;

    // println!("{:?}", IGNORE_FILES);
    IGNORE_FILES.par_iter().try_for_each(|path| {
        let path = root.join(path);
        if path.exists() {
            bail!(
                "there already has path `{}` when you init site",
                path.display()
            )
        } else {
            // println!("ignore file: {:?}, {:?}", path, paths_should_ignore);
            fs::write(path, paths_should_ignore.as_str()).context("")
        }
    })?;

    Ok(())
}
