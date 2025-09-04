use crate::{config::SiteConfig, log, watch::watch_for_changes_blocking};
use anyhow::{Context, Result, anyhow};
use axum::{
    Router,
    http::{StatusCode, Uri},
    response::{Html, IntoResponse},
    routing::{get, get_service},
};
use std::{
    fs,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{net::TcpListener, sync::oneshot};
use tower_http::services::ServeDir;

#[rustfmt::skip]
pub async fn serve_site(config: &'static SiteConfig) -> Result<()> {
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
    let server_ready = Arc::new(AtomicBool::new(false));

    tokio::spawn({
        let server_ready = Arc::clone(&server_ready);
        async move { while let Err(err) = start_server(config, &server_ready).await {
            if is_nonrecoverable(&err, config) { return; }
            wait_for_retrying(&err, 2).await;
        }}
    });

    std::thread::spawn(move || {
        wait_for_server_ready(&server_ready);
        watch_for_changes_blocking(config, &mut shutdown_rx).ok();
    });

    tokio::signal::ctrl_c().await?;
    shutdown_tx.send(()).map_err(|_| anyhow!("Failed to send shutdown message to watcher"))?;

    Ok(())
}

fn is_nonrecoverable(err: &anyhow::Error, config: &SiteConfig) -> bool {
    let mut result = false;
    let err = err.to_string();
    match err.as_str() {
        "address already in use" => log!("error"; "port `{}` is already in use", config.serve.port),
        _ => result = true,
    }
    result
}

fn wait_for_server_ready(server_ready: &Arc<AtomicBool>) {
    log!("watch"; "waiting for server to start");
    while !server_ready.load(Ordering::Acquire) {
        std::thread::sleep(Duration::from_secs(1));
    }
}

#[rustfmt::skip]
async fn wait_for_retrying(err: &anyhow::Error, timeout_secs: u64) {
    log!("error"; "failed to start server (will retry): {err:?}");
    for i in (0..=timeout_secs).rev() {
        log!("serve"; "retrying in {i} seconds");
        tokio::time::sleep(Duration::from_secs(i)).await;
    }
}

pub async fn start_server(
    config: &'static SiteConfig,
    server_ready: &Arc<AtomicBool>,
) -> Result<()> {
    let addr = SocketAddr::new(
        IpAddr::from_str(&config.serve.interface)?,
        config.serve.port,
    );

    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("[serve] Failed to bind to address {addr}"))?;

    let app = {
        let base_path = config.build.output.clone();
        let serve_dir = ServeDir::new(&config.build.output)
            .append_index_html_on_directories(false)
            .not_found_service(get(move |url| handle_path(url, base_path)));
        Router::new().fallback(get_service(serve_dir))
    };

    server_ready.store(true, Ordering::Release);

    log!("serve"; "serving site on http://{}", addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("[serve] failed to start")?;

    Ok(())
}

async fn handle_path(uri: Uri, base_path: PathBuf) -> impl IntoResponse {
    let request_path = uri.path().trim_matches('/');
    let request_path = urlencoding::decode(request_path).unwrap().into_owned();
    let local_path = base_path.join(&request_path);

    // Try to read the file directly
    if let Ok(content) = fs::read_to_string(&local_path) {
        return Html(content).into_response();
    }

    // If not a file, check if it's a directory and try to serve an `index.html`
    if local_path.is_dir() {
        let index_path = local_path.join("index.html");
        if let Ok(content) = fs::read_to_string(&index_path) {
            return Html(content).into_response();
        }

        // If no index.html, generate a directory listing
        if let Ok(file_list) = generate_directory_listing(&local_path, &request_path).await {
            return Html(file_list).into_response();
        }
    }
    // Fallback to 404
    handle_404().await.into_response()
}

// Helper function to generate a directory listing
async fn generate_directory_listing(
    dir_path: &PathBuf,
    request_path: &str,
) -> Result<String, std::io::Error> {
    let mut file_list = String::new();
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let href = format!("{request_path}/{name}");
        file_list.push_str(&format!("<li><a href='{href}'>{name}</a></li>"));
    }

    Ok(format!(
        r#"
        <html>
            <head><style>
                * {{ background: #273748; color: white; }}
                li {{ font-weight: bold; }}
                table {{ border-collapse: collapse; }} td {{ padding: 8px; }}
            </style></head>
            <body>
                <h1>Directory: {request_path}</h1>
                {file_list}
            </body>
        </html>
        "#
    ))
}

// Helper function to handle 404 errors
async fn handle_404() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "404 Not Found")
}

// Helper function to handle shutdown signal
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
    log!("serve"; "shutting down gracefully...");
}
