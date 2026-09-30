use crate::{AppState, auth::require_admin, email};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AuthSettings {
    pub registration_enabled: bool,
    pub registration_require_email_verification: bool,
    pub smtp_host: String,
    pub smtp_port: i32,
    pub smtp_security: String,
    pub smtp_username: String,
    pub smtp_from_email: String,
    pub smtp_from_name: String,
    pub smtp_password_configured: bool,
}

#[derive(Debug, Deserialize)]
struct UpdateAuthSettings {
    registration_enabled: bool,
    registration_require_email_verification: bool,
    smtp_host: String,
    smtp_port: i32,
    smtp_security: String,
    smtp_username: String,
    smtp_password: Option<String>,
    smtp_from_email: String,
    smtp_from_name: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/auth-settings",
            get(get_settings).put(update_settings),
        )
        .route("/admin/auth-settings/test-email", post(test_email))
}

async fn load(db: &sqlx::PgPool) -> Result<AuthSettings, sqlx::Error> {
    sqlx::query_as(
        "SELECT
           registration_enabled,
           registration_require_email_verification,
           smtp_host,
           smtp_port,
           smtp_security,
           smtp_username,
           smtp_from_email,
           smtp_from_name,
           (smtp_password <> '') AS smtp_password_configured
         FROM site_settings WHERE singleton=true",
    )
    .fetch_one(db)
    .await
}

async fn get_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AuthSettings>, StatusCode> {
    require_admin(&state, &headers).await?;
    load(&state.db)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn update_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateAuthSettings>,
) -> Result<Json<AuthSettings>, StatusCode> {
    let actor = require_admin(&state, &headers).await?;

    let host = payload.smtp_host.trim();
    let username = payload.smtp_username.trim();
    let from_email = payload.smtp_from_email.trim().to_lowercase();
    let from_name = payload.smtp_from_name.trim();
    let security = payload.smtp_security.trim().to_ascii_lowercase();

    if host.len() > 320
        || username.len() > 320
        || from_name.len() > 160
        || !(1..=65535).contains(&payload.smtp_port)
        || !matches!(security.as_str(), "starttls" | "tls" | "none")
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    if !from_email.is_empty() && from_email.parse::<lettre::Address>().is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if payload.registration_require_email_verification && (host.is_empty() || from_email.is_empty())
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    let password = payload
        .smtp_password
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());

    sqlx::query(
        "UPDATE site_settings SET
           registration_enabled=$1,
           registration_require_email_verification=$2,
           smtp_host=$3,
           smtp_port=$4,
           smtp_security=$5,
           smtp_username=$6,
           smtp_password=COALESCE($7,smtp_password),
           smtp_from_email=$8,
           smtp_from_name=$9,
           updated_at=now()
         WHERE singleton=true",
    )
    .bind(payload.registration_enabled)
    .bind(payload.registration_require_email_verification)
    .bind(host)
    .bind(payload.smtp_port)
    .bind(&security)
    .bind(username)
    .bind(password)
    .bind(&from_email)
    .bind(from_name)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query(
        "INSERT INTO audit_events(actor_user_id,action,target_type,target_id,metadata)
         VALUES($1,'auth_settings.update','settings','auth',$2)",
    )
    .bind(actor.id)
    .bind(json!({
        "registration_enabled": payload.registration_enabled,
        "registration_require_email_verification": payload.registration_require_email_verification,
        "smtp_host": host,
        "smtp_port": payload.smtp_port,
        "smtp_security": security,
        "smtp_username_configured": !username.is_empty(),
        "smtp_from_email": from_email,
        "smtp_password_changed": password.is_some()
    }))
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    load(&state.db)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn test_email(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = require_admin(&state, &headers).await?;
    email::send_test(&state.db, &actor.email)
        .await
        .map_err(|error| {
            tracing::warn!(?error, actor=%actor.id, "SMTP test failed");
            StatusCode::BAD_GATEWAY
        })?;

    sqlx::query(
        "INSERT INTO audit_events(actor_user_id,action,target_type,target_id,metadata)
         VALUES($1,'smtp.test','settings','smtp',$2)",
    )
    .bind(actor.id)
    .bind(json!({"recipient": actor.email}))
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({"ok":true})))
}
