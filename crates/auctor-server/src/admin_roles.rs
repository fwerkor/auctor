use crate::{AppState, auth::require_admin};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, patch},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

const PROTECTED_ROLES: &[&str] = &["user", "platform-admin"];

#[derive(Serialize, sqlx::FromRow)]
struct RoleRow {
    id: Uuid,
    name: String,
    description: String,
    user_count: i64,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct CreateRole {
    name: String,
    #[serde(default)]
    description: String,
}

#[derive(Deserialize)]
struct UpdateRole {
    name: Option<String>,
    description: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/roles", get(list_roles).post(create_role))
        .route("/admin/roles/{id}", patch(update_role).delete(delete_role))
}

async fn list_roles(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    require_admin(&state, &headers).await?;
    let rows: Vec<RoleRow> = sqlx::query_as(
        "SELECT r.id,r.name,r.description,count(ur.user_id)::bigint AS user_count,r.created_at
         FROM roles r
         LEFT JOIN user_roles ur ON ur.role_id=r.id
         GROUP BY r.id
         ORDER BY CASE WHEN r.name='platform-admin' THEN 0 WHEN r.name='user' THEN 1 ELSE 2 END,
                  lower(r.name)",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({"items": rows})))
}

async fn create_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateRole>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let actor = require_admin(&state, &headers).await?;
    validate_name(&payload.name)?;

    let name = payload.name.trim();
    let id: Uuid =
        sqlx::query_scalar("INSERT INTO roles(name,description) VALUES($1,$2) RETURNING id")
            .bind(name)
            .bind(payload.description.trim())
            .fetch_one(&state.db)
            .await
            .map_err(|_| StatusCode::CONFLICT)?;

    audit(&state, actor.id, "role.create", id, json!({"name": name})).await?;
    Ok((StatusCode::CREATED, Json(json!({"id": id}))))
}

async fn update_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateRole>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = require_admin(&state, &headers).await?;

    let current_name: Option<String> = sqlx::query_scalar("SELECT name FROM roles WHERE id=$1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let current_name = current_name.ok_or(StatusCode::NOT_FOUND)?;

    if let Some(name) = &payload.name {
        validate_name(name)?;
        if PROTECTED_ROLES.contains(&current_name.as_str()) && name.trim() != current_name {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    sqlx::query(
        "UPDATE roles
         SET name=COALESCE($2,name),description=COALESCE($3,description)
         WHERE id=$1",
    )
    .bind(id)
    .bind(payload.name.as_deref().map(str::trim))
    .bind(payload.description.as_deref().map(str::trim))
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::CONFLICT)?;

    audit(
        &state,
        actor.id,
        "role.update",
        id,
        json!({"name": payload.name, "description": payload.description}),
    )
    .await?;
    Ok(Json(json!({"ok": true})))
}

async fn delete_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = require_admin(&state, &headers).await?;

    let name: Option<String> = sqlx::query_scalar("SELECT name FROM roles WHERE id=$1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let name = name.ok_or(StatusCode::NOT_FOUND)?;
    if PROTECTED_ROLES.contains(&name.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    sqlx::query("DELETE FROM roles WHERE id=$1")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    audit(&state, actor.id, "role.delete", id, json!({"name": name})).await?;
    Ok(Json(json!({"ok": true})))
}

fn validate_name(name: &str) -> Result<(), StatusCode> {
    let name = name.trim();
    let valid = !name.is_empty()
        && name.len() <= 80
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'));
    if valid {
        Ok(())
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

async fn audit(
    state: &AppState,
    actor: Uuid,
    action: &str,
    target_id: Uuid,
    metadata: serde_json::Value,
) -> Result<(), StatusCode> {
    sqlx::query(
        "INSERT INTO audit_events(actor_user_id,action,target_type,target_id,metadata)
         VALUES($1,$2,'role',$3,$4)",
    )
    .bind(actor)
    .bind(action)
    .bind(target_id.to_string())
    .bind(metadata)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}
