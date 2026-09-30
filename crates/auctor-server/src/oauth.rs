use crate::{AppState, auth::current_user, branding, model::SessionUser};
use anyhow::Context;
use axum::{
    Form, Json, Router,
    extract::{OriginalUri, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use rand::RngCore;
use rsa::{
    RsaPrivateKey, RsaPublicKey,
    pkcs8::{DecodePrivateKey, EncodePrivateKey, LineEnding},
    rand_core::OsRng,
    traits::PublicKeyParts,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
use url::Url;
use uuid::Uuid;

const CODE_TTL_MINUTES: i64 = 5;
const TOKEN_TTL_SECONDS: i64 = 3600;
const REFRESH_TTL_DAYS: i64 = 30;
const ALLOWED_SCOPES: &[&str] = &[
    "openid",
    "profile",
    "email",
    "groups",
    "roles",
    "offline_access",
];

#[derive(Clone)]
pub struct OidcSigner {
    encoding_key: EncodingKey,
    kid: String,
    modulus: String,
    exponent: String,
}

impl OidcSigner {
    pub fn load_or_create() -> anyhow::Result<Self> {
        let path = PathBuf::from(
            env::var("AUCTOR_OIDC_SIGNING_KEY_FILE")
                .unwrap_or_else(|_| "/var/lib/auctor/oidc-signing-key.pem".to_owned()),
        );
        let private = load_or_create_rsa_key(&path)?;
        let pem = private
            .to_pkcs8_pem(LineEnding::LF)
            .context("encode OIDC RSA private key")?;
        let public = RsaPublicKey::from(&private);
        let modulus_bytes = public.n().to_bytes_be();
        let exponent_bytes = public.e().to_bytes_be();
        let mut hasher = Sha256::new();
        hasher.update(&modulus_bytes);
        hasher.update(&exponent_bytes);
        let kid = URL_SAFE_NO_PAD.encode(&hasher.finalize()[..16]);

        Ok(Self {
            encoding_key: EncodingKey::from_rsa_pem(pem.as_bytes())
                .context("load OIDC RSA signing key")?,
            kid,
            modulus: URL_SAFE_NO_PAD.encode(modulus_bytes),
            exponent: URL_SAFE_NO_PAD.encode(exponent_bytes),
        })
    }

    fn sign<T: Serialize>(&self, claims: &T, token_type: &str) -> anyhow::Result<String> {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(self.kid.clone());
        header.typ = Some(token_type.to_owned());
        encode(&header, claims, &self.encoding_key).context("sign OIDC JWT")
    }

    fn jwks(&self) -> Value {
        json!({
            "keys": [{
                "kty": "RSA",
                "use": "sig",
                "alg": "RS256",
                "kid": self.kid,
                "n": self.modulus,
                "e": self.exponent
            }]
        })
    }
}

fn load_or_create_rsa_key(path: &Path) -> anyhow::Result<RsaPrivateKey> {
    if let Ok(pem) = fs::read_to_string(path) {
        return RsaPrivateKey::from_pkcs8_pem(&pem).context("parse OIDC RSA signing key");
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("create OIDC key directory")?;
    }

    let private = RsaPrivateKey::new(&mut OsRng, 2048).context("generate OIDC RSA signing key")?;
    let pem = private
        .to_pkcs8_pem(LineEnding::LF)
        .context("encode generated OIDC RSA private key")?;
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .context("create OIDC RSA signing key file")?;
    file.write_all(pem.as_bytes())
        .context("write OIDC RSA signing key")?;
    Ok(private)
}

#[derive(Deserialize)]
struct AuthorizeQuery {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    state: Option<String>,
    scope: Option<String>,
    code_challenge: String,
    code_challenge_method: String,
    nonce: Option<String>,
}

#[derive(Deserialize)]
struct TokenRequest {
    grant_type: String,
    client_id: String,
    code: Option<String>,
    redirect_uri: Option<String>,
    code_verifier: Option<String>,
    refresh_token: Option<String>,
}

#[derive(sqlx::FromRow)]
struct Application {
    id: Uuid,
    redirect_uris: Value,
    allowed_roles: Value,
    allowed_groups: Value,
    status: String,
}

#[derive(sqlx::FromRow)]
struct CodeRow {
    id: Uuid,
    application_id: Uuid,
    user_id: Uuid,
    redirect_uri: String,
    scope: String,
    code_challenge: String,
    nonce: Option<String>,
    expires_at: DateTime<Utc>,
    used_at: Option<DateTime<Utc>>,
    client_id: String,
    application_status: String,
}

#[derive(Clone)]
struct UserClaims {
    id: Uuid,
    username: String,
    email: String,
    display_name: String,
    roles: Vec<String>,
    groups: Vec<String>,
}

#[derive(Serialize)]
struct AccessTokenClaims<'a> {
    iss: &'a str,
    sub: String,
    aud: &'a str,
    exp: i64,
    iat: i64,
    jti: String,
    scope: &'a str,
    client_id: &'a str,
    preferred_username: &'a str,
    name: &'a str,
    email: &'a str,
    roles: &'a [String],
    groups: &'a [String],
}

#[derive(Serialize)]
struct IdTokenClaims<'a> {
    iss: &'a str,
    sub: String,
    aud: &'a str,
    exp: i64,
    iat: i64,
    auth_time: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    nonce: Option<&'a str>,
    preferred_username: &'a str,
    name: &'a str,
    email: &'a str,
    roles: &'a [String],
    groups: &'a [String],
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/oauth/authorize", get(authorize))
        .route("/oauth/token", post(token))
        .route("/oauth/userinfo", get(userinfo))
        .route("/.well-known/oauth-authorization-server", get(metadata))
        .route("/.well-known/jwks.json", get(jwks))
}

pub async fn openid_metadata(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
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
        "jwks_uri": format!("{}/.well-known/jwks.json", brand.site_url),
        "response_types_supported": ["code"],
        "response_modes_supported": ["query"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["RS256"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
        "scopes_supported": ALLOWED_SCOPES,
        "claims_supported": [
            "sub", "iss", "aud", "exp", "iat", "auth_time", "nonce",
            "preferred_username", "name", "email", "roles", "groups"
        ]
    })))
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
        "jwks_uri": format!("{}/.well-known/jwks.json", brand.site_url),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
        "scopes_supported": ALLOWED_SCOPES,
    })))
}

