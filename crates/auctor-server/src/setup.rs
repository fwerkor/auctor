use crate::{AppState, db::hash_password};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Serialize)]
struct SetupStatus {
    required: bool,
}

#[derive(Deserialize)]
struct SetupRequest {
    setup_token: String,
    username: String,
    email: String,
    display_name: String,
    password: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/setup/status", get(status))
        .route("/setup", post(create_first_admin))
}

async fn status(State(state): State<AppState>) -> Result<Json<SetupStatus>, StatusCode> {
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(SetupStatus {
        required: count == 0,
    }))
}

async fn create_first_admin(
    State(state): State<AppState>,
    Json(payload): Json<SetupRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let expected = state.setup_token.as_ref().ok_or(StatusCode::CONFLICT)?;
    let supplied_hash = Sha256::digest(payload.setup_token.trim().as_bytes());
    let expected_hash = Sha256::digest(expected.as_bytes());
    if supplied_hash != expected_hash {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let username = payload.username.trim();
    let email = payload.email.trim();
    let display_name = payload.display_name.trim();

    validate_identity(username, email, display_name)?;
    let password_hash = hash_password(&payload.password).map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if count != 0 {
        return Err(StatusCode::CONFLICT);
    }

    let user_id: Uuid = sqlx::query_scalar(
        "INSERT INTO users(username,email,display_name,password_hash) VALUES($1,$2,$3,$4) RETURNING id",
    )
    .bind(username)
    .bind(email.to_lowercase())
    .bind(display_name)
    .bind(password_hash)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| StatusCode::CONFLICT)?;

    sqlx::query(
        "INSERT INTO user_roles(user_id,role_id)
         SELECT $1,id FROM roles WHERE name IN ('user','platform-admin')",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query(
        "INSERT INTO audit_events(actor_user_id,action,target_type,target_id,metadata)
         VALUES($1,'setup.first_admin','user',$2,$3)",
    )
    .bind(user_id)
    .bind(user_id.to_string())
    .bind(json!({"username": username}))
    .execute(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = std::fs::remove_file(&state.setup_token_path);
    Ok((StatusCode::CREATED, Json(json!({"id": user_id}))))
}

fn validate_identity(username: &str, email: &str, display_name: &str) -> Result<(), StatusCode> {
    let username_ok = username.len() >= 3
        && username.len() <= 64
        && username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    let email_ok = email.contains('@') && email.len() <= 320;
    let name_ok = !display_name.trim().is_empty() && display_name.len() <= 160;

    if username_ok && email_ok && name_ok {
        Ok(())
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}
