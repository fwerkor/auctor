use crate::{AppState, db::hash_password, verification};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Deserialize)]
struct RequestReset {
    email: String,
}

#[derive(Deserialize)]
struct ConfirmReset {
    email: String,
    code: String,
    new_password: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/password/forgot", post(forgot))
        .route("/auth/password/reset", post(reset))
}

async fn forgot(State(state): State<AppState>, Json(payload): Json<RequestReset>) -> Response {
    let email = payload.email.trim().to_lowercase();
    if email.is_empty() || email.len() > 320 {
        return Json(json!({"ok":true})).into_response();
    }

    let user_id: Option<Uuid> = match sqlx::query_scalar(
        "SELECT id FROM users WHERE lower(email)=lower($1) AND status='active'",
    )
    .bind(&email)
    .fetch_optional(&state.db)
    .await
    {
        Ok(v) => v,
        Err(error) => {
            tracing::error!(?error, "password recovery lookup failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if let Some(user_id) = user_id
        && let Err(error) =
            verification::issue_code(&state.db, user_id, &email, "password_reset").await
    {
        tracing::error!(?error, user_id=%user_id, "could not send password recovery email");
    }

    // Always return the same response so callers cannot enumerate accounts.
    Json(json!({"ok":true})).into_response()
}

async fn reset(State(state): State<AppState>, Json(payload): Json<ConfirmReset>) -> Response {
    let email = payload.email.trim().to_lowercase();
    let password_hash = match hash_password(&payload.new_password) {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error":"weak_password"})),
            )
                .into_response();
        }
    };

    let user_id: Option<Uuid> = match sqlx::query_scalar(
        "SELECT id FROM users WHERE lower(email)=lower($1) AND status='active'",
    )
    .bind(&email)
    .fetch_optional(&state.db)
    .await
    {
        Ok(v) => v,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let Some(user_id) = user_id else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"invalid_code"})),
        )
            .into_response();
    };

    match verification::consume_code(&state.db, user_id, &email, "password_reset", &payload.code)
        .await
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

    if sqlx::query("UPDATE users SET password_hash=$2,updated_at=now() WHERE id=$1")
        .bind(user_id)
        .bind(password_hash)
        .execute(&mut *tx)
        .await
        .is_err()
    {
        let _ = tx.rollback().await;
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    if sqlx::query(
        "UPDATE sessions SET revoked_at=COALESCE(revoked_at,now())
         WHERE user_id=$1 AND revoked_at IS NULL",
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
        "UPDATE oauth_access_tokens SET revoked_at=COALESCE(revoked_at,now())
         WHERE user_id=$1 AND revoked_at IS NULL",
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
        "UPDATE oauth_authorization_codes SET used_at=COALESCE(used_at,now())
         WHERE user_id=$1 AND used_at IS NULL",
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
        "UPDATE oauth_authorization_codes SET used_at=COALESCE(used_at,now())
         WHERE user_id=$1 AND used_at IS NULL",
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
        "INSERT INTO audit_events(action,target_type,target_id)
         VALUES('account.password.recover','user',$1)",
    )
    .bind(user_id.to_string())
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
