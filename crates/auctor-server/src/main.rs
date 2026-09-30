mod account;
mod admin;
mod admin_applications;
mod admin_bulk;
mod admin_groups;
mod admin_roles;
mod admin_sessions;
mod auth;
mod avatar;
mod branding;
mod db;
mod email;
mod model;
mod oauth;
mod password_recovery;
mod registration;
mod security;
mod settings;
mod setup;
mod username_policy;
mod verification;

use anyhow::Context;
use axum::{Json, Router, extract::DefaultBodyLimit, middleware, routing::get};
use serde::Serialize;
use sqlx::PgPool;
use std::{env, net::SocketAddr, path::PathBuf, sync::Arc};
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
    pub setup_token: Option<String>,
    pub setup_token_path: PathBuf,
    pub rate_limiter: security::RateLimiter,
    pub oidc_signer: Arc<oauth::OidcSigner>,
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
    let (setup_token, setup_token_path) = db::ensure_setup_token(&db).await?;
    let oidc_signer = Arc::new(oauth::OidcSigner::load_or_create()?);

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
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(8))
            .build()?,
        setup_token,
        setup_token_path,
        rate_limiter: security::RateLimiter::new(),
        oidc_signer,
    };

    let api = Router::new()
        .merge(auth::router())
        .merge(account::router())
        .merge(admin::router())
        .merge(admin_applications::router())
        .merge(admin_bulk::router())
        .merge(admin_groups::router())
        .merge(admin_roles::router())
        .merge(admin_sessions::router())
        .merge(avatar::router())
        .merge(branding::router())
        .merge(password_recovery::router())
        .merge(registration::router())
        .merge(settings::router())
        .merge(setup::router())
        .merge(username_policy::router());

    let web_dir = env::var("AUCTOR_WEB_DIR").unwrap_or_else(|_| "web-dist".into());
    let assets_dir = format!("{web_dir}/assets");
    let brand_dir = format!("{web_dir}/brand");
    let index_file = format!("{web_dir}/index.html");

    let app = Router::new()
        .route("/health", get(health))
        .route(
            "/.well-known/openid-configuration",
            get(oauth::openid_metadata),
        )
        .merge(oauth::router())
        .nest("/api", api)
        .nest_service("/assets", ServeDir::new(assets_dir))
        .nest_service("/brand", ServeDir::new(brand_dir))
        .fallback_service(ServeFile::new(index_file))
        .layer(DefaultBodyLimit::max(128 * 1024))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            security::middleware,
        ))
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
