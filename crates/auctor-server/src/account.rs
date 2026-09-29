use crate::{AppState, auth::current_user, db::hash_password};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, patch, put},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Deserialize)]
struct UpdateProfile {
    display_name: String,
}

#[derive(Deserialize)]
struct ChangePassword {
    current_password: String,
    new_password: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct OwnSession {
    id: Uuid,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    last_seen_at: DateTime<Utc>,
    ip: Option<String>,
    user_agent: Option<String>,
    revoked_at: Option<DateTime<Utc>>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/account/profile", patch(update_profile))
        .route("/account/password", put(change_password))
        .route("/account/sessions", get(list_sessions))
        .route("/account/sessions/{id}", delete(revoke_session))
}

async fn update_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateProfile>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = current_user(&state, &headers).await?;
    let display_name = payload.display_name.trim();
    if display_name.is_empty() || display_name.len() > 160 {
        return Err(StatusCode::BAD_REQUEST);
    }

    sqlx::query("UPDATE users SET display_name=$2,updated_at=now() WHERE id=$1")
        .bind(user.id)
        .bind(display_name)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    audit(
        &state,
        user.id,
        "account.profile.update",
        json!({"display_name": display_name}),
    )
    .await?;
    Ok(Json(json!({"ok": true})))
}

async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ChangePassword>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = current_user(&state, &headers).await?;
    let password_hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id=$1")
        .bind(user.id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let parsed =
        PasswordHash::new(&password_hash).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if Argon2::default()
        .verify_password(payload.current_password.as_bytes(), &parsed)
        .is_err()
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    if payload.current_password == payload.new_password {
        return Err(StatusCode::BAD_REQUEST);
    }

    let new_hash = hash_password(&payload.new_password).map_err(|_| StatusCode::BAD_REQUEST)?;
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query("UPDATE users SET password_hash=$2,updated_at=now() WHERE id=$1")
        .bind(user.id)
        .bind(new_hash)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query(
        "UPDATE sessions SET revoked_at=COALESCE(revoked_at,now())
         WHERE user_id=$1 AND revoked_at IS NULL",
    )
    .bind(user.id)
    .execute(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query(
        "INSERT INTO audit_events(actor_user_id,action,target_type,target_id,metadata)
         VALUES($1,'account.password.change','user',$2,'{}'::jsonb)",
    )
    .bind(user.id)
    .bind(user.id.to_string())
    .execute(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({"ok": true, "sessions_revoked": true})))
}

async fn list_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = current_user(&state, &headers).await?;
    let rows: Vec<OwnSession> = sqlx::query_as(
        "SELECT id,created_at,expires_at,last_seen_at,ip::text AS ip,user_agent,revoked_at
         FROM sessions
         WHERE user_id=$1
         ORDER BY last_seen_at DESC
         LIMIT 100",
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let now = Utc::now();
    let items = rows
        .into_iter()
        .map(|row| {
            let active = row.revoked_at.is_none() && row.expires_at > now;
            json!({
                "id": row.id,
                "created_at": row.created_at,
                "expires_at": row.expires_at,
                "last_seen_at": row.last_seen_at,
                "ip": row.ip,
                "user_agent": row.user_agent,
                "revoked_at": row.revoked_at,
                "active": active
            })
        })
        .collect::<Vec<_>>();

    Ok(Json(json!({"items": items})))
}

async fn revoke_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = current_user(&state, &headers).await?;
    let result = sqlx::query(
        "UPDATE sessions SET revoked_at=COALESCE(revoked_at,now())
         WHERE id=$1 AND user_id=$2",
    )
    .bind(session_id)
    .bind(user.id)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    audit(
        &state,
        user.id,
        "account.session.revoke",
        json!({"session_id": session_id}),
    )
    .await?;
    Ok(Json(json!({"ok": true})))
}

async fn audit(
    state: &AppState,
    actor: Uuid,
    action: &str,
    metadata: serde_json::Value,
) -> Result<(), StatusCode> {
    sqlx::query(
        "INSERT INTO audit_events(actor_user_id,action,target_type,target_id,metadata)
         VALUES($1,$2,'user',$3,$4)",
    )
    .bind(actor)
    .bind(action)
    .bind(actor.to_string())
    .bind(metadata)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}
