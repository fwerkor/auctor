use crate::{AppState, auth::current_user, branding};
use axum::{
    Router,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, Response, StatusCode, header},
    routing::get,
};
use md5::{Digest, Md5};
use serde::Deserialize;
use url::Url;
use uuid::Uuid;

const MAX_AVATAR_BYTES: u64 = 5 * 1024 * 1024;

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

    let settings = branding::load_avatar_settings(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if settings.avatar_delivery != "proxy" {
        return Err(StatusCode::NOT_FOUND);
    }

    let email: String = sqlx::query_scalar("SELECT email FROM users WHERE id=$1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let size = query.size.unwrap_or(96).clamp(24, 512);
    let url = avatar_url(&settings.avatar_source_template, &email, size)?;

    let upstream = state
        .http
        .get(url)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !upstream.status().is_success() {
        return Err(StatusCode::NOT_FOUND);
    }

    if upstream
        .content_length()
        .is_some_and(|length| length > MAX_AVATAR_BYTES)
    {
        return Err(StatusCode::BAD_GATEWAY);
    }

    let content_type = upstream
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .filter(|value| value.to_ascii_lowercase().starts_with("image/"))
        .ok_or(StatusCode::BAD_GATEWAY)?
        .to_owned();

    let bytes = upstream
        .bytes()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    if bytes.len() as u64 > MAX_AVATAR_BYTES {
        return Err(StatusCode::BAD_GATEWAY);
    }

    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        "private, max-age=3600".parse().unwrap(),
    );
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        content_type.parse().map_err(|_| StatusCode::BAD_GATEWAY)?,
    );
    Ok(response)
}

pub fn avatar_url(template: &str, email: &str, size: u16) -> Result<Url, StatusCode> {
    branding::validate_avatar_source_template(template)?;

    let normalized = email.trim().to_lowercase();
    let hash = hex::encode(Md5::digest(normalized.as_bytes()));
    let encoded_email: String =
        url::form_urlencoded::byte_serialize(normalized.as_bytes()).collect();

    let rendered = template
        .replace("{email_md5}", &hash)
        .replace("{email}", &encoded_email)
        .replace("{size}", &size.clamp(24, 512).to_string());

    let url = Url::parse(&rendered).map_err(|_| StatusCode::BAD_REQUEST)?;
    if url.scheme() != "https" {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avatar_template_expands_normalized_email() {
        let url = avatar_url(
            "https://avatar.example/{email_md5}?email={email}&size={size}",
            " User@Example.COM ",
            128,
        )
        .unwrap();

        assert_eq!(url.scheme(), "https");
        assert!(url.as_str().contains("email=user%40example.com"));
        assert!(url.as_str().contains("size=128"));
        assert!(!url.as_str().contains('{'));
    }
}
