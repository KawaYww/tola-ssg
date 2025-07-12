use anyhow::{bail, Result};
use crate::{builder::build_site, config::SiteConfig, utils::git};

pub fn deploy_site(config: &'static SiteConfig) -> Result<()> {
    match config.deploy.provider.as_str() {
        "github" => deploy_github(config),
        _ => bail!("This platform is not supported now")
    }
}

fn deploy_github(config: &'static SiteConfig) -> Result<()> {
    let repo = build_site(config, config.deploy.force)?;
    git::commit_all(&repo, "deploy it")?;
    git::push(&repo, config)?;
    
    Ok(())
}
