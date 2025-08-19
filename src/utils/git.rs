/// Fucking
// Fucking
//
// Everyone holds hands to form a circle around `git subprocess` and dances.
//
// Fucking ! !
// Fucking ! ! ! !
use crate::{config::SiteConfig, init::init_ignore_files, log, run_command};
use anyhow::{Context, Result, anyhow, bail};
use gix::{
    Repository, ThreadSafeRepository,
    bstr::{BString, ByteSlice},
    commit::NO_PARENT_IDS,
    glob::wildmatch,
    index::{
        State,
        entry::{Flags, Mode, Stat},
        fs::Metadata,
    },
    objs::{Tree, tree},
    path::into_bstr,
};
use std::{
    fs,
    io::BufRead,
    path::{Path, PathBuf},
};

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

        let remotes = output.lines().filter(|line| line.ends_with("fetch)")).map(|line| {
            let parts: Vec<_> = line.split_whitespace().collect();
            let name = parts[0].to_owned();
            let url = parts[1].to_owned();
            assert_eq!(name, "origin");

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

pub fn create_repo(root: &Path) -> Result<ThreadSafeRepository> {
    let repo = gix::init(root)?;
    init_ignore_files(root, &[Path::new(".DS_Store")])?;
    Ok(repo.into_sync())
}

pub fn open_repo(root: &Path) -> Result<ThreadSafeRepository> {
    let repo = gix::open(root)?;
    Ok(repo.into_sync())
}

pub fn commit_all(repo: &ThreadSafeRepository, message: &str) -> Result<()> {
    if message.trim().is_empty() {
        bail!("Commit message cannot be empty");
    }

    let repo_local = repo.to_thread_local();

    let root = repo_local
        .path()
        .parent()
        .ok_or(anyhow!("Invalid repository path"))?;

    let git_ignore = root.join(".gitignore");
    let git_ignore = if git_ignore.exists() {
        fs::read(git_ignore)?
    } else {
        vec![]
    };

    let mut index = State::new(repo_local.object_hash());
    let tree = build_tree_from_dir(root, repo, &mut index, &git_ignore)?;
    index.sort_entries();

    let mut file = gix::index::File::from_state(index, repo_local.index_path());
    file.write(gix::index::write::Options::default())?;

    let tree_id = repo_local.write_object(&tree)?;
    let commit_id = repo_local.commit("HEAD", message, tree_id, parent_ids_or_empty(repo)?)?;

    log!("commit"; "commit for blob `{commit_id:?}` in  repo `{}`", root.display());
    Ok(())
}

fn parent_ids_or_empty(repo: &ThreadSafeRepository) -> Result<Vec<gix::ObjectId>> {
    let repo_local = repo.to_thread_local();

    let ids = match repo_local.find_reference("refs/heads/main") {
        Err(_) => NO_PARENT_IDS.to_vec(),
        Ok(refs) => {
            let target = refs.target();
            [target.id().to_owned()].to_vec()
        }
    };
    Ok(ids)
}

#[rustfmt::skip]
pub fn push(repo: &ThreadSafeRepository, config: &'static SiteConfig) -> Result<()> {
    let remote_url = config.deploy.github_provider.url.as_str();
    log!("git"; "pushing to `{remote_url}`");

    let repo_local = repo.to_thread_local();
    let root = repo_local.path().parent().ok_or(anyhow!("Invalid repository path"))?;
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

    let remote_action = if Remotes::new(&repo_local)?.any(|remote| remote.name == "origin") { "set-url" } else { "add" };

    run_command!(root; ["git"];
        "remote", remote_action, "origin", &remote_url
    )?;
    run_command!(root; ["git"];
        "push", "--set-upstream", "origin", branch_in_config, if force { "-f" } else { "" }
    )?;

    let remote_url_equals_config = Remotes::new(&repo_local)?.any(|remote| remote.name == "origin" && remote.url == remote_url);
    if !remote_url_equals_config && !force { bail!(
        "The url in remote `origin` in repo `{root:?}` not equal to url in [deploy.git], enable [deploy.force] or reset url manually"
    )}
    Ok(())
}

fn build_tree_from_dir(
    dir_root: &Path,
    repo: &ThreadSafeRepository,
    index: &mut gix::index::State,
    git_ignore: &[u8],
) -> Result<Tree> {
    let mut tree = Tree::empty();
    let repo_local = repo.to_thread_local();
    let repo_root = repo.path().parent().unwrap();

    for entry in fs::read_dir(dir_root)? {
        let entry = entry?;
        let path = entry.path();
        let filename: BString = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow!("Invalid file name"))?
            .into();

        let mut ignored_paths = gix::ignore::parse(git_ignore);

        if path.is_dir() && filename != ".git" {
            let sub_tree = build_tree_from_dir(&path, repo, index, git_ignore)?;
            let tree_id = repo_local.write_object(&sub_tree)?.detach();

            let path = path.strip_prefix(repo_root)?.to_string_lossy().into_owned();
            ignored_paths
                .all(|(ignore_path, _, _)| {
                    !wildmatch(
                        path.as_str().into(),
                        ignore_path.text.as_bstr(),
                        wildmatch::Mode::NO_MATCH_SLASH_LITERAL,
                    )
                })
                .then(|| {
                    tree.entries.push(tree::Entry {
                        mode: tree::EntryKind::Tree.into(),
                        oid: tree_id,
                        filename,
                    });
                });
        } else if path.is_file() {
            let contents = fs::read(&path)?;
            let blob_id = repo_local.write_blob(contents)?.into();

            index.dangerously_push_entry(
                Stat::from_fs(&Metadata::from_path_no_follow(&path)?)?,
                blob_id,
                Flags::empty(),
                Mode::FILE,
                filename.as_ref(),
            );
            let path = path.strip_prefix(repo_root)?.to_string_lossy().into_owned();
            ignored_paths
                .all(|(ignore_path, _, _)| {
                    !wildmatch(
                        path.as_str().into(),
                        ignore_path.text.as_bstr(),
                        wildmatch::Mode::NO_MATCH_SLASH_LITERAL,
                    )
                })
                .then(|| {
                    tree.entries.push(tree::Entry {
                        mode: tree::EntryKind::Blob.into(),
                        oid: blob_id,
                        filename,
                    });
                });
        }
    }

    tree.entries.sort_by(|a, b| {
        let mut a_key = a.filename.as_slice().to_vec();
        let mut b_key = b.filename.as_slice().to_vec();

        let tree_mode = tree::EntryKind::Tree.into();
        if a.mode == tree_mode {
            a_key.push(b'/');
        }
        if b.mode == tree_mode {
            b_key.push(b'/');
        }

        a_key.cmp(&b_key)
    });
    Ok(tree)
}
