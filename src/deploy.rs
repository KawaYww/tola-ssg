use crate::{config::SiteConfig, utils::git};
use anyhow::{Result, bail};
use gix::ThreadSafeRepository;

pub fn deploy_site(repo: ThreadSafeRepository, config: &'static SiteConfig) -> Result<()> {
    match config.deploy.provider.as_str() {
        "github" => deploy_github(repo, config),
        _ => bail!("This platform is not supported now"),
    }
}

fn deploy_github(repo: ThreadSafeRepository, config: &'static SiteConfig) -> Result<()> {
    git::commit_all(&repo, "deploy it")?;
    git::push(&repo, config)?;
    Ok(())
}