async fn jwks(State(state): State<AppState>) -> Json<Value> {
    Json(state.oidc_signer.jwks())
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
    if !valid_pkce_challenge(&query.code_challenge)
        || query
            .nonce
            .as_deref()
            .is_some_and(|nonce| nonce.len() > 1024)
    {
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

    if !application_allows_user(&application, &user) {
        return oauth_error(StatusCode::FORBIDDEN, "access_denied");
    }

    let raw_code = random_token();
    let code_hash = Sha256::digest(raw_code.as_bytes()).to_vec();
    let expires_at = Utc::now() + Duration::minutes(CODE_TTL_MINUTES);

    if sqlx::query(
        "INSERT INTO oauth_authorization_codes
         (code_hash,application_id,user_id,redirect_uri,scope,code_challenge,code_challenge_method,nonce,expires_at)
         VALUES($1,$2,$3,$4,$5,$6,'S256',$7,$8)",
    )
    .bind(code_hash)
    .bind(application.id)
    .bind(user.id)
    .bind(&query.redirect_uri)
    .bind(&scope)
    .bind(&query.code_challenge)
    .bind(query.nonce.as_deref())
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
    match payload.grant_type.as_str() {
        "authorization_code" => authorization_code_token(&state, payload).await,
        "refresh_token" => refresh_token(&state, payload).await,
        _ => token_error(StatusCode::BAD_REQUEST, "unsupported_grant_type"),
    }
}

async fn authorization_code_token(state: &AppState, payload: TokenRequest) -> Response {
    let code = match payload.code.as_deref() {
        Some(value) if !value.is_empty() => value,
        _ => return token_error(StatusCode::BAD_REQUEST, "invalid_request"),
    };
    let redirect_uri = match payload.redirect_uri.as_deref() {
        Some(value) if !value.is_empty() => value,
        _ => return token_error(StatusCode::BAD_REQUEST, "invalid_request"),
    };
    let verifier = match payload.code_verifier.as_deref() {
        Some(value) if valid_pkce_verifier(value) => value,
        _ => return token_error(StatusCode::BAD_REQUEST, "invalid_request"),
    };

    let code_hash = Sha256::digest(code.as_bytes()).to_vec();
    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let row = match sqlx::query_as::<_, CodeRow>(
        "SELECT c.id,c.application_id,c.user_id,c.redirect_uri,c.scope,c.code_challenge,
                c.nonce,c.expires_at,c.used_at,a.client_id,a.status AS application_status
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

    if row.used_at.is_some()
        || row.expires_at <= Utc::now()
        || row.client_id != payload.client_id
        || row.application_status != "active"
        || row.redirect_uri != redirect_uri
        || !verify_pkce(verifier, &row.code_challenge)
    {
        return token_error(StatusCode::BAD_REQUEST, "invalid_grant");
    }

    let identity = match load_user_claims(state, row.user_id).await {
        Ok(identity) => identity,
        Err(status) => return status.into_response(),
    };
    let brand = match branding::load(&state.db).await {
        Ok(brand) if !brand.site_url.is_empty() => brand,
        _ => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };

    let now = Utc::now();
    let expires_at = now + Duration::seconds(TOKEN_TTL_SECONDS);
    let is_oidc = has_scope(&row.scope, "openid");
    let raw_access_token = if is_oidc {
        match sign_access_token(
            state,
            &brand.site_url,
            &row.client_id,
            &row.scope,
            &identity,
            now,
            expires_at,
        ) {
            Ok(token) => token,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    } else {
        random_token()
    };
    let access_hash = Sha256::digest(raw_access_token.as_bytes()).to_vec();

    let refresh = if has_scope(&row.scope, "offline_access") {
        Some(random_token())
    } else {
        None
    };

    let id_token = if is_oidc {
        match sign_id_token(
            state,
            &brand.site_url,
            &row.client_id,
            &identity,
            row.nonce.as_deref(),
            now,
            expires_at,
        ) {
            Ok(token) => Some(token),
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    } else {
        None
    };

    if sqlx::query("UPDATE oauth_authorization_codes SET used_at=now() WHERE id=$1")
        .bind(row.id)
        .execute(&mut *tx)
        .await
        .is_err()
        || sqlx::query(
            "INSERT INTO oauth_access_tokens
             (token_hash,application_id,user_id,scope,expires_at)
             VALUES($1,$2,$3,$4,$5)",
        )
        .bind(access_hash)
        .bind(row.application_id)
        .bind(row.user_id)
        .bind(&row.scope)
        .bind(expires_at)
        .execute(&mut *tx)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    if let Some(refresh_token) = &refresh {
        let refresh_hash = Sha256::digest(refresh_token.as_bytes()).to_vec();
        let refresh_expiry = now + Duration::days(REFRESH_TTL_DAYS);
        if sqlx::query(
            "INSERT INTO oauth_refresh_tokens
             (token_hash,application_id,user_id,scope,expires_at)
             VALUES($1,$2,$3,$4,$5)",
        )
        .bind(refresh_hash)
        .bind(row.application_id)
        .bind(row.user_id)
        .bind(&row.scope)
        .bind(refresh_expiry)
        .execute(&mut *tx)
        .await
        .is_err()
        {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    if tx.commit().await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let mut body = Map::new();
    body.insert("access_token".into(), json!(raw_access_token));
    body.insert("token_type".into(), json!("Bearer"));
    body.insert("expires_in".into(), json!(TOKEN_TTL_SECONDS));
    body.insert("scope".into(), json!(row.scope));
    if let Some(token) = refresh {
        body.insert("refresh_token".into(), json!(token));
    }
    if let Some(token) = id_token {
        body.insert("id_token".into(), json!(token));
    }
    Json(Value::Object(body)).into_response()
}

async fn refresh_token(state: &AppState, payload: TokenRequest) -> Response {
    let raw_refresh = match payload.refresh_token.as_deref() {
        Some(value) if !value.is_empty() => value,
        _ => return token_error(StatusCode::BAD_REQUEST, "invalid_request"),
    };
    let refresh_hash = Sha256::digest(raw_refresh.as_bytes()).to_vec();

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
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            String,
            String,
            String,
        ),
    >(
        "SELECT t.id,t.application_id,t.user_id,t.scope,t.expires_at,t.revoked_at,
                a.client_id,a.status,u.status
         FROM oauth_refresh_tokens t
         JOIN applications a ON a.id=t.application_id
         JOIN users u ON u.id=t.user_id
         WHERE t.token_hash=$1
         FOR UPDATE",
    )
    .bind(refresh_hash)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => return token_error(StatusCode::BAD_REQUEST, "invalid_grant"),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    if row.5.is_some()
        || row.4 <= Utc::now()
        || row.6 != payload.client_id
        || row.7 != "active"
        || row.8 != "active"
    {
        return token_error(StatusCode::BAD_REQUEST, "invalid_grant");
    }

    let identity = match load_user_claims(state, row.2).await {
        Ok(identity) => identity,
        Err(status) => return status.into_response(),
    };
    let brand = match branding::load(&state.db).await {
        Ok(brand) if !brand.site_url.is_empty() => brand,
        _ => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };

    let now = Utc::now();
    let access_expiry = now + Duration::seconds(TOKEN_TTL_SECONDS);
    let access_token = match sign_access_token(
        state,
        &brand.site_url,
        &row.6,
        &row.3,
        &identity,
        now,
        access_expiry,
    ) {
        Ok(token) => token,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let new_refresh = random_token();

    let access_hash = Sha256::digest(access_token.as_bytes()).to_vec();
    let new_refresh_hash = Sha256::digest(new_refresh.as_bytes()).to_vec();
    let refresh_expiry = now + Duration::days(REFRESH_TTL_DAYS);

    if sqlx::query("UPDATE oauth_refresh_tokens SET revoked_at=now() WHERE id=$1")
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
        .bind(&row.3)
        .bind(access_expiry)
        .execute(&mut *tx)
        .await
        .is_err()
        || sqlx::query(
            "INSERT INTO oauth_refresh_tokens
             (token_hash,application_id,user_id,scope,expires_at)
             VALUES($1,$2,$3,$4,$5)",
        )
        .bind(new_refresh_hash)
        .bind(row.1)
        .bind(row.2)
        .bind(&row.3)
        .bind(refresh_expiry)
        .execute(&mut *tx)
        .await
        .is_err()
        || tx.commit().await.is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    Json(json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": TOKEN_TTL_SECONDS,
        "refresh_token": new_refresh,
        "scope": row.3,
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
        "SELECT id,redirect_uris,allowed_roles,allowed_groups,status
         FROM applications WHERE client_id=$1",
    )
    .bind(client_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::BAD_REQUEST)
}

async fn load_user_claims(state: &AppState, user_id: Uuid) -> Result<UserClaims, StatusCode> {
    let row = sqlx::query_as::<_, (Uuid, String, String, String)>(
        "SELECT id,username,email,display_name FROM users WHERE id=$1 AND status='active'",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::UNAUTHORIZED)?;

    let roles: Vec<String> = sqlx::query_scalar(
        "SELECT r.name FROM roles r JOIN user_roles ur ON ur.role_id=r.id
         WHERE ur.user_id=$1 ORDER BY r.name",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let groups: Vec<String> = sqlx::query_scalar(
        "SELECT g.name FROM groups g JOIN user_groups ug ON ug.group_id=g.id
         WHERE ug.user_id=$1 ORDER BY lower(g.name)",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(UserClaims {
        id: row.0,
        username: row.1,
        email: row.2,
        display_name: row.3,
        roles,
        groups,
    })
}

fn application_allows_user(application: &Application, user: &SessionUser) -> bool {
    let allowed_roles = json_string_array(&application.allowed_roles);
    let allowed_groups = json_string_array(&application.allowed_groups);
    if allowed_roles.is_empty() && allowed_groups.is_empty() {
        return true;
    }

    allowed_roles
        .iter()
        .any(|allowed| user.roles.iter().any(|actual| actual == allowed))
        || allowed_groups
            .iter()
            .any(|allowed| user.groups.iter().any(|actual| actual == allowed))
}

fn json_string_array(value: &Value) -> Vec<String> {
    serde_json::from_value(value.clone()).unwrap_or_default()
}

fn sign_access_token(
    state: &AppState,
    issuer: &str,
    client_id: &str,
    scope: &str,
    identity: &UserClaims,
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<String> {
    let claims = AccessTokenClaims {
        iss: issuer,
        sub: identity.id.to_string(),
        aud: client_id,
        exp: expires_at.timestamp(),
        iat: issued_at.timestamp(),
        jti: Uuid::new_v4().to_string(),
        scope,
        client_id,
        preferred_username: &identity.username,
        name: &identity.display_name,
        email: &identity.email,
        roles: &identity.roles,
        groups: &identity.groups,
    };
    state.oidc_signer.sign(&claims, "at+jwt")
}

fn sign_id_token(
    state: &AppState,
    issuer: &str,
    client_id: &str,
    identity: &UserClaims,
    nonce: Option<&str>,
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<String> {
    let claims = IdTokenClaims {
        iss: issuer,
        sub: identity.id.to_string(),
        aud: client_id,
        exp: expires_at.timestamp(),
        iat: issued_at.timestamp(),
        auth_time: issued_at.timestamp(),
        nonce,
        preferred_username: &identity.username,
        name: &identity.display_name,
        email: &identity.email,
        roles: &identity.roles,
        groups: &identity.groups,
    };
    state.oidc_signer.sign(&claims, "JWT")
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

    if result.contains(&"offline_access") && !result.contains(&"openid") {
        return Err(StatusCode::BAD_REQUEST);
    }
    if !result.contains(&"profile") {
        result.insert(0, "profile");
    }
    if result.contains(&"openid") {
        result.retain(|scope| *scope != "openid");
        result.insert(0, "openid");
    }
    Ok(result.join(" "))
}

fn has_scope(scope: &str, expected: &str) -> bool {
    scope.split_whitespace().any(|value| value == expected)
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
        assert_eq!(
            normalize_scope(Some("openid profile groups offline_access")).unwrap(),
            "openid profile groups offline_access"
        );
        assert!(normalize_scope(Some("profile unknown")).is_err());
        assert!(normalize_scope(Some("offline_access profile")).is_err());
    }

    #[test]
    fn application_policy_defaults_open_and_supports_roles_and_groups() {
        let mut app = Application {
            id: Uuid::new_v4(),
            redirect_uris: json!([]),
            allowed_roles: json!([]),
            allowed_groups: json!([]),
            status: "active".into(),
        };
        let user = SessionUser {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            username: "alice".into(),
            email: "alice@example.com".into(),
            display_name: "Alice".into(),
            roles: vec!["user".into()],
            groups: vec!["research".into()],
        };
        assert!(application_allows_user(&app, &user));

        app.allowed_roles = json!(["platform-admin"]);
        assert!(!application_allows_user(&app, &user));

        app.allowed_groups = json!(["research"]);
        assert!(application_allows_user(&app, &user));
    }
}
