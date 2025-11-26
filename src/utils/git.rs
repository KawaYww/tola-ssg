//! Git operations for the static site generator.
//!
//! Handles repository initialization, commits, and remote pushing.

use crate::{config::SiteConfig, init::init_ignored_files, log, run_command};
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
};
use std::{fs, path::Path};

/// Repository root path helper
fn repo_root(repo: &Repository) -> Result<&Path> {
    repo.path().parent().ok_or_else(|| anyhow!("Invalid repository path"))
}

#[derive(Debug)]
struct Remote {
    name: String,
    url: String,
}

impl Remote {
    /// Parse remotes from `git remote -v` output
    fn list_from_repo(repo: &Repository) -> Result<Vec<Self>> {
        let root = repo_root(repo)?;
        let output = run_command!(root; ["git"]; "remote", "-v")?;
        let stdout = std::str::from_utf8(&output.stdout)?;

        let remotes = stdout
            .lines()
            .filter(|line| line.ends_with("(fetch)"))
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                Some(Remote {
                    name: parts.next()?.to_owned(),
                    url: parts.next()?.to_owned(),
                })
            })
            .collect();

        Ok(remotes)
    }

    /// Check if origin remote exists with matching URL
    fn origin_matches(repo: &Repository, expected_url: &str) -> Result<bool> {
        Ok(Self::list_from_repo(repo)?
            .iter()
            .any(|r| r.name == "origin" && r.url == expected_url))
    }

    /// Check if origin remote exists
    fn origin_exists(repo: &Repository) -> Result<bool> {
        Ok(Self::list_from_repo(repo)?
            .iter()
            .any(|r| r.name == "origin"))
    }
}

pub fn create_repo(root: &Path) -> Result<ThreadSafeRepository> {
    let repo = gix::init(root)?;
    init_ignored_files(root, &[Path::new(".DS_Store")])?;
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
    let root = repo_root(&repo_local)?;

    let git_ignore = read_gitignore(root)?;

    let mut index = State::new(repo_local.object_hash());
    let tree = build_tree_from_dir(root, repo, &mut index, &git_ignore)?;
    index.sort_entries();

    let mut file = gix::index::File::from_state(index, repo_local.index_path());
    file.write(gix::index::write::Options::default())?;

    let tree_id = repo_local.write_object(&tree)?;
    let parent_ids = get_parent_ids(repo)?;
    let commit_id = repo_local.commit("HEAD", message, tree_id, parent_ids)?;

    log!("commit"; "created commit `{commit_id}` in repo `{}`", root.display());
    Ok(())
}

/// Read .gitignore file if it exists
fn read_gitignore(root: &Path) -> Result<Vec<u8>> {
    let path = root.join(".gitignore");
    if path.exists() {
        Ok(fs::read(path)?)
    } else {
        Ok(Vec::new())
    }
}

/// Get parent commit IDs (empty for initial commit)
fn get_parent_ids(repo: &ThreadSafeRepository) -> Result<Vec<gix::ObjectId>> {
    let repo_local = repo.to_thread_local();

    Ok(repo_local
        .find_reference("refs/heads/main")
        .ok()
        .map(|refs| vec![refs.target().id().to_owned()])
        .unwrap_or_else(|| NO_PARENT_IDS.to_vec()))
}

pub fn push(repo: &ThreadSafeRepository, config: &'static SiteConfig) -> Result<()> {
    let github = &config.deploy.github_provider;
    log!("git"; "pushing to `{}`", github.url);

    let repo_local = repo.to_thread_local();
    let root = repo_root(&repo_local)?;

    let remote_url = build_authenticated_url(&github.url, github.token_path.as_ref())?;
    let remote_action = if Remote::origin_exists(&repo_local)? {
        "set-url"
    } else {
        "add"
    };

    run_command!(root; ["git"]; "remote", remote_action, "origin", &remote_url)?;

    // Build push command with optional force flag
    if config.deploy.force {
        run_command!(root; ["git"]; "push", "--set-upstream", "origin", &github.branch, "-f")?;
    } else {
        run_command!(root; ["git"]; "push", "--set-upstream", "origin", &github.branch)?;
    }

    // Verify remote URL matches config (unless force is enabled)
    if !config.deploy.force && !Remote::origin_matches(&repo_local, &remote_url)? {
        bail!(
            "Remote origin URL in `{root:?}` doesn't match [deploy.git] config. \
             Enable [deploy.force] or fix manually."
        );
    }

    Ok(())
}

/// Build authenticated HTTPS URL with optional token
fn build_authenticated_url(url: &str, token_path: Option<&std::path::PathBuf>) -> Result<String> {
    let base_url = url
        .strip_prefix("https://")
        .context("Remote URL must start with https://")?;

    let token = token_path
        .map(|p| fs::read_to_string(p).unwrap_or_default().trim().to_owned())
        .unwrap_or_default();

    if token.is_empty() {
        Ok(format!("https://{base_url}"))
    } else {
        Ok(format!("https://{token}@{base_url}"))
    }
}

/// Check if path should be ignored based on .gitignore patterns
fn is_ignored(path: &str, git_ignore: &[u8]) -> bool {
    gix::ignore::parse(git_ignore).any(|(pattern, _, _)| {
        wildmatch(
            path.into(),
            pattern.text.as_bstr(),
            wildmatch::Mode::NO_MATCH_SLASH_LITERAL,
        )
    })
}

fn build_tree_from_dir(
    dir_root: &Path,
    repo: &ThreadSafeRepository,
    index: &mut gix::index::State,
    git_ignore: &[u8],
) -> Result<Tree> {
    let mut tree = Tree::empty();
    let repo_local = repo.to_thread_local();
    let root = repo.path().parent().context("Invalid repo path")?;

    for entry in fs::read_dir(dir_root)? {
        let entry = entry?;
        let path = entry.path();
        let filename: BString = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow!("Invalid UTF-8 in filename"))?
            .into();

        let relative_path = path.strip_prefix(root)?.to_string_lossy();

        // Skip ignored paths
        if is_ignored(&relative_path, git_ignore) {
            continue;
        }

        if path.is_dir() && filename != ".git" {
            let sub_tree = build_tree_from_dir(&path, repo, index, git_ignore)?;
            let tree_id = repo_local.write_object(&sub_tree)?.detach();

            tree.entries.push(tree::Entry {
                mode: tree::EntryKind::Tree.into(),
                oid: tree_id,
                filename,
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

            tree.entries.push(tree::Entry {
                mode: tree::EntryKind::Blob.into(),
                oid: blob_id,
                filename,
            });
        }
    }

    // Sort entries according to git tree ordering (directories get trailing slash)
    tree.entries.sort_by(|a, b| {
        let tree_mode: tree::EntryMode = tree::EntryKind::Tree.into();
        let key = |e: &tree::Entry| {
            let mut k = e.filename.as_slice().to_vec();
            if e.mode == tree_mode {
                k.push(b'/');
            }
            k
        };
        key(a).cmp(&key(b))
    });

    Ok(tree)
}
