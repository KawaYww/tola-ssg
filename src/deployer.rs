use anyhow::Result;
use crate::config::SiteConfig;

pub fn deploy_site(config: &'static SiteConfig) -> Result<()> {
    let root = &config.build.root_path;
    let _repo = gix::init(root)?;
    
    Ok(())
}

