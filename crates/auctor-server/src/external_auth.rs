use crate::{
    AppState,
    auth::{current_user, issue_session_cookie},
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::{delete, get},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const PROVIDER_GITHUB: &str = "github";

#[derive(Debug)]
struct GithubConfig {
    site_url: String,
    enabled: bool,
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Deserialize)]
struct StartQuery {
    intent: Option<String>,
    bind: Option<bool>,
    r#continue: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct Flow {
    intent: String,
    user_id: Option<Uuid>,
    return_to: String,
}

#[derive(Debug, Deserialize)]
struct GithubTokenResponse {
    access_token: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubUser {
    id: u64,
    login: String,
    name: Option<String>,
    avatar_url: Option<String>,
    html_url: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct ExternalIdentity {
    provider: String,
    subject: String,
    login: String,
    display_name: Option<String>,
    avatar_url: Option<String>,
    profile_url: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/external/providers", get(list_providers))
        .route("/auth/external/github/start", get(start_github))
        .route("/auth/external/github/callback", get(github_callback))
        .route("/account/external-identities", get(list_identities))
        .route(
            "/account/external-identities/{provider}",
            delete(unbind_identity),
        )
}

async fn github_config(db: &sqlx::PgPool) -> Result<GithubConfig, sqlx::Error> {
    sqlx::query_as::<_, (String, bool, String, String)>(
        "SELECT site_url,github_oauth_enabled,github_oauth_client_id,github_oauth_client_secret
         FROM site_settings WHERE singleton=true",
    )
    .fetch_one(db)
    .await
    .map(
        |(site_url, enabled, client_id, client_secret)| GithubConfig {
            site_url,
            enabled,
            client_id,
            client_secret,
        },
    )
}

fn github_ready(config: &GithubConfig) -> bool {
    config.enabled
        && !config.site_url.trim().is_empty()
        && !config.client_id.trim().is_empty()
        && !config.client_secret.trim().is_empty()
}

async fn list_providers(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let config = github_config(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let items = if github_ready(&config) {
        vec![json!({"id": PROVIDER_GITHUB, "name": "GitHub"})]
    } else {
        Vec::new()
    };
    Ok(Json(json!({"items": items})))
}

async fn start_github(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<StartQuery>,
) -> Result<Response, StatusCode> {
    let config = github_config(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !github_ready(&config) {
        return Err(StatusCode::NOT_FOUND);
    }

    let bind_requested = query.bind.unwrap_or(false) || query.intent.as_deref() == Some("bind");
    let intent = if bind_requested { "bind" } else { "login" };
    if query
        .intent
        .as_deref()
        .is_some_and(|value| value != "login" && value != "bind")
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    let (user_id, return_to) = if bind_requested {
        let user = current_user(&state, &headers).await?;
        (
            Some(user.id),
            safe_continue_path(query.r#continue.as_deref(), "/account"),
        )
    } else {
        (None, safe_continue_path(query.r#continue.as_deref(), "/"))
    };

    let mut raw = [0u8; 32];
    rand::rng().fill_bytes(&mut raw);
    let oauth_state = URL_SAFE_NO_PAD.encode(raw);
    let state_hash = Sha256::digest(oauth_state.as_bytes()).to_vec();

    let _ = sqlx::query(
        "DELETE FROM external_auth_flows
         WHERE expires_at < now() OR (used_at IS NOT NULL AND used_at < now() - interval '1 day')",
    )
    .execute(&state.db)
    .await;

    sqlx::query(
        "INSERT INTO external_auth_flows(state_hash,provider,intent,user_id,return_to,expires_at)
         VALUES($1,$2,$3,$4,$5,$6)",
    )
    .bind(state_hash)
    .bind(PROVIDER_GITHUB)
    .bind(intent)
    .bind(user_id)
    .bind(&return_to)
    .bind(Utc::now() + Duration::minutes(10))
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let callback = github_callback_url(&config.site_url);
    let mut authorize =
        url::Url::parse("https://github.com/login/oauth/authorize").expect("static GitHub URL");
    authorize
        .query_pairs_mut()
        .append_pair("client_id", config.client_id.trim())
        .append_pair("redirect_uri", &callback)
        .append_pair("scope", "read:user")
        .append_pair("state", &oauth_state)
        .append_pair("allow_signup", "false");

    Ok(Redirect::temporary(authorize.as_str()).into_response())
}

async fn github_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CallbackQuery>,
) -> Response {
    let Some(raw_state) = query.state.as_deref() else {
        return redirect_login_error("invalid_state");
    };
    let state_hash = Sha256::digest(raw_state.as_bytes()).to_vec();

    let flow = match sqlx::query_as::<_, Flow>(
        "UPDATE external_auth_flows
         SET used_at=now()
         WHERE state_hash=$1
           AND provider=$2
           AND used_at IS NULL
           AND expires_at>now()
         RETURNING intent,user_id,return_to",
    )
    .bind(state_hash)
    .bind(PROVIDER_GITHUB)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(flow)) => flow,
        Ok(None) => return redirect_login_error("invalid_state"),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    if flow.intent == "bind" {
        let Some(expected_user_id) = flow.user_id else {
            return redirect_account_error("invalid_flow");
        };
        match current_user(&state, &headers).await {
            Ok(user) if user.id == expected_user_id => {}
            _ => return redirect_account_error("reauth_required"),
        }
    }

    if query.error.is_some() {
        return callback_error(&flow, "provider_denied");
    }
    let Some(code) = query.code.as_deref() else {
        return callback_error(&flow, "provider_failed");
    };

    let config = match github_config(&state.db).await {
        Ok(config) if github_ready(&config) => config,
        _ => return callback_error(&flow, "provider_unavailable"),
    };
    let github_user = match fetch_github_user(&state, &config, code).await {
        Ok(user) => user,
        Err(error) => {
            tracing::warn!(?error, "GitHub OAuth callback failed");
            return callback_error(&flow, "provider_failed");
        }
    };

    if flow.intent == "bind" {
        let Some(user_id) = flow.user_id else {
            return redirect_account_error("invalid_flow");
        };
        return bind_github_identity(&state, user_id, &github_user).await;
    }

    login_with_github(&state, &headers, &flow.return_to, &github_user).await
}

async fn fetch_github_user(
    state: &AppState,
    config: &GithubConfig,
    code: &str,
) -> anyhow::Result<GithubUser> {
    let callback = github_callback_url(&config.site_url);
    let body = {
        let mut form = url::form_urlencoded::Serializer::new(String::new());
        form.append_pair("client_id", config.client_id.trim());
        form.append_pair("client_secret", config.client_secret.trim());
        form.append_pair("code", code);
        form.append_pair("redirect_uri", &callback);
        form.finish()
    };

    let token_response = state
        .http
        .post("https://github.com/login/oauth/access_token")
        .header(header::ACCEPT, "application/json")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await?;
    if !token_response.status().is_success() {
        anyhow::bail!("GitHub token endpoint returned {}", token_response.status());
    }
    let token: GithubTokenResponse = serde_json::from_str(&token_response.text().await?)?;
    let access_token = token
        .access_token
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("GitHub token exchange failed: {:?}", token.error))?;

    let user_response = state
        .http
        .get("https://api.github.com/user")
        .header(header::ACCEPT, "application/vnd.github+json")
        .header("x-github-api-version", "2022-11-28")
        .bearer_auth(access_token)
        .send()
        .await?;
    if !user_response.status().is_success() {
        anyhow::bail!("GitHub user endpoint returned {}", user_response.status());
    }
    Ok(serde_json::from_str(&user_response.text().await?)?)
}

async fn bind_github_identity(
    state: &AppState,
    user_id: Uuid,
    github_user: &GithubUser,
) -> Response {
    let subject = github_user.id.to_string();

    let owner: Option<Uuid> = match sqlx::query_scalar(
        "SELECT user_id FROM external_identities WHERE provider=$1 AND subject=$2",
    )
    .bind(PROVIDER_GITHUB)
    .bind(&subject)
    .fetch_optional(&state.db)
    .await
    {
        Ok(owner) => owner,
        Err(_) => return redirect_account_error("storage_failed"),
    };
    if owner.is_some_and(|owner| owner != user_id) {
        return redirect_account_error("identity_in_use");
    }

    let existing_subject: Option<String> = match sqlx::query_scalar(
        "SELECT subject FROM external_identities WHERE user_id=$1 AND provider=$2",
    )
    .bind(user_id)
    .bind(PROVIDER_GITHUB)
    .fetch_optional(&state.db)
    .await
    {
        Ok(subject) => subject,
        Err(_) => return redirect_account_error("storage_failed"),
    };
    if existing_subject
        .as_deref()
        .is_some_and(|existing| existing != subject)
    {
        return redirect_account_error("provider_already_bound");
    }

    let result = sqlx::query(
        "INSERT INTO external_identities(
             user_id,provider,subject,login,display_name,avatar_url,profile_url
         )
         VALUES($1,$2,$3,$4,$5,$6,$7)
         ON CONFLICT(user_id,provider) DO UPDATE SET
             login=EXCLUDED.login,
             display_name=EXCLUDED.display_name,
             avatar_url=EXCLUDED.avatar_url,
             profile_url=EXCLUDED.profile_url,
             updated_at=now()",
    )
    .bind(user_id)
    .bind(PROVIDER_GITHUB)
    .bind(&subject)
    .bind(&github_user.login)
    .bind(&github_user.name)
    .bind(&github_user.avatar_url)
    .bind(&github_user.html_url)
    .execute(&state.db)
    .await;
    if result.is_err() {
        return redirect_account_error("storage_failed");
    }

    if let Err(error) = audit(
        state,
        user_id,
        "external_identity.bind",
        json!({
            "provider": PROVIDER_GITHUB,
            "subject": subject,
            "login": github_user.login,
        }),
    )
    .await
    {
        tracing::warn!(?error, user_id=%user_id, "could not write external identity bind audit event");
    }

    Redirect::temporary("/account?external_result=bound&provider=github").into_response()
}

async fn login_with_github(
    state: &AppState,
    headers: &HeaderMap,
    continue_path: &str,
    github_user: &GithubUser,
) -> Response {
    let subject = github_user.id.to_string();
    let row: Option<(Uuid, String)> = match sqlx::query_as(
        "SELECT u.id,u.status
         FROM external_identities e
         JOIN users u ON u.id=e.user_id
         WHERE e.provider=$1 AND e.subject=$2",
    )
    .bind(PROVIDER_GITHUB)
    .bind(&subject)
    .fetch_optional(&state.db)
    .await
    {
        Ok(row) => row,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let Some((user_id, status)) = row else {
        return redirect_login_error("unbound_identity");
    };
    if status != "active" {
        return redirect_login_error("account_unavailable");
    }

    let _ = sqlx::query(
        "UPDATE external_identities
         SET login=$3,display_name=$4,avatar_url=$5,profile_url=$6,updated_at=now()
         WHERE user_id=$1 AND provider=$2",
    )
    .bind(user_id)
    .bind(PROVIDER_GITHUB)
    .bind(&github_user.login)
    .bind(&github_user.name)
    .bind(&github_user.avatar_url)
    .bind(&github_user.html_url)
    .execute(&state.db)
    .await;

    let cookie = match issue_session_cookie(state, headers, user_id).await {
        Ok(cookie) => cookie,
        Err(_) => return redirect_login_error("session_failed"),
    };

    if let Err(error) = audit(
        state,
        user_id,
        "external_auth.login",
        json!({
            "provider": PROVIDER_GITHUB,
            "subject": subject,
            "login": github_user.login,
        }),
    )
    .await
    {
        tracing::warn!(?error, user_id=%user_id, "could not write external login audit event");
    }

    let target = safe_continue_path(Some(continue_path), "/");
    let mut response = Redirect::temporary(&target).into_response();
    response.headers_mut().insert(header::SET_COOKIE, cookie);
    response
}

async fn list_identities(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = current_user(&state, &headers).await?;
    let items: Vec<ExternalIdentity> = sqlx::query_as(
        "SELECT provider,subject,login,display_name,avatar_url,profile_url,created_at,updated_at
         FROM external_identities
         WHERE user_id=$1
         ORDER BY provider",
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({"items": items})))
}

async fn unbind_identity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(provider): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = current_user(&state, &headers).await?;
    let provider = provider.trim().to_ascii_lowercase();
    if provider.is_empty() || provider.len() > 64 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let result = sqlx::query("DELETE FROM external_identities WHERE user_id=$1 AND provider=$2")
        .bind(user.id)
        .bind(&provider)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    audit(
        &state,
        user.id,
        "external_identity.unbind",
        json!({"provider": provider}),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({"ok": true})))
}

fn github_callback_url(site_url: &str) -> String {
    format!(
        "{}/api/auth/external/github/callback",
        site_url.trim_end_matches('/')
    )
}

fn safe_continue_path(raw: Option<&str>, fallback: &str) -> String {
    raw.map(str::trim)
        .filter(|value| {
            value.starts_with('/')
                && !value.starts_with("//")
                && !value
                    .chars()
                    .any(|ch| matches!(ch, '\r' | '\n' | '\\' | '\0'))
                && value.len() <= 2048
        })
        .unwrap_or(fallback)
        .to_owned()
}

fn callback_error(flow: &Flow, code: &str) -> Response {
    if flow.intent == "bind" {
        redirect_account_error(code)
    } else {
        redirect_login_error(code)
    }
}

fn redirect_login_error(code: &str) -> Response {
    let target = format!(
        "/login?external_error={}",
        url::form_urlencoded::byte_serialize(code.as_bytes()).collect::<String>()
    );
    Redirect::temporary(&target).into_response()
}

fn redirect_account_error(code: &str) -> Response {
    let target = format!(
        "/account?external_error={}",
        url::form_urlencoded::byte_serialize(code.as_bytes()).collect::<String>()
    );
    Redirect::temporary(&target).into_response()
}

async fn audit(
    state: &AppState,
    actor: Uuid,
    action: &str,
    metadata: serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit_events(actor_user_id,action,target_type,target_id,metadata)
         VALUES($1,$2,'user',$3,$4)",
    )
    .bind(actor)
    .bind(action)
    .bind(actor.to_string())
    .bind(metadata)
    .execute(&state.db)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continue_path_stays_same_origin() {
        assert_eq!(safe_continue_path(Some("/account"), "/"), "/account");
        assert_eq!(
            safe_continue_path(Some("/oauth/authorize?client_id=test"), "/"),
            "/oauth/authorize?client_id=test"
        );
        assert_eq!(safe_continue_path(Some("//evil.example"), "/"), "/");
        assert_eq!(safe_continue_path(Some("https://evil.example"), "/"), "/");
        assert_eq!(safe_continue_path(Some("/\\evil"), "/"), "/");
    }

    #[test]
    fn github_provider_requires_explicit_enable_and_complete_config() {
        let ready = GithubConfig {
            site_url: "https://account.example.com".to_owned(),
            enabled: true,
            client_id: "client".to_owned(),
            client_secret: "secret".to_owned(),
        };
        assert!(github_ready(&ready));

        let mut disabled = GithubConfig {
            enabled: false,
            ..ready
        };
        assert!(!github_ready(&disabled));
        disabled.enabled = true;
        disabled.client_secret.clear();
        assert!(!github_ready(&disabled));
    }
}
