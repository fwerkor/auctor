use crate::{AppState, model::SessionUser};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const COOKIE_NAME: &str = "auctor_session";

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct MeResponse {
    id: Uuid,
    session_id: Uuid,
    username: String,
    email: String,
    display_name: String,
    roles: Vec<String>,
    groups: Vec<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/me", get(me))
}

pub async fn current_user(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<SessionUser, StatusCode> {
    let token = cookie_value(headers, COOKIE_NAME).ok_or(StatusCode::UNAUTHORIZED)?;
    let token_hash = Sha256::digest(token.as_bytes()).to_vec();

    let row = sqlx::query_as::<_, (Uuid, String, String, String, Uuid)>(
        "SELECT u.id,u.username,u.email,u.display_name,s.id
         FROM sessions s JOIN users u ON u.id=s.user_id
         WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>now() AND u.status='active'",
    )
    .bind(token_hash)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::UNAUTHORIZED)?;

    let _ = sqlx::query("UPDATE sessions SET last_seen_at=now() WHERE token_hash=$1")
        .bind(Sha256::digest(token.as_bytes()).to_vec())
        .execute(&state.db)
        .await;

    let roles: Vec<String> = sqlx::query_scalar(
        "SELECT r.name FROM roles r JOIN user_roles ur ON ur.role_id=r.id WHERE ur.user_id=$1 ORDER BY r.name",
    )
    .bind(row.0)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let groups: Vec<String> = sqlx::query_scalar(
        "SELECT g.name FROM groups g JOIN user_groups ug ON ug.group_id=g.id WHERE ug.user_id=$1 ORDER BY lower(g.name)",
    )
    .bind(row.0)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(SessionUser {
        id: row.0,
        session_id: row.4,
        username: row.1,
        email: row.2,
        display_name: row.3,
        roles,
        groups,
    })
}

pub async fn require_admin(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<SessionUser, StatusCode> {
    let user = current_user(state, headers).await?;
    if !user.is_admin() {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(user)
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Response {
    let row = match sqlx::query_as::<_, (Uuid, String, String, String, String)>(
        "SELECT id,username,email,display_name,password_hash FROM users
         WHERE (lower(username)=lower($1) OR lower(email)=lower($1)) AND status='active'",
    )
    .bind(payload.username.trim())
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error":"invalid_credentials"})),
            )
                .into_response();
        }
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let parsed = match PasswordHash::new(&row.4) {
        Ok(v) => v,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    if Argon2::default()
        .verify_password(payload.password.as_bytes(), &parsed)
        .is_err()
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error":"invalid_credentials"})),
        )
            .into_response();
    }

    let cookie = match issue_session_cookie(&state, &headers, row.0).await {
        Ok(cookie) => cookie,
        Err(status) => return status.into_response(),
    };
    let mut response = Json(serde_json::json!({"ok":true})).into_response();
    response.headers_mut().insert(header::SET_COOKIE, cookie);
    response
}

pub async fn issue_session_cookie(
    state: &AppState,
    headers: &HeaderMap,
    user_id: Uuid,
) -> Result<HeaderValue, StatusCode> {
    let mut raw = [0u8; 32];
    rand::rng().fill_bytes(&mut raw);
    let token = URL_SAFE_NO_PAD.encode(raw);
    let hash = Sha256::digest(token.as_bytes()).to_vec();
    let expires = Utc::now() + Duration::hours(state.session_hours);
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);

    sqlx::query(
        "INSERT INTO sessions(user_id,token_hash,expires_at,user_agent) VALUES ($1,$2,$3,$4)",
    )
    .bind(user_id)
    .bind(hash)
    .bind(expires)
    .bind(user_agent)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        COOKIE_NAME,
        token,
        state.session_hours * 3600
    );
    if state.secure_cookies {
        cookie.push_str("; Secure");
    }
    HeaderValue::from_str(&cookie).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(token) = cookie_value(&headers, COOKIE_NAME) {
        let hash = Sha256::digest(token.as_bytes()).to_vec();
        let _ = sqlx::query("UPDATE sessions SET revoked_at=now() WHERE token_hash=$1")
            .bind(hash)
            .execute(&state.db)
            .await;
    }
    let mut response = Json(serde_json::json!({"ok":true})).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static("auctor_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0"),
    );
    response
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, StatusCode> {
    let user = current_user(&state, &headers).await?;
    Ok(Json(MeResponse {
        id: user.id,
        session_id: user.session_id,
        username: user.username,
        email: user.email,
        display_name: user.display_name,
        roles: user.roles,
        groups: user.groups,
    }))
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .map(str::trim)
        .find_map(|part| {
            let (key, value) = part.split_once('=')?;
            (key == name).then(|| value.to_owned())
        })
}
