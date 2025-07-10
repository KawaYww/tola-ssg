use anyhow::{bail, Result};
use crate::{builder::build_site, config::SiteConfig, utils::git};

pub fn deploy_site(config: &'static SiteConfig) -> Result<()> {
    match config.deploy.provider.as_str() {
        "github" => deploy_github(config),
        _ => bail!("This platform is not supported now")
    }
}

fn deploy_github(config: &'static SiteConfig) -> Result<()> {
    let output_dir = &config.build.output_dir;

    let repo = if !output_dir.exists() {
        build_site(config)?
    } else {
        git::open_repo(output_dir)?
    };

    git::commit_all(&repo, "deploy it")?;
    git::push(&repo, config)?;
    
    Ok(())
}
