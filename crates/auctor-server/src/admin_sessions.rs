use crate::{AppState, auth::require_admin};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get},
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Serialize, sqlx::FromRow)]
struct SessionRow {
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
        .route("/admin/users/{id}/sessions", get(list_user_sessions))
        .route("/admin/sessions/{id}", delete(revoke_session))
}

async fn list_user_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    require_admin(&state, &headers).await?;

    let user_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id=$1)")
        .bind(user_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !user_exists {
        return Err(StatusCode::NOT_FOUND);
    }

    let rows: Vec<SessionRow> = sqlx::query_as(
        "SELECT id,created_at,expires_at,last_seen_at,ip::text AS ip,user_agent,revoked_at
         FROM sessions
         WHERE user_id=$1
         ORDER BY last_seen_at DESC
         LIMIT 100",
    )
    .bind(user_id)
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
    let actor = require_admin(&state, &headers).await?;

    let user_id: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM sessions WHERE id=$1")
        .bind(session_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let user_id = user_id.ok_or(StatusCode::NOT_FOUND)?;

    sqlx::query("UPDATE sessions SET revoked_at=COALESCE(revoked_at,now()) WHERE id=$1")
        .bind(session_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query(
        "INSERT INTO audit_events(actor_user_id,action,target_type,target_id,metadata)
         VALUES($1,'session.revoke','session',$2,$3)",
    )
    .bind(actor.id)
    .bind(session_id.to_string())
    .bind(json!({"user_id": user_id}))
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({"ok": true})))
}
