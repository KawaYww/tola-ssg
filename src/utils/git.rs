// Fucking 
// Fucking
// 
// Everyone holds hands to form a circle around `git subprocess` and dances.
//
// Fucking ! !
// Fucking ! ! ! !

use std::{fs, io::BufWriter, path::Path, process::Command};
use anyhow::{anyhow, bail, Context, Result};
use gix::{bstr::BString, commit::NO_PARENT_IDS, index::{entry::{Flags, Mode, Stat}, fs::Metadata, State}, objs::{tree, Tree}, Repository};
use crate::{config::SiteConfig, log};

#[derive(Debug, Default)]
struct Remote {
    name: String,
    url: String,
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
    let root = repo.path().parent().unwrap();
    let mut index = State::new(repo.object_hash());
    let tree = build_tree_from_dir(root, repo, &mut index)?;
    index.sort_entries();

    let mut buffer = {
        let file = fs::OpenOptions::new().write(true).create(true).truncate(true).open(repo.index_path())?;
        BufWriter::new(file)
    };
    index.write_to(&mut buffer, gix::index::write::Options::default())?;

    let tree_id = repo.write_object(&tree)?;
    let commit_id = repo.commit(
        "HEAD",
        message,
        tree_id,
        // gix::commit::NO_PARENT_IDS
        parent_ids_or_empty(repo)?
    )?;
    
    log!("commit", "In repo `{}`, commit id for blob: {commit_id:?}", root.display());
    
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

pub fn push(repo: &Repository, config: &'static SiteConfig) -> Result<()> {
    let root = repo.path().parent().unwrap();
    let token_path = &config.deploy.github_provider.token_path;
    let force = config.deploy.force;
    let remote_url_in_config = config.deploy.github_provider.remote_url.as_str();
    let branch_in_config = config.deploy.github_provider.branch.as_str();

    let token = if token_path == Path::new("") {
        String::new()
    } else {
        fs::read_to_string(token_path)?.trim().to_owned()
    };
    
    let remotes = get_remotes(repo)?;
    let remote_origin_exists =  remotes.iter().any(|remote| remote.name == "origin");

    let remote_url = format!("https://{token}@{}", remote_url_in_config.strip_prefix("https://").context("the rmeote url starts without https").unwrap());
    if !remote_origin_exists {
        Command::new("git").args(["remote", "add", "origin", remote_url.as_str()]).current_dir(root).output()?;
        Command::new("git")
            .args(["push", "-u", "origin", branch_in_config, if force { "-f" } else { "" } ])
            .current_dir(root)
            .output()?;
    } else {
        let remote_url_equals_config = remotes.iter().any(|remote| remote.name == "origin" && remote.url == remote_url_in_config);

        if !remote_url_equals_config && !force {
            bail!("The url in remote `origin` not equal to url in [deploy.git], enable [deploy.force] or reset url manually")
        }

        println!("AAA");
        let a = Command::new("git").args(["remote", "set-url", "origin", remote_url.as_str()]).current_dir(root).output()?;
        let b = Command::new("git")
            .args(["push", "-u", "origin", branch_in_config, if force { "-f" } else { "" } ])
            .current_dir(root)
            .output()?;

        println!("{}", String::from_utf8(a.stderr)?);
        println!("{}", String::from_utf8(b.stderr)?);
        println!("{}", String::from_utf8(a.stdout)?);
        println!("{}", String::from_utf8(b.stdout)?);
    }

    Ok(())
}

fn get_remotes(repo: &Repository) -> Result<Vec<Remote>> {
    let root = repo.path().parent().unwrap();

    let output = Command::new("git").args(["remote", "-v"]).current_dir(root).output()?;
    let output = String::from_utf8(output.stdout)?;
        
    let remotes = output.lines().map(|line| {
        let parts: Vec<_> = line.split_whitespace().collect();
        let name = parts[0].to_owned();
        let url = parts[1].to_owned();

        Remote { name, url }
    }).collect();

    Ok(remotes)
}

fn build_tree_from_dir(root: &Path, repo: &Repository, index: &mut gix::index::State) -> Result<Tree> {
    let mut tree = Tree::empty();

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let file_name: BString = entry.file_name().into_string()
            .map_err(|e| anyhow!("{e:?}"))?.into();

        if path.is_dir() && file_name != ".git" {
            let sub_tree = build_tree_from_dir(&path, repo, index)?;
            let tree_id = repo.write_object(&sub_tree)?.detach();

            tree.entries.push(tree::Entry {
                mode: tree::EntryKind::Tree.into(),
                oid: tree_id,
                filename: file_name,
            });
        } else if path.is_file() {
            let contents = fs::read(&path)?;
            let blob_id = repo.write_blob(contents)?.into();

            index.dangerously_push_entry(
                Stat::from_fs(&Metadata::from_path_no_follow(&path)?)?,
                blob_id,
                Flags::empty(),
                Mode::FILE,
                file_name.as_ref());
            tree.entries.push(tree::Entry {
                mode: tree::EntryKind::Blob.into(),
                oid: blob_id,
                filename: file_name,
            });
        }
    }

    tree.entries.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(tree)
}
