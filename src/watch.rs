use crate::{config::SiteConfig, log, utils::watch::process_watched_files};
use anyhow::{Context, Result};
#[allow(unused_imports)]
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::{
    path::Path, sync::{Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}
};
// use tokio::sync::oneshot;

#[rustfmt::skip]
pub fn watch_for_changes_blocking(config: &'static SiteConfig, server_ready: Arc<AtomicBool>) -> Result<()> {
    if !config.serve.watch { return Ok(()) }
    // println!("watch: {:?}", server_ready);

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx).context("Failed to create file watcher")?;

    watch_directory(&mut watcher, "content", &config.build.content).unwrap();
    watch_directory(&mut watcher, "assets", &config.build.assets).unwrap();

    let mut last_event_time = Instant::now();
    let debounce_duration = Duration::from_millis(50);

    let mut last_path = String::new();
    for res in rx {
        if !server_ready.load(Ordering::SeqCst) { break }
        match res {
            Err(e) => log!("watch"; "error: {e:?}"),
            Ok(event) if should_process_event(&event) => {
                let Some(path) = event.paths.first() else { continue };
                let path_str = path.to_string_lossy();

                if last_path == path_str || last_event_time.elapsed() < debounce_duration { continue }

                // println!("{event:?}");
                last_path = path_str.to_string();
                last_event_time = Instant::now();
                handle_event(&event, config);
            },
            _ => continue
        };
    }
    Ok(())
}

// Helper function to watch a directory and log it
#[rustfmt::skip]
fn watch_directory(
    watcher: &mut impl Watcher,
    name: &str,
    path: &Path,
) -> Result<()> {
    watcher.watch(path, RecursiveMode::Recursive)
        .with_context(|| format!("[watch] Failed to watch {name} directory: {}", path.display()))?;
    log!("watch"; "watching for changes in {}: {}", name, path.display());
    Ok(())
}

#[rustfmt::skip]
fn should_process_event(event: &Event) -> bool {
    let kind = event.kind;
    matches!(kind, EventKind::Modify(_) | EventKind::Create(_)) && !matches!(kind, EventKind::Remove(_))
}

#[rustfmt::skip]
fn handle_event(event: &Event, config: &'static SiteConfig)  {
    // log!("watch"; "Detected changes in: {:?}", event.paths);
    if let Err(err) = process_watched_files(&event.paths, config).context("Failed to process changed files")  {
        log!("watch"; "{err}");
    };
}
