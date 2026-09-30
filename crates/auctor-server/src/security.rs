use crate::AppState;
use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderName, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

type RateBuckets = HashMap<(String, String), VecDeque<Instant>>;

#[derive(Clone, Default)]
pub struct RateLimiter {
    inner: Arc<Mutex<RateBuckets>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    fn allow(&self, bucket: &str, client: &str, limit: usize, window: Duration) -> bool {
        let now = Instant::now();
        let cutoff = now.checked_sub(window).unwrap_or(now);
        let mut inner = self.inner.lock().expect("rate limiter mutex poisoned");
        let entries = inner
            .entry((bucket.to_owned(), client.to_owned()))
            .or_default();

        while entries.front().is_some_and(|instant| *instant < cutoff) {
            entries.pop_front();
        }
        if entries.len() >= limit {
            return false;
        }
        entries.push_back(now);

        if inner.len() > 10_000 {
            inner.retain(|_, values| values.back().is_some_and(|instant| *instant >= cutoff));
        }
        true
    }
}

pub async fn middleware(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    let client = client_key(request.headers());

    let rule = match path.as_str() {
        "/api/auth/login" => Some(("login", 10, Duration::from_secs(60))),
        "/api/auth/register" => Some(("register", 5, Duration::from_secs(300))),
        "/api/auth/register/verify" => Some(("register-verify", 10, Duration::from_secs(300))),
        "/api/auth/register/resend" => Some(("register-resend", 3, Duration::from_secs(600))),
        "/api/auth/password/forgot" => Some(("password-forgot", 3, Duration::from_secs(600))),
        "/api/auth/password/reset" => Some(("password-reset", 10, Duration::from_secs(600))),
        "/api/account/email/request" => Some(("email-change-request", 3, Duration::from_secs(600))),
        "/api/account/email/confirm" => {
            Some(("email-change-confirm", 10, Duration::from_secs(300)))
        }
        "/api/setup" => Some(("setup", 10, Duration::from_secs(300))),
        "/oauth/token" => Some(("oauth-token", 30, Duration::from_secs(60))),
        "/oauth/authorize" => Some(("oauth-authorize", 120, Duration::from_secs(60))),
        _ if path.starts_with("/api/") => Some(("api", 300, Duration::from_secs(60))),
        _ => None,
    };

    if let Some((bucket, limit, window)) = rule {
        if !state.rate_limiter.allow(bucket, &client, limit, window) {
            let mut response = (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({"error":"rate_limited"})),
            )
                .into_response();
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from_static("60"));
            apply_security_headers(&mut response, &path);
            return response;
        }
    }

    let mut response = next.run(request).await;
    apply_security_headers(&mut response, &path);
    response
}

fn client_key(headers: &axum::http::HeaderMap) -> String {
    headers
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            headers
                .get("x-forwarded-for")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.split(',').next_back())
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or("unknown")
        .to_owned()
}

fn apply_security_headers(response: &mut Response, path: &str) {
    let headers = response.headers_mut();

    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'",
        ),
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        HeaderName::from_static("strict-transport-security"),
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );

    if path.starts_with("/api/") || path.starts_with("/oauth/") || path.starts_with("/.well-known/")
    {
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store, max-age=0"),
        );
        headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    }
}
