use crate::{AppState, auth::require_admin};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, sqlx::FromRow)]
struct ReservedUsername {
    username: String,
    note: String,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct AddReservedUsername {
    username: String,
    #[serde(default)]
    note: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/reserved-usernames",
            get(list_reserved).post(add_reserved),
        )
        .route(
            "/admin/reserved-usernames/{username}",
            axum::routing::delete(remove_reserved),
        )
}

pub async fn ensure_allowed(db: &sqlx::PgPool, username: &str) -> Result<(), StatusCode> {
    let normalized = normalize(username)?;
    let reserved: bool = sqlx::query_scalar(
        "SELECT EXISTS(
           SELECT 1 FROM reserved_usernames WHERE username=$1
         )",
    )
    .bind(normalized)
    .fetch_one(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if reserved {
        Err(StatusCode::BAD_REQUEST)
    } else {
        Ok(())
    }
}

async fn list_reserved(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    require_admin(&state, &headers).await?;
    let items: Vec<ReservedUsername> = sqlx::query_as(
        "SELECT username,note,created_at
         FROM reserved_usernames
         ORDER BY username",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({"items": items})))
}

async fn add_reserved(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AddReservedUsername>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let actor = require_admin(&state, &headers).await?;
    let username = normalize(&payload.username)?;
    let note = payload.note.trim();
    if note.len() > 240 {
        return Err(StatusCode::BAD_REQUEST);
    }

    sqlx::query(
        "INSERT INTO reserved_usernames(username,note)
         VALUES($1,$2)
         ON CONFLICT(username) DO UPDATE SET note=EXCLUDED.note",
    )
    .bind(&username)
    .bind(note)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query(
        "INSERT INTO audit_events(actor_user_id,action,target_type,target_id,metadata)
         VALUES($1,'reserved_username.upsert','settings',$2,$3)",
    )
    .bind(actor.id)
    .bind(&username)
    .bind(json!({"username": username, "note": note}))
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(json!({"username": username}))))
}

async fn remove_reserved(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(username): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = require_admin(&state, &headers).await?;
    let username = normalize(&username)?;

    let result = sqlx::query("DELETE FROM reserved_usernames WHERE username=$1")
        .bind(&username)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    sqlx::query(
        "INSERT INTO audit_events(actor_user_id,action,target_type,target_id,metadata)
         VALUES($1,'reserved_username.delete','settings',$2,$3)",
    )
    .bind(actor.id)
    .bind(&username)
    .bind(json!({"username": username}))
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({"ok": true})))
}

fn normalize(value: &str) -> Result<String, StatusCode> {
    let value = value.trim().to_ascii_lowercase();
    let valid = (1..=64).contains(&value.len())
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if valid {
        Ok(value)
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}
