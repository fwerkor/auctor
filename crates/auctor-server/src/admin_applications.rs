use crate::{AppState, auth::require_admin};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use url::Url;
use uuid::Uuid;

#[derive(Serialize, sqlx::FromRow)]
struct ApplicationRow {
    id: Uuid,
    client_id: String,
    name: String,
    app_type: String,
    redirect_uris: Value,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct CreateApplication {
    name: String,
    client_id: Option<String>,
    app_type: String,
    #[serde(default)]
    redirect_uris: Vec<String>,
}

#[derive(Deserialize)]
struct UpdateApplication {
    name: Option<String>,
    client_id: Option<String>,
    app_type: Option<String>,
    redirect_uris: Option<Vec<String>>,
    status: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/applications",
            get(list_applications).post(create_application),
        )
        .route(
            "/admin/applications/{id}",
            get(get_application)
                .patch(update_application)
                .delete(delete_application),
        )
}

async fn list_applications(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    require_admin(&state, &headers).await?;
    let rows: Vec<ApplicationRow> = sqlx::query_as(
        "SELECT id,client_id,name,app_type,redirect_uris,status,created_at,updated_at
         FROM applications
         ORDER BY lower(name),created_at",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({"items": rows})))
}

async fn get_application(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApplicationRow>, StatusCode> {
    require_admin(&state, &headers).await?;
    let row: ApplicationRow = sqlx::query_as(
        "SELECT id,client_id,name,app_type,redirect_uris,status,created_at,updated_at
         FROM applications WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(row))
}

async fn create_application(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateApplication>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let actor = require_admin(&state, &headers).await?;
    validate_name(&payload.name)?;
    validate_type(&payload.app_type)?;
    validate_redirect_uris(&payload.redirect_uris)?;

    let client_id = match payload.client_id.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => {
            validate_client_id(value)?;
            value.to_owned()
        }
        _ => format!("auc_{}", Uuid::new_v4().simple()),
    };

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO applications(client_id,name,app_type,redirect_uris)
         VALUES($1,$2,$3,$4) RETURNING id",
    )
    .bind(&client_id)
    .bind(payload.name.trim())
    .bind(payload.app_type.trim())
    .bind(json!(payload.redirect_uris))
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::CONFLICT)?;

    audit(
        &state,
        actor.id,
        "application.create",
        id,
        json!({"name": payload.name, "client_id": client_id}),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({"id": id, "client_id": client_id})),
    ))
}

async fn update_application(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateApplication>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = require_admin(&state, &headers).await?;

    if let Some(name) = &payload.name {
        validate_name(name)?;
    }
    if let Some(client_id) = &payload.client_id {
        validate_client_id(client_id.trim())?;
    }
    if let Some(app_type) = &payload.app_type {
        validate_type(app_type)?;
    }
    if let Some(redirect_uris) = &payload.redirect_uris {
        validate_redirect_uris(redirect_uris)?;
    }
    if let Some(status) = &payload.status {
        if status != "active" && status != "disabled" {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let result = sqlx::query(
        "UPDATE applications SET
           name=COALESCE($2,name),
           client_id=COALESCE($3,client_id),
           app_type=COALESCE($4,app_type),
           redirect_uris=COALESCE($5,redirect_uris),
           status=COALESCE($6,status),
           updated_at=now()
         WHERE id=$1",
    )
    .bind(id)
    .bind(payload.name.as_deref().map(str::trim))
    .bind(payload.client_id.as_deref().map(str::trim))
    .bind(payload.app_type.as_deref().map(str::trim))
    .bind(payload.redirect_uris.as_ref().map(|uris| json!(uris)))
    .bind(payload.status.as_deref())
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::CONFLICT)?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    audit(
        &state,
        actor.id,
        "application.update",
        id,
        json!({
            "name": payload.name,
            "client_id": payload.client_id,
            "app_type": payload.app_type,
            "redirect_uris": payload.redirect_uris,
            "status": payload.status
        }),
    )
    .await?;

    Ok(Json(json!({"ok": true})))
}

async fn delete_application(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = require_admin(&state, &headers).await?;
    let app: Option<(String, String)> =
        sqlx::query_as("SELECT name,client_id FROM applications WHERE id=$1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let (name, client_id) = app.ok_or(StatusCode::NOT_FOUND)?;

    sqlx::query("DELETE FROM applications WHERE id=$1")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    audit(
        &state,
        actor.id,
        "application.delete",
        id,
        json!({"name": name, "client_id": client_id}),
    )
    .await?;
    Ok(Json(json!({"ok": true})))
}

fn validate_name(name: &str) -> Result<(), StatusCode> {
    let name = name.trim();
    if name.is_empty() || name.len() > 160 {
        Err(StatusCode::BAD_REQUEST)
    } else {
        Ok(())
    }
}

fn validate_client_id(client_id: &str) -> Result<(), StatusCode> {
    let valid = (3..=160).contains(&client_id.len())
        && client_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'));
    if valid {
        Ok(())
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

fn validate_type(app_type: &str) -> Result<(), StatusCode> {
    if matches!(app_type.trim(), "web" | "native" | "service") {
        Ok(())
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

fn validate_redirect_uris(uris: &[String]) -> Result<(), StatusCode> {
    if uris.len() > 32 {
        return Err(StatusCode::BAD_REQUEST);
    }

    for raw in uris {
        if raw.len() > 2048 {
            return Err(StatusCode::BAD_REQUEST);
        }
        let url = Url::parse(raw).map_err(|_| StatusCode::BAD_REQUEST)?;
        if url.fragment().is_some() || !url.username().is_empty() || url.password().is_some() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let secure = url.scheme() == "https";
        let local_http = url.scheme() == "http"
            && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
        if !secure && !local_http {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    Ok(())
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
         VALUES($1,$2,'application',$3,$4)",
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
