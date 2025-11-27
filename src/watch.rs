//! File system watcher for live reload.
//!
//! Monitors content and asset directories for changes and triggers rebuilds.

use crate::{config::SiteConfig, log, utils::watch::process_watched_files};
use anyhow::{Context, Result};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::{
    collections::HashMap,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

/// Debounce duration in milliseconds to prevent duplicate events
const DEBOUNCE_MS: u64 = 50;

/// Start blocking file watcher for content and asset changes
pub fn watch_for_changes_blocking(
    config: &'static SiteConfig,
    server_ready: Arc<AtomicBool>,
) -> Result<()> {
    if !config.serve.watch {
        return Ok(());
    }

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx).context("Failed to create file watcher")?;

    // Watch all relevant directories
    watch_directory(&mut watcher, "content", &config.build.content)?;
    watch_directory(&mut watcher, "assets", &config.build.assets)?;

    let debounce_duration = Duration::from_millis(DEBOUNCE_MS);
    let mut last_events: HashMap<String, Instant> = HashMap::new();

    for res in rx {
        if !server_ready.load(Ordering::Relaxed) {
            break;
        }

        match res {
            Err(e) => log!("watch"; "error: {e:?}"),
            Ok(event) if should_process_event(&event) => {
                let paths: Vec<_> = event
                    .paths
                    .iter()
                    .filter(|path| {
                        let path_str = path.to_string_lossy();
                        let now = Instant::now();

                        // Check if this path was recently processed
                        if let Some(&last_time) = last_events.get(path_str.as_ref())
                            && now.duration_since(last_time) < debounce_duration
                        {
                            return false;
                        }

                        last_events.insert(path_str.to_string(), now);
                        true
                    })
                    .cloned()
                    .collect();

                if !paths.is_empty() {
                    handle_event(&paths, config);
                }

                // Periodically clean up old entries to prevent memory growth
                if last_events.len() > 100 {
                    let now = Instant::now();
                    last_events.retain(|_, &mut time| now.duration_since(time) < Duration::from_secs(5));
                }
            }
            _ => continue,
        }
    }

    Ok(())
}

/// Watch a directory and log the action
fn watch_directory(watcher: &mut impl Watcher, name: &str, path: &Path) -> Result<()> {
    watcher
        .watch(path, RecursiveMode::Recursive)
        .with_context(|| format!("Failed to watch {name} directory: {}", path.display()))?;
    log!("watch"; "watching for changes in {}: {}", name, path.display());
    Ok(())
}

/// Determine if an event should trigger a rebuild
fn should_process_event(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Modify(_) | EventKind::Create(_)
    )
}

/// Handle file change events
fn handle_event(paths: &[std::path::PathBuf], config: &'static SiteConfig) {
    if let Err(err) = process_watched_files(paths, config).context("Failed to process changed files") {
        log!("watch"; "{err}");
    }
}
