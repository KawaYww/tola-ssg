use crate::{
    builder::build_site, config::SiteConfig, log, watcher::watch_for_changes_blocking
};
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
    str::FromStr, time::Duration,
};
use tokio::{net::TcpListener, sync::oneshot};
use tower_http::services::ServeDir;

#[rustfmt::skip]
pub async fn serve_site(config: &'static SiteConfig) -> Result<()> {
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    std::thread::spawn(move || watch_for_changes_blocking(config, shutdown_rx));

    tokio::spawn({
        let timeout_secs = 2;
        let mut restart_flag = true;
        async move { while restart_flag { match start_server(config).await {
            Ok(()) => restart_flag = false,
            Err(e) => {
                log!("error", "Failed to start server: {e:?}");
                for i in (0..=timeout_secs).rev() {
                    log!("tips", "Automatically trying to start it again in {i} seconds");
                    tokio::time::sleep(Duration::from_secs(i)).await;
                }
            }
        }}}
    });

    tokio::signal::ctrl_c().await?;
    shutdown_tx.send(()).map_err(|_| anyhow!("Failed to send shutdown message to watcher"))?;

    Ok(())
}

pub async fn start_server(config: &'static SiteConfig) -> Result<()> {
    build_site(config)?;

    let interface = IpAddr::from_str(&config.serve.interface)?;
    let port = config.serve.port;
    let addr = SocketAddr::new(interface, port);
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("[Server] Failed to bind to address {addr}"))?;
    let app = {
        let base_path = config.build.output_dir.clone();
        let serve_dir = ServeDir::new(&config.build.output_dir)
            .append_index_html_on_directories(false)
            .not_found_service(get(move |url| handle_path(url, base_path)));
        Router::new().fallback(get_service(serve_dir))
    };

    log!("server", "Serving site on http://{}", addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("[Server] Failed to start")?;

    Ok(())
}

async fn handle_path(uri: Uri, base_path: PathBuf) -> impl IntoResponse {
    let request_path = uri.path().trim_start_matches('/');
    let local_path = base_path.join(request_path);

    if local_path.is_file() {
        return match fs::read_to_string(&local_path) {
            Ok(content) => Html(content).into_response(),
            Err(_) => handle_404().await.into_response(),
        };
    }
    if local_path.is_dir() {
        let index_path = local_path.join("index.html");
        if index_path.is_file() {
            return match fs::read_to_string(&index_path) {
                Ok(content) => Html(content).into_response(),
                Err(_) => handle_404().await.into_response(),
            };
        }
        let mut file_list = String::new();
        if let Ok(entries) = fs::read_dir(&local_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy().into_owned();
                let href = format!("{}/{}", uri.path().trim_end_matches('/'), name);
                file_list.push_str(&format!("<li><a href='{href}'>{name}</a></li>"));
            }
            let html_content = Html(format!(r#"
                <html>
                    <head><style>
                        * {{ background: #273748; color: white;  }}
                        li {{ font-weight: bold; }}
                        table {{ border-collapse: collapse; }} td {{ padding: 8px; }}
                    </style></head>
                    <body>
                        <h1>Directory: {}</h1>
                        {}
                    </body>
                </html>
            "#, uri.path(), file_list));
            return html_content.into_response();
        }
    }

    handle_404().await.into_response()
}

async fn handle_404() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "404 Not Found")
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
    log!("server", "Shutting down gracefully...");
}
