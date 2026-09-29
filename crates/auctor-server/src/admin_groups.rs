use crate::{AppState, auth::require_admin};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, patch, put},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Serialize, sqlx::FromRow)]
struct GroupRow {
    id: Uuid,
    name: String,
    description: String,
    member_count: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct CreateGroup {
    name: String,
    #[serde(default)]
    description: String,
}

#[derive(Deserialize)]
struct UpdateGroup {
    name: Option<String>,
    description: Option<String>,
}

#[derive(Deserialize)]
struct SetGroups {
    group_ids: Vec<Uuid>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/groups", get(list_groups).post(create_group))
        .route(
            "/admin/groups/{id}",
            patch(update_group).delete(delete_group),
        )
        .route("/admin/users/{id}/groups", put(set_user_groups))
}

async fn list_groups(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    require_admin(&state, &headers).await?;
    let groups: Vec<GroupRow> = sqlx::query_as(
        "SELECT g.id,g.name,g.description,count(ug.user_id)::bigint AS member_count,
                g.created_at,g.updated_at
         FROM groups g
         LEFT JOIN user_groups ug ON ug.group_id=g.id
         GROUP BY g.id
         ORDER BY lower(g.name)",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({"items": groups})))
}

async fn create_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateGroup>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let actor = require_admin(&state, &headers).await?;
    validate_name(&payload.name)?;
    let id: Uuid =
        sqlx::query_scalar("INSERT INTO groups(name,description) VALUES($1,$2) RETURNING id")
            .bind(payload.name.trim())
            .bind(payload.description.trim())
            .fetch_one(&state.db)
            .await
            .map_err(|_| StatusCode::CONFLICT)?;
    audit(
        &state,
        actor.id,
        "group.create",
        "group",
        id,
        json!({"name": payload.name.trim()}),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(json!({"id": id}))))
}

async fn update_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateGroup>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = require_admin(&state, &headers).await?;
    if let Some(name) = &payload.name {
        validate_name(name)?;
    }

    let result = sqlx::query(
        "UPDATE groups
         SET name=COALESCE($2,name),description=COALESCE($3,description),updated_at=now()
         WHERE id=$1",
    )
    .bind(id)
    .bind(payload.name.as_deref().map(str::trim))
    .bind(payload.description.as_deref().map(str::trim))
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::CONFLICT)?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    audit(
        &state,
        actor.id,
        "group.update",
        "group",
        id,
        json!({"name": payload.name, "description": payload.description}),
    )
    .await?;
    Ok(Json(json!({"ok": true})))
}

async fn delete_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = require_admin(&state, &headers).await?;
    let name: Option<String> = sqlx::query_scalar("SELECT name FROM groups WHERE id=$1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let name = name.ok_or(StatusCode::NOT_FOUND)?;

    sqlx::query("DELETE FROM groups WHERE id=$1")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    audit(
        &state,
        actor.id,
        "group.delete",
        "group",
        id,
        json!({"name": name}),
    )
    .await?;
    Ok(Json(json!({"ok": true})))
}

async fn set_user_groups(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<SetGroups>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = require_admin(&state, &headers).await?;

    let user_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id=$1)")
        .bind(user_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !user_exists {
        return Err(StatusCode::NOT_FOUND);
    }

    if !payload.group_ids.is_empty() {
        let found: i64 =
            sqlx::query_scalar("SELECT count(*) FROM groups WHERE id = ANY($1::uuid[])")
                .bind(&payload.group_ids)
                .fetch_one(&state.db)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if found as usize != payload.group_ids.len() {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    sqlx::query("DELETE FROM user_groups WHERE user_id=$1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for group_id in &payload.group_ids {
        sqlx::query("INSERT INTO user_groups(user_id,group_id) VALUES($1,$2)")
            .bind(user_id)
            .bind(group_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    sqlx::query(
        "INSERT INTO audit_events(actor_user_id,action,target_type,target_id,metadata)
         VALUES($1,'user.groups.update','user',$2,$3)",
    )
    .bind(actor.id)
    .bind(user_id.to_string())
    .bind(json!({"group_ids": payload.group_ids}))
    .execute(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({"ok": true})))
}

fn validate_name(name: &str) -> Result<(), StatusCode> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.len() > 120 {
        Err(StatusCode::BAD_REQUEST)
    } else {
        Ok(())
    }
}

async fn audit(
    state: &AppState,
    actor: Uuid,
    action: &str,
    target_type: &str,
    target_id: Uuid,
    metadata: serde_json::Value,
) -> Result<(), StatusCode> {
    sqlx::query(
        "INSERT INTO audit_events(actor_user_id,action,target_type,target_id,metadata)
         VALUES($1,$2,$3,$4,$5)",
    )
    .bind(actor)
    .bind(action)
    .bind(target_type)
    .bind(target_id.to_string())
    .bind(metadata)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}
