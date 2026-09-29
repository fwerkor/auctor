use axum::{Json, Router, routing::get};
use serde::Serialize;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

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
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "auctor_server=info,tower_http=info".into()),
        )
        .init();

    let app = Router::new()
        .route("/health", get(health))
        .route("/.well-known/openid-configuration", get(discovery))
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!(%addr, "starting Auctor");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
