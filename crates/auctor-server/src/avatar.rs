use crate::{AppState, auth::current_user};
use axum::{
    Router,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, Response, StatusCode, header},
    routing::get,
};
use md5::{Digest, Md5};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
struct AvatarQuery {
    size: Option<u16>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/avatar/user/{id}", get(user_avatar))
}

async fn user_avatar(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Query(query): Query<AvatarQuery>,
) -> Result<Response<Body>, StatusCode> {
    current_user(&state, &headers).await?;
    let email: String = sqlx::query_scalar("SELECT email FROM users WHERE id=$1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let normalized = email.trim().to_lowercase();
    let hash = hex::encode(Md5::digest(normalized.as_bytes()));
    let size = query.size.unwrap_or(96).clamp(24, 512);
    let url = format!("https://www.gravatar.com/avatar/{}?s={}&d=404", hash, size);
    let upstream = state
        .http
        .get(url)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    if !upstream.status().is_success() {
        return Err(StatusCode::NOT_FOUND);
    }
    let content_type = upstream.headers().get(header::CONTENT_TYPE).cloned();
    let bytes = upstream
        .bytes()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        "public, max-age=3600".parse().unwrap(),
    );
    if let Some(value) = content_type {
        response.headers_mut().insert(header::CONTENT_TYPE, value);
    }
    Ok(response)
}
