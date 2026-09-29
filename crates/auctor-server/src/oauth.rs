use crate::{AppState, auth::current_user, branding};
use axum::{
    Form, Json, Router,
    extract::{OriginalUri, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;

const CODE_TTL_MINUTES: i64 = 5;
const TOKEN_TTL_SECONDS: i64 = 3600;
const ALLOWED_SCOPES: &[&str] = &["profile", "email", "groups", "roles"];

#[derive(Deserialize)]
struct AuthorizeQuery {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    state: Option<String>,
    scope: Option<String>,
    code_challenge: String,
    code_challenge_method: String,
}

#[derive(Deserialize)]
struct TokenRequest {
    grant_type: String,
    code: String,
    client_id: String,
    redirect_uri: String,
    code_verifier: String,
}

#[derive(sqlx::FromRow)]
struct Application {
    id: Uuid,
    redirect_uris: Value,
    status: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/oauth/authorize", get(authorize))
        .route("/oauth/token", post(token))
        .route("/oauth/userinfo", get(userinfo))
        .route("/.well-known/oauth-authorization-server", get(metadata))
}

async fn metadata(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    let brand = branding::load(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if brand.site_url.is_empty() {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    Ok(Json(json!({
        "issuer": brand.site_url,
        "authorization_endpoint": format!("{}/oauth/authorize", brand.site_url),
        "token_endpoint": format!("{}/oauth/token", brand.site_url),
        "userinfo_endpoint": format!("{}/oauth/userinfo", brand.site_url),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
        "scopes_supported": ALLOWED_SCOPES,
    })))
}

async fn authorize(
    State(state): State<AppState>,
    headers: HeaderMap,
    OriginalUri(original_uri): OriginalUri,
    Query(query): Query<AuthorizeQuery>,
) -> Response {
    if query.response_type != "code" || query.code_challenge_method != "S256" {
        return oauth_error(StatusCode::BAD_REQUEST, "unsupported_request");
    }
    if !valid_pkce_challenge(&query.code_challenge) {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request");
    }

    let application = match load_application(&state, &query.client_id).await {
        Ok(app) => app,
        Err(status) => return oauth_error(status, "invalid_client"),
    };
    if application.status != "active"
        || !redirect_allowed(&application.redirect_uris, &query.redirect_uri)
    {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request");
    }

    let scope = match normalize_scope(query.scope.as_deref()) {
        Ok(scope) => scope,
        Err(status) => return oauth_error(status, "invalid_scope"),
    };

    let user = match current_user(&state, &headers).await {
        Ok(user) => user,
        Err(StatusCode::UNAUTHORIZED) => {
            let target = original_uri
                .path_and_query()
                .map(|value| value.as_str())
                .unwrap_or("/oauth/authorize");
            let mut serializer = url::form_urlencoded::Serializer::new(String::from("/?"));
            serializer.append_pair("continue", target);
            return Redirect::to(&serializer.finish()).into_response();
        }
        Err(status) => return status.into_response(),
    };

    let raw_code = random_token();
    let code_hash = Sha256::digest(raw_code.as_bytes()).to_vec();
    let expires_at = Utc::now() + Duration::minutes(CODE_TTL_MINUTES);

    if sqlx::query(
        "INSERT INTO oauth_authorization_codes
         (code_hash,application_id,user_id,redirect_uri,scope,code_challenge,code_challenge_method,expires_at)
         VALUES($1,$2,$3,$4,$5,$6,'S256',$7)",
    )
    .bind(code_hash)
    .bind(application.id)
    .bind(user.id)
    .bind(&query.redirect_uri)
    .bind(&scope)
    .bind(&query.code_challenge)
    .bind(expires_at)
    .execute(&state.db)
    .await
    .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let mut redirect = match Url::parse(&query.redirect_uri) {
        Ok(url) => url,
        Err(_) => return oauth_error(StatusCode::BAD_REQUEST, "invalid_request"),
    };
    {
        let mut pairs = redirect.query_pairs_mut();
        pairs.append_pair("code", &raw_code);
        if let Some(state_value) = &query.state {
            pairs.append_pair("state", state_value);
        }
    }
    Redirect::to(redirect.as_str()).into_response()
}

async fn token(State(state): State<AppState>, Form(payload): Form<TokenRequest>) -> Response {
    if payload.grant_type != "authorization_code" || !valid_pkce_verifier(&payload.code_verifier) {
        return token_error(StatusCode::BAD_REQUEST, "invalid_request");
    }

    let code_hash = Sha256::digest(payload.code.as_bytes()).to_vec();
    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let row = match sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            Uuid,
            String,
            String,
            String,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            String,
            String,
        ),
    >(
        "SELECT c.id,c.application_id,c.user_id,c.redirect_uri,c.scope,c.code_challenge,
                c.expires_at,c.used_at,a.client_id,a.status
         FROM oauth_authorization_codes c
         JOIN applications a ON a.id=c.application_id
         WHERE c.code_hash=$1
         FOR UPDATE",
    )
    .bind(code_hash)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => return token_error(StatusCode::BAD_REQUEST, "invalid_grant"),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    if row.7.is_some()
        || row.6 <= Utc::now()
        || row.8 != payload.client_id
        || row.9 != "active"
        || row.3 != payload.redirect_uri
        || !verify_pkce(&payload.code_verifier, &row.5)
    {
        return token_error(StatusCode::BAD_REQUEST, "invalid_grant");
    }

    let raw_token = random_token();
    let access_hash = Sha256::digest(raw_token.as_bytes()).to_vec();
    let expires_at = Utc::now() + Duration::seconds(TOKEN_TTL_SECONDS);

    if sqlx::query("UPDATE oauth_authorization_codes SET used_at=now() WHERE id=$1")
        .bind(row.0)
        .execute(&mut *tx)
        .await
        .is_err()
        || sqlx::query(
            "INSERT INTO oauth_access_tokens
             (token_hash,application_id,user_id,scope,expires_at)
             VALUES($1,$2,$3,$4,$5)",
        )
        .bind(access_hash)
        .bind(row.1)
        .bind(row.2)
        .bind(&row.4)
        .bind(expires_at)
        .execute(&mut *tx)
        .await
        .is_err()
        || tx.commit().await.is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    Json(json!({
        "access_token": raw_token,
        "token_type": "Bearer",
        "expires_in": TOKEN_TTL_SECONDS,
        "scope": row.4,
    }))
    .into_response()
}

async fn userinfo(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let bearer = match headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
    {
        Some(token) if !token.is_empty() => token,
        _ => return bearer_error(StatusCode::UNAUTHORIZED, "invalid_token"),
    };

    let token_hash = Sha256::digest(bearer.as_bytes()).to_vec();
    let row = match sqlx::query_as::<_, (Uuid, String, String, String, String)>(
        "SELECT u.id,u.username,u.email,u.display_name,t.scope
         FROM oauth_access_tokens t
         JOIN users u ON u.id=t.user_id
         JOIN applications a ON a.id=t.application_id
         WHERE t.token_hash=$1
           AND t.revoked_at IS NULL
           AND t.expires_at>now()
           AND u.status='active'
           AND a.status='active'",
    )
    .bind(token_hash)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => return bearer_error(StatusCode::UNAUTHORIZED, "invalid_token"),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let scopes: Vec<&str> = row.4.split_whitespace().collect();
    let mut body = json!({
        "sub": row.0.to_string(),
        "preferred_username": row.1,
        "name": row.3,
    });

    if scopes.contains(&"email") {
        body["email"] = json!(row.2);
    }
    if scopes.contains(&"roles") {
        let roles: Vec<String> = sqlx::query_scalar(
            "SELECT r.name FROM roles r
             JOIN user_roles ur ON ur.role_id=r.id
             WHERE ur.user_id=$1 ORDER BY r.name",
        )
        .bind(row.0)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
        body["roles"] = json!(roles);
    }
    if scopes.contains(&"groups") {
        let groups: Vec<String> = sqlx::query_scalar(
            "SELECT g.name FROM groups g
             JOIN user_groups ug ON ug.group_id=g.id
             WHERE ug.user_id=$1 ORDER BY lower(g.name)",
        )
        .bind(row.0)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
        body["groups"] = json!(groups);
    }

    Json(body).into_response()
}

async fn load_application(state: &AppState, client_id: &str) -> Result<Application, StatusCode> {
    sqlx::query_as(
        "SELECT id,redirect_uris,status
         FROM applications WHERE client_id=$1",
    )
    .bind(client_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::BAD_REQUEST)
}

fn redirect_allowed(value: &Value, requested: &str) -> bool {
    serde_json::from_value::<Vec<String>>(value.clone())
        .map(|uris| uris.iter().any(|uri| uri == requested))
        .unwrap_or(false)
}

fn normalize_scope(raw: Option<&str>) -> Result<String, StatusCode> {
    let requested = raw.unwrap_or("profile email");
    let mut result = Vec::new();
    for scope in requested.split_whitespace() {
        if !ALLOWED_SCOPES.contains(&scope) {
            return Err(StatusCode::BAD_REQUEST);
        }
        if !result.contains(&scope) {
            result.push(scope);
        }
    }
    if !result.contains(&"profile") {
        result.insert(0, "profile");
    }
    Ok(result.join(" "))
}

fn valid_pkce_challenge(value: &str) -> bool {
    (43..=128).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~'))
}

fn valid_pkce_verifier(value: &str) -> bool {
    valid_pkce_challenge(value)
}

fn verify_pkce(verifier: &str, challenge: &str) -> bool {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest) == challenge
}

fn random_token() -> String {
    let mut raw = [0u8; 32];
    rand::rng().fill_bytes(&mut raw);
    URL_SAFE_NO_PAD.encode(raw)
}

fn oauth_error(status: StatusCode, error: &str) -> Response {
    (status, Json(json!({"error": error}))).into_response()
}

fn token_error(status: StatusCode, error: &str) -> Response {
    (status, Json(json!({"error": error}))).into_response()
}

fn bearer_error(status: StatusCode, error: &str) -> Response {
    let mut response = (status, Json(json!({"error": error}))).into_response();
    if let Ok(value) = format!("Bearer error=\"{error}\"").parse() {
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, value);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_s256_round_trip() {
        let verifier = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        assert!(valid_pkce_verifier(verifier));
        assert!(valid_pkce_challenge(&challenge));
        assert!(verify_pkce(verifier, &challenge));
        assert!(!verify_pkce(
            "different-verifier-value-that-is-long-enough-1234567890",
            &challenge
        ));
    }

    #[test]
    fn scopes_are_normalized_and_restricted() {
        assert_eq!(
            normalize_scope(Some("email roles email")).unwrap(),
            "profile email roles"
        );
        assert!(normalize_scope(Some("profile unknown")).is_err());
    }
}
