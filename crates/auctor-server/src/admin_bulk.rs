use crate::{AppState, auth::require_admin};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Deserialize)]
struct BulkUsersRequest {
    ids: Vec<Uuid>,
    action: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/users/bulk", post(bulk_users))
}

async fn bulk_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<BulkUsersRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = require_admin(&state, &headers).await?;

    if payload.ids.is_empty() || payload.ids.len() > 200 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut ids = payload.ids;
    ids.sort_unstable();
    ids.dedup();

    let affected = match payload.action.as_str() {
        "activate" => sqlx::query(
            "UPDATE users SET status='active',updated_at=now() WHERE id=ANY($1::uuid[])",
        )
        .bind(&ids)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .rows_affected(),
        "disable" => {
            if ids.contains(&actor.id) {
                return Err(StatusCode::BAD_REQUEST);
            }
            sqlx::query(
                "UPDATE users SET status='disabled',updated_at=now() WHERE id=ANY($1::uuid[])",
            )
            .bind(&ids)
            .execute(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .rows_affected()
        }
        "revoke_sessions" => sqlx::query(
            "UPDATE sessions SET revoked_at=COALESCE(revoked_at,now())
             WHERE user_id=ANY($1::uuid[]) AND revoked_at IS NULL",
        )
        .bind(&ids)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .rows_affected(),
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    sqlx::query(
        "INSERT INTO audit_events(actor_user_id,action,target_type,metadata)
         VALUES($1,$2,'user_batch',$3)",
    )
    .bind(actor.id)
    .bind(format!("users.bulk.{}", payload.action))
    .bind(json!({"user_ids": ids, "affected": affected}))
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({"ok": true, "affected": affected})))
}
