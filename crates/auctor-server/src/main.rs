mod admin;
mod auth;
mod avatar;
mod db;
mod model;

use anyhow::Context;
use axum::{Json, Router, routing::get};
use serde::Serialize;
use sqlx::PgPool;
use std::{env, net::SocketAddr};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub secure_cookies: bool,
    pub session_hours: i64,
    pub http: reqwest::Client,
}

#[derive(Serialize)]
struct Health<'a> {
    status: &'a str,
    service: &'a str,
    version: &'a str,
}

async fn health() -> Json<Health<'static>> {
    Json(Health {
        status: "ok",
        service: "auctor",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn discovery() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "issuer": null,
        "status": "not_configured",
        "message": "OIDC provider endpoints are not enabled in this pre-alpha build"
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "auctor_server=info,tower_http=info".into()),
        )
        .init();

    let database_url = env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    let db = db::connect(&database_url).await?;
    db::bootstrap_admin(&db).await?;

    let state = AppState {
        db,
        secure_cookies: env::var("AUCTOR_SECURE_COOKIES")
            .map(|v| v != "false")
            .unwrap_or(true),
        session_hours: env::var("AUCTOR_SESSION_HOURS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(168),
        http: reqwest::Client::builder()
            .user_agent("Auctor/0.1 avatar proxy")
            .build()?,
    };

    let api = Router::new()
        .merge(auth::router())
        .merge(admin::router())
        .merge(avatar::router());

    let web_dir = env::var("AUCTOR_WEB_DIR").unwrap_or_else(|_| "web-dist".into());
    let assets_dir = format!("{web_dir}/assets");
    let index_file = format!("{web_dir}/index.html");

    let app = Router::new()
        .route("/health", get(health))
        .route("/.well-known/openid-configuration", get(discovery))
        .nest("/api", api)
        .nest_service("/assets", ServeDir::new(assets_dir))
        .fallback_service(ServeFile::new(index_file))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let port = env::var("AUCTOR_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(%addr, "starting Auctor");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
