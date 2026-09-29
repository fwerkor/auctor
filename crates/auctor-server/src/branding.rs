use crate::{AppState, auth::require_admin};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::get,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use url::Url;

#[derive(Clone, Serialize, sqlx::FromRow)]
pub struct Branding {
    pub site_name: String,
    pub site_url: String,
    pub logo_url: String,
}

#[derive(Deserialize)]
struct UpdateBranding {
    site_name: String,
    site_url: String,
    logo_url: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/config", get(public_config))
        .route("/admin/branding", get(admin_get).put(admin_update))
}

pub async fn load(db: &sqlx::PgPool) -> Result<Branding, sqlx::Error> {
    sqlx::query_as("SELECT site_name,site_url,logo_url FROM site_settings WHERE singleton=true")
        .fetch_one(db)
        .await
}

async fn public_config(State(state): State<AppState>) -> Result<Json<Branding>, StatusCode> {
    load(&state.db)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn admin_get(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Branding>, StatusCode> {
    require_admin(&state, &headers).await?;
    public_config(State(state)).await
}

async fn admin_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateBranding>,
) -> Result<Json<Branding>, StatusCode> {
    let actor = require_admin(&state, &headers).await?;

    let site_name = payload.site_name.trim();
    let site_url = payload.site_url.trim().trim_end_matches('/');
    let logo_url = payload.logo_url.trim();

    if site_name.is_empty() || site_name.len() > 120 {
        return Err(StatusCode::BAD_REQUEST);
    }
    validate_site_url(site_url)?;
    validate_logo_url(logo_url)?;

    sqlx::query(
        "UPDATE site_settings
         SET site_name=$1,site_url=$2,logo_url=$3,updated_at=now()
         WHERE singleton=true",
    )
    .bind(site_name)
    .bind(site_url)
    .bind(logo_url)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query(
        "INSERT INTO audit_events(actor_user_id,action,target_type,target_id,metadata)
         VALUES($1,'branding.update','settings','branding',$2)",
    )
    .bind(actor.id)
    .bind(json!({
        "site_name": site_name,
        "site_url": site_url,
        "logo_url": logo_url,
    }))
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    load(&state.db)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn validate_site_url(raw: &str) -> Result<(), StatusCode> {
    if raw.is_empty() {
        return Ok(());
    }
    let url = Url::parse(raw).map_err(|_| StatusCode::BAD_REQUEST)?;
    if url.query().is_some() || url.fragment().is_some() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let secure = url.scheme() == "https";
    let local_http =
        url.scheme() == "http" && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
    if secure || local_http {
        Ok(())
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

fn validate_logo_url(raw: &str) -> Result<(), StatusCode> {
    if raw.is_empty() || raw.starts_with('/') {
        return Ok(());
    }
    let url = Url::parse(raw).map_err(|_| StatusCode::BAD_REQUEST)?;
    if url.scheme() == "https" {
        Ok(())
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}
