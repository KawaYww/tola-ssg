// Fucking
// Fucking
//
// Everyone holds hands to form a circle around `git subprocess` and dances.
//
// Fucking ! !
// Fucking ! ! ! !

use crate::{config::SiteConfig, log, run_command};
use anyhow::{Context, Result, anyhow, bail};
use gix::{
    Repository,
    bstr::BString,
    commit::NO_PARENT_IDS,
    index::{
        State,
        entry::{Flags, Mode, Stat},
        fs::Metadata,
    },
    objs::{Tree, tree},
};
use std::{fs, path::Path};

#[derive(Debug, Default)]
struct Remotes(Vec<Remote>);

#[derive(Debug, Default)]
struct Remote {
    pub name: String,
    pub url: String,
}

impl Remotes {
    #[rustfmt::skip]
    fn new(repo: &Repository) -> Result<Self> {
        let root = repo.path().parent().unwrap();
        let output = run_command!(root; ["git"]; "remote", "-v")?;
        let output = str::from_utf8(&output.stdout)?;

        let remotes = output.lines().filter(|line| line.ends_with("fetch")).map(|line| {
            let parts: Vec<_> = line.split_whitespace().collect();
            let name = parts[0].to_owned();
            let url = parts[1].to_owned();

            Remote { name, url }
        })
        .collect();

        Ok(Self(remotes))
    }

    fn any<P>(&self, p: P) -> bool
    where
        P: Fn(&Remote) -> bool,
    {
        self.0.iter().any(p)
    }
}

pub fn create_repo(root: &Path) -> Result<Repository> {
    let repo = gix::init(root)?;
    Ok(repo)
}

pub fn open_repo(root: &Path) -> Result<Repository> {
    let repo = gix::open(root)?;
    Ok(repo)
}

pub fn commit_all(repo: &Repository, message: &str) -> Result<()> {
    if message.trim().is_empty() {
        bail!("Commit message cannot be empty");
    }

    let root = repo
        .path()
        .parent()
        .ok_or(anyhow!("Invalid repository path"))?;
    let mut index = State::new(repo.object_hash());
    let tree = build_tree_from_dir(root, repo, &mut index)?;
    index.sort_entries();

    let mut file = gix::index::File::from_state(index, repo.index_path());
    file.write(gix::index::write::Options::default())?;

    let tree_id = repo.write_object(&tree)?;
    let commit_id = repo.commit("HEAD", message, tree_id, parent_ids_or_empty(repo)?)?;

    log!("commit"; "commit for blob `{commit_id:?}` in  repo `{}`", root.display());
    Ok(())
}

fn parent_ids_or_empty(repo: &Repository) -> Result<Vec<gix::ObjectId>> {
    let ids = match repo.find_reference("refs/heads/main") {
        Err(_) => NO_PARENT_IDS.to_vec(),
        Ok(refs) => {
            let target = refs.target();
            [target.id().to_owned()].to_vec()
        }
    };
    Ok(ids)
}

#[rustfmt::skip]
pub fn push(repo: &Repository, config: &'static SiteConfig) -> Result<()> {
    let remote_url = config.deploy.github_provider.url.as_str();
    log!("git"; "pushing to `{remote_url}`");

    let root = repo.path().parent().ok_or(anyhow!("Invalid repository path"))?;
    let token_path = config.deploy.github_provider.token_path.as_ref();
    let force = config.deploy.force;
    let remote_url_in_config = config.deploy.github_provider.url.as_str();
    let branch_in_config = config.deploy.github_provider.branch.as_str();

    let token = match token_path {
        None => String::new(),
        Some(token_path) => fs::read_to_string(token_path)
            .unwrap_or_default()
            .trim()
            .to_owned(),
    };

    let split_symbol = if token.is_empty() { "" } else { "@" };
    let remote_url = format!(
        "https://{token}{split_symbol}{}",
        remote_url_in_config
            .strip_prefix("https://")
            .context("the remote url starts without https")
            .unwrap()
    );

    let remote_action = if Remotes::new(repo)?.any(|remote| remote.name == "origin") { "set-url" } else { "add" };
    run_command!(root; ["git"];
        "remote", remote_action, "origin", &remote_url
    )?;
    run_command!(root; ["git"];
        "push", "--set-upstream", "origin", branch_in_config, if force { "-f" } else { "" }
    )?;

    let remote_url_equals_config = Remotes::new(repo)?.any(|remote| remote.name == "origin" && remote.url == remote_url);
    if !remote_url_equals_config && !force { bail!(
        "The url in remote `origin` in repo `{root:?}` not equal to url in [deploy.git], enable [deploy.force] or reset url manually"
    )}
    Ok(())
}

fn build_tree_from_dir(
    root: &Path,
    repo: &Repository,
    index: &mut gix::index::State,
) -> Result<Tree> {
    let mut tree = Tree::empty();

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let filename: BString = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow!("Invalid file name"))?
            .into();

        if path.is_dir() && filename != ".git" {
            let sub_tree = build_tree_from_dir(&path, repo, index)?;
            let tree_id = repo.write_object(&sub_tree)?.detach();

            tree.entries.push(tree::Entry {
                mode: tree::EntryKind::Tree.into(),
                oid: tree_id,
                filename,
            });
        } else if path.is_file() && filename != ".gitignore" {
            let contents = fs::read(&path)?;
            let blob_id = repo.write_blob(contents)?.into();

            index.dangerously_push_entry(
                Stat::from_fs(&Metadata::from_path_no_follow(&path)?)?,
                blob_id,
                Flags::empty(),
                Mode::FILE,
                filename.as_ref(),
            );
            tree.entries.push(tree::Entry {
                mode: tree::EntryKind::Blob.into(),
                oid: blob_id,
                filename,
            });
        }
    }

    tree.entries.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(tree)
}
