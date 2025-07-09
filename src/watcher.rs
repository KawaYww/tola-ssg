use crate::{config::SiteConfig, log, utils};
use anyhow::{Context, Result};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::{path::PathBuf, time::{Duration, Instant}};
use tokio::sync::oneshot;


pub fn watch_for_changes_blocking(config: &'static SiteConfig, mut shutdown_rx: oneshot::Receiver<()>) -> Result<()> {
    if !config.serve.watch {
        return Ok(());
    }
    
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher =
        notify::recommended_watcher(tx).context("[watcher] Failed to create file watcher")?;

    watcher
        .watch(&config.build.content_dir, RecursiveMode::Recursive)
        .context(format!(
            "[watcher] Failed to watch directory: {}",
            config.build.content_dir.display()
        ))?;

    watcher
        .watch(&config.build.assets_dir, RecursiveMode::Recursive)
        .context(format!(
            "[watcher] Failed to watch directory: {}",
            config.build.assets_dir.display()
        ))?;

    let mut last_event_time = Instant::now();
    let debounce_duration = Duration::from_millis(50);

    log!(
        "watcher",
        "Watching for changes in {}",
        config.build.content_dir.display()
    );

    for res in rx {
        match res {
            Ok(event) => {
                if should_process_event(&event) && last_event_time.elapsed() > debounce_duration {
                    last_event_time = Instant::now();
                    std::thread::spawn(move || {

                        match handle_files(&event.paths, config) {
                            Ok(()) => (),
                            Err(e) => log!("watcher", "Error: {:?}", e),
                        }

                    });
                }
            },
            Err(e) => {
                log!("watcher", "Error: {:?}", e);
            },
        };

        if shutdown_rx.try_recv().is_ok() {
            log!("watcher", "Received shutdown signal");
            break;
        }
    }

    Ok(())
}

fn should_process_event(event: &Event) -> bool {
    matches!(event.kind, EventKind::Any)
    // matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_))

}

fn handle_files(paths: &[PathBuf], config: &SiteConfig) -> Result<()> {
    // log!("watcher", "Detected changes in: {:?}", paths);
    utils::process_watched_files(paths, config).context("Failed to process changed files")
}
