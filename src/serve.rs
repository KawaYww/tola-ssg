use crate::{
    build::build_site, config::SiteConfig, log, watch::watch_for_changes_blocking
};
use anyhow::{Context, Result, anyhow};
use axum::{
    Router,
    http::{StatusCode, Uri},
    response::{Html, IntoResponse},
    routing::{get, get_service},
};
use std::{
    fs, net::{IpAddr, SocketAddr}, path::PathBuf, str::FromStr, sync::{atomic::{AtomicBool, Ordering}, Arc}, time::Duration
};
use tokio::{net::TcpListener, sync::oneshot};
use tower_http::services::ServeDir;

#[rustfmt::skip]
pub async fn serve_site(config: &'static SiteConfig) -> Result<()> {
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
    let server_ready = Arc::new(AtomicBool::new(false));
    
    tokio::spawn({
        let server_ready = Arc::clone(&server_ready);
        async move { while let Err(e) = start_server(config, &server_ready).await {
            log!("error", "failed to start server: {e:?}");
            let timeout_secs = 2;
            for i in (0..=timeout_secs).rev() {
                log!("serve", "automatically trying to start it again in {i} seconds");
                tokio::time::sleep(Duration::from_secs(i)).await;
            }
        }}
    });

    std::thread::spawn(move || {
        log!("watch", "waiting for server starting");
        while !server_ready.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_secs(1));
        }
        watch_for_changes_blocking(config, &mut shutdown_rx).ok();
    });

    tokio::signal::ctrl_c().await?;
    shutdown_tx.send(()).map_err(|_| anyhow!("Failed to send shutdown message to watcher"))?;

    Ok(())
}

pub async fn start_server(config: &'static SiteConfig, server_ready: &Arc<AtomicBool>) -> Result<()> {
    build_site(config, false)?;

    let interface = IpAddr::from_str(&config.serve.interface)?;
    let port = config.serve.port;
    let addr = SocketAddr::new(interface, port);
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
    log!("serve", "serving site on http://{}", addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("[server] failed to start")?;

    Ok(())
}

async fn handle_path(uri: Uri, base_path: PathBuf) -> impl IntoResponse {
    let request_path = uri.path().trim_matches('/');
    let request_path = urlencoding::decode(request_path).unwrap().into_owned();
    let local_path = base_path.join(&request_path);

    if local_path.is_file() {
        return match fs::read_to_string(&local_path) {
            Ok(content) => Html(content).into_response(),
            Err(_) => handle_404().await.into_response(),
        };
    }
    if local_path.is_dir() {
        println!("CCCC");
        let index_path = local_path.join("index.html");
        if index_path.is_file() {
            return match fs::read_to_string(&index_path) {
                Ok(content) => Html(content).into_response(),
                Err(_) => handle_404().await.into_response(),
            };
        }
        let mut file_list = String::new();
        if let Ok(entries) = fs::read_dir(&local_path) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let href = format!("{}/{}", &request_path, name);
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
                        <h1>Directory: {request_path}</h1>
                        {file_list}
                    </body>
                </html>
            "#));
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
    log!("serve", "shutting down gracefully...");
}
