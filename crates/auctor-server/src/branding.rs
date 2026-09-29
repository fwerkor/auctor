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
    pub avatar_source_template: String,
    pub avatar_delivery: String,
}

#[derive(Clone, Serialize, sqlx::FromRow)]
pub struct AvatarSettings {
    pub avatar_source_template: String,
    pub avatar_delivery: String,
}

#[derive(Deserialize)]
struct UpdateBranding {
    site_name: String,
    site_url: String,
    logo_url: String,
}

#[derive(Deserialize)]
struct UpdateAvatarSettings {
    avatar_source_template: String,
    avatar_delivery: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/config", get(public_config))
        .route("/admin/branding", get(admin_get).put(admin_update))
        .route(
            "/admin/avatar-settings",
            get(admin_avatar_get).put(admin_avatar_update),
        )
}

pub async fn load(db: &sqlx::PgPool) -> Result<Branding, sqlx::Error> {
    sqlx::query_as(
        "SELECT site_name,site_url,logo_url,avatar_source_template,avatar_delivery
         FROM site_settings WHERE singleton=true",
    )
    .fetch_one(db)
    .await
}

pub async fn load_avatar_settings(db: &sqlx::PgPool) -> Result<AvatarSettings, sqlx::Error> {
    sqlx::query_as(
        "SELECT avatar_source_template,avatar_delivery
         FROM site_settings WHERE singleton=true",
    )
    .fetch_one(db)
    .await
}

async fn public_config(State(state): State<AppState>) -> Result<Json<Branding>, StatusCode> {
    let mut config = load(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if config.avatar_delivery == "proxy" {
        config.avatar_source_template.clear();
    }
    Ok(Json(config))
}

async fn admin_get(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Branding>, StatusCode> {
    require_admin(&state, &headers).await?;
    load(&state.db)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
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

async fn admin_avatar_get(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AvatarSettings>, StatusCode> {
    require_admin(&state, &headers).await?;
    load_avatar_settings(&state.db)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn admin_avatar_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateAvatarSettings>,
) -> Result<Json<AvatarSettings>, StatusCode> {
    let actor = require_admin(&state, &headers).await?;
    let source = payload.avatar_source_template.trim();
    let delivery = payload.avatar_delivery.trim().to_ascii_lowercase();

    validate_avatar_source_template(source)?;
    if !matches!(delivery.as_str(), "direct" | "proxy") {
        return Err(StatusCode::BAD_REQUEST);
    }

    sqlx::query(
        "UPDATE site_settings
         SET avatar_source_template=$1,avatar_delivery=$2,updated_at=now()
         WHERE singleton=true",
    )
    .bind(source)
    .bind(&delivery)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query(
        "INSERT INTO audit_events(actor_user_id,action,target_type,target_id,metadata)
         VALUES($1,'avatar_settings.update','settings','avatar',$2)",
    )
    .bind(actor.id)
    .bind(json!({
        "avatar_source_template": source,
        "avatar_delivery": delivery,
    }))
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    load_avatar_settings(&state.db)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub fn validate_avatar_source_template(raw: &str) -> Result<(), StatusCode> {
    if raw.is_empty() || raw.len() > 2048 {
        return Err(StatusCode::BAD_REQUEST);
    }
    if !raw.contains("{email}") && !raw.contains("{email_md5}") {
        return Err(StatusCode::BAD_REQUEST);
    }

    let sample = raw
        .replace("{email_md5}", "00000000000000000000000000000000")
        .replace("{email}", "user%40example.com")
        .replace("{size}", "96");

    if sample.contains('{') || sample.contains('}') {
        return Err(StatusCode::BAD_REQUEST);
    }

    let url = Url::parse(&sample).map_err(|_| StatusCode::BAD_REQUEST)?;
    if url.scheme() != "https" || url.host_str().is_none() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avatar_source_requires_https_and_identity_placeholder() {
        assert!(
            validate_avatar_source_template(
                "https://www.gravatar.com/avatar/{email_md5}?s={size}&d=404"
            )
            .is_ok()
        );
        assert!(
            validate_avatar_source_template("https://avatar.example/{email}?size={size}").is_ok()
        );
        assert!(validate_avatar_source_template("http://avatar.example/{email_md5}").is_err());
        assert!(validate_avatar_source_template("https://avatar.example/static.png").is_err());
        assert!(validate_avatar_source_template("https://avatar.example/{unknown}").is_err());
    }
}
