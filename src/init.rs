//! Site initialization module.
//!
//! Creates new site structure with default configuration.

use crate::{config::SiteConfig, utils::git};
use anyhow::{Context, Result, bail};
use inquire::{Text, validator::Validation};
use std::{fs, path::Path};

/// Files to write ignore patterns to
const IGNORE_FILES: &[&str] = &[".gitignore", ".ignore"];

/// Default config filename
const CONFIG_FILE: &str = "tola.toml";

/// Default site directory structure
const SITE_DIRS: &[&str] = &[
    "content",
    "assets/images",
    "assets/iconfonts",
    "assets/fonts",
    "assets/scripts",
    "assets/styles",
    "templates",
    "utils",
];

/// User input collected from interactive prompts
struct SiteInfo {
    title: String,
    description: String,
    author: String,
    email: String,
    url: Option<String>,
}

/// Run interactive prompts to collect site information
fn prompt_site_info() -> Result<SiteInfo> {
    let title = Text::new("Site title:")
        .with_help_message("The title of your site")
        .with_validator(|input: &str| {
            if input.trim().is_empty() {
                Ok(Validation::Invalid("Title cannot be empty".into()))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt()?;

    let description = Text::new("Site description:")
        .with_help_message("A brief description of your site")
        .with_validator(|input: &str| {
            if input.trim().is_empty() {
                Ok(Validation::Invalid("Description cannot be empty".into()))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt()?;

    let author = Text::new("Author name:")
        .with_help_message("Your name")
        .with_default(&SiteConfig::default().base.author)
        .prompt()?;

    let email = Text::new("Author email:")
        .with_help_message("Your email address")
        .with_default(&SiteConfig::default().base.email)
        .prompt()?;

    let url = Text::new("Base URL (optional):")
        .with_help_message("Your site's base URL (e.g., https://example.com)")
        .with_validator(|input: &str| {
            if input.is_empty()
                || input.starts_with("http://")
                || input.starts_with("https://")
            {
                Ok(Validation::Valid)
            } else {
                Ok(Validation::Invalid("URL must start with http:// or https://".into()))
            }
        })
        .prompt()?;

    Ok(SiteInfo {
        title,
        description,
        author,
        email,
        url: if url.is_empty() { None } else { Some(url) },
    })
}

/// Create a new site with default structure
pub fn new_site(config: &'static SiteConfig) -> Result<()> {
    let root = config.get_root();

    let site_info = prompt_site_info()?;

    let repo = git::create_repo(root)?;
    init_site_structure(root)?;
    init_config_with_info(root, &site_info)?;
    init_ignored_files(root, &[config.build.output.as_path(), Path::new("/assets/images/")])?;
    git::commit_all(&repo, "initial commit")?;

    Ok(())
}

/// Write configuration file with user-provided information
fn init_config_with_info(root: &Path, info: &SiteInfo) -> Result<()> {
    let mut site_config = SiteConfig::default();
    site_config.base.title = info.title.clone();
    site_config.base.description = info.description.clone();
    site_config.base.author = info.author.clone();
    site_config.base.email = info.email.clone();
    site_config.base.url = info.url.clone();

    let content = toml::to_string_pretty(&site_config)?;
    fs::write(root.join(CONFIG_FILE), content)?;
    Ok(())
}

/// Create site directory structure
fn init_site_structure(root: &Path) -> Result<()> {
    for dir in SITE_DIRS {
        let path = root.join(dir);
        if path.exists() {
            bail!(
                "Path `{}` already exists. Try `tola init <SITE_NAME>` instead.",
                path.display()
            );
        }
        fs::create_dir_all(&path).with_context(|| format!("Failed to create {}", path.display()))?;
    }
    Ok(())
}

/// Initialize .gitignore and .ignore files with specified paths
pub fn init_ignored_files(root: &Path, paths: &[&Path]) -> Result<()> {
    let content = paths
        .iter()
        .filter_map(|p| p.to_str())
        .collect::<Vec<_>>()
        .join("\n");

    for filename in IGNORE_FILES {
        let path = root.join(filename);
        if !path.exists() {
            fs::write(&path, &content)?;
        }
    }

    Ok(())
}
