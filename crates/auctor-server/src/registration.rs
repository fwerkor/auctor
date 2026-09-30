use crate::{AppState, db::hash_password, username_policy, verification};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Serialize)]
struct RegistrationConfig {
    enabled: bool,
    require_email_verification: bool,
}

#[derive(Deserialize)]
struct RegisterRequest {
    username: String,
    email: String,
    display_name: String,
    password: String,
}

#[derive(Deserialize)]
struct EmailCodeRequest {
    email: String,
    code: String,
}

#[derive(Deserialize)]
struct EmailRequest {
    email: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/registration-config", get(config))
        .route("/auth/register", post(register))
        .route("/auth/register/verify", post(verify))
        .route("/auth/register/resend", post(resend))
}

async fn config(State(state): State<AppState>) -> Result<Json<RegistrationConfig>, StatusCode> {
    let (enabled, require_email_verification) = sqlx::query_as::<_, (bool, bool)>(
        "SELECT registration_enabled,registration_require_email_verification
             FROM site_settings WHERE singleton=true",
    )
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(RegistrationConfig {
        enabled,
        require_email_verification,
    }))
}

async fn register(State(state): State<AppState>, Json(payload): Json<RegisterRequest>) -> Response {
    let (enabled, require_verification) = match sqlx::query_as::<_, (bool, bool)>(
        "SELECT registration_enabled,registration_require_email_verification
         FROM site_settings WHERE singleton=true",
    )
    .fetch_one(&state.db)
    .await
    {
        Ok(v) => v,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    if !enabled {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error":"registration_disabled"})),
        )
            .into_response();
    }

    let username = payload.username.trim();
    let email = payload.email.trim().to_lowercase();
    let display_name = payload.display_name.trim();
    if validate_identity(username, &email, display_name).is_err() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"invalid_identity"})),
        )
            .into_response();
    }
    if username_policy::ensure_allowed(&state.db, username)
        .await
        .is_err()
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"username_unavailable"})),
        )
            .into_response();
    }
    let password_hash = match hash_password(&payload.password) {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error":"weak_password"})),
            )
                .into_response();
        }
    };

    let status = if require_verification {
        "pending_email"
    } else {
        "active"
    };
    let mut tx = match state.db.begin().await {
        Ok(v) => v,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let user_id: Uuid = match sqlx::query_scalar(
        "INSERT INTO users(username,email,display_name,password_hash,status,email_verified_at)
         VALUES($1,lower($2),$3,$4,$5,CASE WHEN $5='active' THEN now() ELSE NULL END)
         RETURNING id",
    )
    .bind(username)
    .bind(&email)
    .bind(display_name)
    .bind(password_hash)
    .bind(status)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(id) => id,
        Err(error) => {
            tracing::warn!(?error, "registration insert rejected");
            let _ = tx.rollback().await;
            return (
                StatusCode::CONFLICT,
                Json(json!({"error":"identity_exists"})),
            )
                .into_response();
        }
    };

    if sqlx::query(
        "INSERT INTO user_roles(user_id,role_id)
         SELECT $1,id FROM roles WHERE name='user'",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .is_err()
    {
        let _ = tx.rollback().await;
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    if sqlx::query(
        "INSERT INTO audit_events(action,target_type,target_id,metadata)
         VALUES('registration.create','user',$1,$2)",
    )
    .bind(user_id.to_string())
    .bind(json!({"email_verification_required": require_verification}))
    .execute(&mut *tx)
    .await
    .is_err()
    {
        let _ = tx.rollback().await;
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    if tx.commit().await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    if require_verification {
        if let Err(error) =
            verification::issue_code(&state.db, user_id, &email, "registration").await
        {
            tracing::error!(?error, user_id=%user_id, "could not send registration verification email");
            let _ = sqlx::query("DELETE FROM users WHERE id=$1 AND status='pending_email'")
                .bind(user_id)
                .execute(&state.db)
                .await;
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error":"verification_email_unavailable"})),
            )
                .into_response();
        }
    }

    (
        StatusCode::CREATED,
        Json(json!({
            "id": user_id,
            "verification_required": require_verification
        })),
    )
        .into_response()
}

async fn verify(State(state): State<AppState>, Json(payload): Json<EmailCodeRequest>) -> Response {
    let email = payload.email.trim().to_lowercase();
    let row = match sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id,status FROM users WHERE lower(email)=lower($1)",
    )
    .bind(&email)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(v)) => v,
        Ok(None) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error":"invalid_code"})),
            )
                .into_response();
        }
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    if row.1 != "pending_email" {
        return Json(json!({"ok":true})).into_response();
    }

    match verification::consume_code(&state.db, row.0, &email, "registration", &payload.code).await
    {
        Ok(true) => {}
        Ok(false) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error":"invalid_code"})),
            )
                .into_response();
        }
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }

    let mut tx = match state.db.begin().await {
        Ok(v) => v,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    if sqlx::query(
        "UPDATE users
         SET status='active',email_verified_at=now(),updated_at=now()
         WHERE id=$1 AND status='pending_email'",
    )
    .bind(row.0)
    .execute(&mut *tx)
    .await
    .is_err()
    {
        let _ = tx.rollback().await;
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    if sqlx::query(
        "INSERT INTO audit_events(action,target_type,target_id)
         VALUES('registration.email.verify','user',$1)",
    )
    .bind(row.0.to_string())
    .execute(&mut *tx)
    .await
    .is_err()
    {
        let _ = tx.rollback().await;
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    if tx.commit().await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    Json(json!({"ok":true})).into_response()
}

async fn resend(State(state): State<AppState>, Json(payload): Json<EmailRequest>) -> Response {
    let email = payload.email.trim().to_lowercase();
    let user_id: Option<Uuid> = match sqlx::query_scalar(
        "SELECT id FROM users WHERE lower(email)=lower($1) AND status='pending_email'",
    )
    .bind(&email)
    .fetch_optional(&state.db)
    .await
    {
        Ok(v) => v,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    if let Some(user_id) = user_id {
        if let Err(error) =
            verification::issue_code(&state.db, user_id, &email, "registration").await
        {
            tracing::error!(?error, user_id=%user_id, "could not resend verification email");
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error":"verification_email_unavailable"})),
            )
                .into_response();
        }
    }

    Json(json!({"ok":true})).into_response()
}

fn validate_identity(username: &str, email: &str, display_name: &str) -> Result<(), ()> {
    let username_ok = username.len() >= 3
        && username.len() <= 64
        && username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    let email_ok = email.len() <= 320 && email.parse::<lettre::Address>().is_ok();
    let name_ok = !display_name.is_empty() && display_name.len() <= 160;
    if username_ok && email_ok && name_ok {
        Ok(())
    } else {
        Err(())
    }
}
