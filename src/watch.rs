use crate::{config::SiteConfig, log, utils::watch::process_watched_files};
use anyhow::{Context, Result};
#[allow(unused_imports)]
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::sync::oneshot;

#[rustfmt::skip]
pub fn watch_for_changes_blocking(config: &'static SiteConfig, shutdown_rx: &mut oneshot::Receiver<()>) -> Result<()> {
    if !config.serve.watch { return Ok(()) }

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher =
        notify::recommended_watcher(tx).context("[watch] Failed to create file watcher")?;

    watcher.watch(&config.build.content, RecursiveMode::Recursive)
        .with_context(|| format!(
            "[watch] Failed to watch directory: {}",
            config.build.content.display()
        ))?;
    log!("watch"; "watching for changes in {}", config.build.content.display());

    watcher.watch(&config.build.assets, RecursiveMode::Recursive)
        .with_context(|| format!(
            "[watch] Failed to watch directory: {}",
            config.build.assets.display()
        ))?;
    log!("watch"; "watching for changes in {}", config.build.assets.display());

    let mut last_event_time = Instant::now();
    let debounce_duration = Duration::from_millis(50);

    for res in rx {
        match res {
            Ok(event) => if should_process_event(&event) && last_event_time.elapsed() > debounce_duration {
                last_event_time = Instant::now();
                std::thread::spawn(move || if let Err(e) = handle_files(&event.paths, config) {
                    log!("watch"; "error: {:?}; event kind: {:?}", e, event.kind);
                });
            },
            Err(e) => {
                log!("watch"; "error: {:?}", e);
            },
        };

        if shutdown_rx.try_recv().is_ok() {
            log!("watch"; "received shutdown signal");
            break;
        }
    }

    Ok(())
}

#[rustfmt::skip]
fn should_process_event(event: &Event) -> bool {
    let kind = event.kind;
    matches!(kind, EventKind::Modify(_) | EventKind::Create(_)) && !matches!(kind, EventKind::Remove(_))
}

fn handle_files(paths: &[PathBuf], config: &'static SiteConfig) -> Result<()> {
    // log!("watch"; "Detected changes in: {:?}", paths);
    process_watched_files(paths, config).context("Failed to process changed files")
}
