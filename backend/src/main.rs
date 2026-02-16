use anyhow::Context;
use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::{env, net::SocketAddr, path::PathBuf};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::info;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "uran-api",
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();

    let host = env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("API_PORT").unwrap_or_else(|_| "8181".to_string());
    let repo_root = env::var("REPO_ROOT").unwrap_or_else(|_| "..".to_string());
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .context("failed to parse API_HOST/API_PORT")?;

    let frontend_dist = PathBuf::from(repo_root).join("frontend").join("dist");
    let frontend_index = frontend_dist.join("index.html");
    let static_service = ServeDir::new(frontend_dist).fallback(ServeFile::new(frontend_index));

    let app = Router::new()
        .route("/health", get(health))
        .fallback_service(static_service)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    info!("uran-api listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
