// Fucking 
// Fucking
// 
// Everyone holds hands to form a circle around `git subprocess` and dances.
//
// Fucking ! !
// Fucking ! ! ! !

use std::{fs, io::BufWriter, path::Path};
use anyhow::{anyhow, Result};
use gix::{bstr::BString, index::{entry::{Flags, Mode, Stat}, fs::Metadata, State}, objs::{tree, Tree}, Repository};
use crate::log;

pub fn create_repo(root: &Path) -> Result<Repository> {
    let repo = gix::init(root)?;
    Ok(repo)
}

pub fn commit_all(repo: &Repository) -> Result<()> {   
    let root = repo.path().parent().unwrap();
    let mut index = State::new(repo.object_hash());
    let tree = build_tree_from_dir(root, repo, &mut index)?;
    index.sort_entries();

    let mut buffer = {
        let file = fs::OpenOptions::new().write(true).create(true).truncate(true).open(repo.index_path())?;
        BufWriter::new(file)
    };
    index.write_to(&mut buffer, gix::index::write::Options::default())?;

    let initial_tree_id = repo.write_object(&tree)?;
    let initial_commit_id = repo.commit(
        "HEAD",
        "initial commit",
        initial_tree_id,
        gix::commit::NO_PARENT_IDS
    )?;
    
    log!("Initer", "commit id for blob: {initial_commit_id:?}");
    
    Ok(())
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
