use crate::{
    AppState,
    auth::require_admin,
    db::hash_password,
    model::{CreateUser, Pagination, PasswordChange, SetRoles, UpdateUser, UserRecord},
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post, put},
};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct AuditRow {
    id: i64,
    actor_user_id: Option<Uuid>,
    actor_username: Option<String>,
    action: String,
    target_type: String,
    target_id: Option<String>,
    metadata: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/stats", get(stats))
        .route("/admin/users", get(list_users).post(create_user))
        .route("/admin/users/{id}", get(get_user).patch(update_user))
        .route("/admin/users/{id}/roles", put(set_roles))
        .route("/admin/users/{id}/password", put(set_password))
        .route("/admin/users/{id}/revoke-sessions", post(revoke_sessions))
        .route("/admin/audit", get(list_audit))
}

#[derive(Serialize)]
struct Stats {
    users_total: i64,
    users_active: i64,
    sessions_active: i64,
    applications_total: i64,
}

async fn stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Stats>, StatusCode> {
    require_admin(&state, &headers).await?;
    let (users_total, users_active, sessions_active, applications_total) = tokio::try_join!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM users").fetch_one(&state.db),
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM users WHERE status='active'")
            .fetch_one(&state.db),
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM sessions WHERE revoked_at IS NULL AND expires_at>now()"
        )
        .fetch_one(&state.db),
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM applications").fetch_one(&state.db),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(Stats {
        users_total,
        users_active,
        sessions_active,
        applications_total,
    }))
}

#[derive(Serialize)]
struct UserListItem {
    #[serde(flatten)]
    user: UserRecord,
    roles: Vec<String>,
    groups: Vec<String>,
    active_sessions: i64,
    last_seen_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize)]
struct UserList {
    items: Vec<UserListItem>,
    total: i64,
}

async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(p): Query<Pagination>,
) -> Result<Json<UserList>, StatusCode> {
    require_admin(&state, &headers).await?;
    let limit = p.limit.unwrap_or(50).clamp(1, 200);
    let offset = p.offset.unwrap_or(0).max(0);
    let q = p.q.unwrap_or_default();
    let status = p.status.unwrap_or_default();

    let users = sqlx::query_as::<_, UserRecord>(
        "SELECT id,username,email,display_name,status,created_at,updated_at
         FROM users
         WHERE ($1='' OR username ILIKE '%'||$1||'%' OR email ILIKE '%'||$1||'%' OR display_name ILIKE '%'||$1||'%')
           AND ($2='' OR status=$2)
         ORDER BY created_at DESC LIMIT $3 OFFSET $4",
    )
    .bind(&q).bind(&status).bind(limit).bind(offset)
    .fetch_all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let total: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM users
         WHERE ($1='' OR username ILIKE '%'||$1||'%' OR email ILIKE '%'||$1||'%' OR display_name ILIKE '%'||$1||'%')
           AND ($2='' OR status=$2)",
    )
    .bind(&q).bind(&status)
    .fetch_one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut items = Vec::with_capacity(users.len());
    for user in users {
        let roles: Vec<String> = sqlx::query_scalar(
            "SELECT r.name FROM roles r JOIN user_roles ur ON ur.role_id=r.id WHERE ur.user_id=$1 ORDER BY r.name"
        ).bind(user.id).fetch_all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let groups: Vec<String> = sqlx::query_scalar(
            "SELECT g.name FROM groups g JOIN user_groups ug ON ug.group_id=g.id WHERE ug.user_id=$1 ORDER BY lower(g.name)"
        ).bind(user.id).fetch_all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let active_sessions: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM sessions WHERE user_id=$1 AND revoked_at IS NULL AND expires_at>now()"
        ).bind(user.id).fetch_one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let last_seen_at =
            sqlx::query_scalar("SELECT max(last_seen_at) FROM sessions WHERE user_id=$1")
                .bind(user.id)
                .fetch_one(&state.db)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        items.push(UserListItem {
            user,
            roles,
            groups,
            active_sessions,
            last_seen_at,
        });
    }

    Ok(Json(UserList { items, total }))
}

async fn get_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<UserListItem>, StatusCode> {
    require_admin(&state, &headers).await?;
    let user = sqlx::query_as::<_, UserRecord>(
        "SELECT id,username,email,display_name,status,created_at,updated_at FROM users WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;
    let roles: Vec<String> = sqlx::query_scalar(
        "SELECT r.name FROM roles r JOIN user_roles ur ON ur.role_id=r.id WHERE ur.user_id=$1 ORDER BY r.name"
    ).bind(id).fetch_all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let groups: Vec<String> = sqlx::query_scalar(
        "SELECT g.name FROM groups g JOIN user_groups ug ON ug.group_id=g.id WHERE ug.user_id=$1 ORDER BY lower(g.name)"
    ).bind(id).fetch_all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let active_sessions: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM sessions WHERE user_id=$1 AND revoked_at IS NULL AND expires_at>now()"
    ).bind(id).fetch_one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let last_seen_at =
        sqlx::query_scalar("SELECT max(last_seen_at) FROM sessions WHERE user_id=$1")
            .bind(id)
            .fetch_one(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(UserListItem {
        user,
        roles,
        groups,
        active_sessions,
        last_seen_at,
    }))
}

async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateUser>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let actor = require_admin(&state, &headers).await?;
    validate_identity(&payload.username, &payload.email, &payload.display_name)?;
    crate::username_policy::ensure_allowed(&state.db, &payload.username).await?;
    let password_hash = hash_password(&payload.password).map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO users(username,email,display_name,password_hash) VALUES($1,$2,$3,$4) RETURNING id"
    ).bind(payload.username.trim())
     .bind(payload.email.trim().to_lowercase())
     .bind(payload.display_name.trim())
     .bind(password_hash)
     .fetch_one(&mut *tx).await.map_err(|_| StatusCode::CONFLICT)?;

    let mut roles = payload.roles;
    if !roles.iter().any(|r| r == "user") {
        roles.push("user".into());
    }
    for role in roles {
        let result = sqlx::query(
            "INSERT INTO user_roles(user_id,role_id) SELECT $1,id FROM roles WHERE name=$2 ON CONFLICT DO NOTHING"
        ).bind(id).bind(role).execute(&mut *tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if result.rows_affected() == 0 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    audit(
        &mut tx,
        actor.id,
        "user.create",
        "user",
        Some(id.to_string()),
        json!({"username":payload.username}),
    )
    .await?;
    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(json!({"id":id}))))
}

async fn update_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateUser>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = require_admin(&state, &headers).await?;
    if payload
        .status
        .as_deref()
        .is_some_and(|s| s != "active" && s != "disabled")
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    if id == actor.id && payload.status.as_deref() == Some("disabled") {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(username) = &payload.username {
        let username = username.trim();
        if username.len() < 3
            || username.len() > 64
            || !username
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            return Err(StatusCode::BAD_REQUEST);
        }

        let current_username: String = sqlx::query_scalar("SELECT username FROM users WHERE id=$1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        if !current_username.eq_ignore_ascii_case(username) {
            crate::username_policy::ensure_allowed(&state.db, username).await?;
        }
    }
    if let Some(email) = &payload.email {
        if !email.contains('@') || email.len() > 320 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    if let Some(name) = &payload.display_name {
        if name.trim().is_empty() || name.len() > 160 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let result = sqlx::query(
        "UPDATE users SET
           username=COALESCE($2,username),
           email=COALESCE(lower($3),email),
           display_name=COALESCE($4,display_name),
           status=COALESCE($5,status),
           updated_at=now()
         WHERE id=$1",
    )
    .bind(id)
    .bind(payload.username.as_deref().map(str::trim))
    .bind(payload.email.as_deref())
    .bind(payload.display_name.as_deref())
    .bind(payload.status.as_deref())
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::CONFLICT)?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    sqlx::query("INSERT INTO audit_events(actor_user_id,action,target_type,target_id,metadata) VALUES($1,'user.update','user',$2,$3)")
        .bind(actor.id).bind(id.to_string()).bind(json!({"status":payload.status}))
        .execute(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({"ok":true})))
}

async fn set_roles(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<SetRoles>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = require_admin(&state, &headers).await?;
    let removes_admin = !payload.roles.iter().any(|r| r == "platform-admin");
    if id == actor.id && removes_admin {
        return Err(StatusCode::BAD_REQUEST);
    }
    if removes_admin {
        let target_is_admin: bool = sqlx::query_scalar(
            "SELECT EXISTS(
               SELECT 1 FROM user_roles ur JOIN roles r ON r.id=ur.role_id
               WHERE ur.user_id=$1 AND r.name='platform-admin'
             )",
        )
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if target_is_admin {
            let active_admins: i64 = sqlx::query_scalar(
                "SELECT count(DISTINCT u.id)
                 FROM users u
                 JOIN user_roles ur ON ur.user_id=u.id
                 JOIN roles r ON r.id=ur.role_id
                 WHERE u.status='active' AND r.name='platform-admin'",
            )
            .fetch_one(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            if active_admins <= 1 {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    sqlx::query("DELETE FROM user_roles WHERE user_id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut roles = payload.roles;
    if !roles.iter().any(|r| r == "user") {
        roles.push("user".into());
    }
    for role in &roles {
        let result = sqlx::query(
            "INSERT INTO user_roles(user_id,role_id) SELECT $1,id FROM roles WHERE name=$2",
        )
        .bind(id)
        .bind(role)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if result.rows_affected() == 0 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    audit(
        &mut tx,
        actor.id,
        "user.roles.update",
        "user",
        Some(id.to_string()),
        json!({"roles":roles}),
    )
    .await?;
    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({"ok":true})))
}

async fn set_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<PasswordChange>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = require_admin(&state, &headers).await?;
    let password_hash = hash_password(&payload.password).map_err(|_| StatusCode::BAD_REQUEST)?;
    let result = sqlx::query("UPDATE users SET password_hash=$2,updated_at=now() WHERE id=$1")
        .bind(id)
        .bind(password_hash)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    sqlx::query("UPDATE sessions SET revoked_at=now() WHERE user_id=$1 AND revoked_at IS NULL")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    sqlx::query("INSERT INTO audit_events(actor_user_id,action,target_type,target_id) VALUES($1,'user.password.reset','user',$2)")
        .bind(actor.id).bind(id.to_string()).execute(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({"ok":true})))
}

async fn revoke_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = require_admin(&state, &headers).await?;
    let affected =
        sqlx::query("UPDATE sessions SET revoked_at=now() WHERE user_id=$1 AND revoked_at IS NULL")
            .bind(id)
            .execute(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .rows_affected();
    sqlx::query("INSERT INTO audit_events(actor_user_id,action,target_type,target_id,metadata) VALUES($1,'user.sessions.revoke','user',$2,$3)")
        .bind(actor.id).bind(id.to_string()).bind(json!({"revoked":affected}))
        .execute(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({"ok":true,"revoked":affected})))
}

async fn list_audit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(p): Query<Pagination>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    require_admin(&state, &headers).await?;
    let limit = p.limit.unwrap_or(100).clamp(1, 200);
    let offset = p.offset.unwrap_or(0).max(0);
    let rows: Vec<AuditRow> = sqlx::query_as(
        "SELECT a.id,a.actor_user_id,u.username AS actor_username,a.action,a.target_type,a.target_id,a.metadata,a.created_at
         FROM audit_events a LEFT JOIN users u ON u.id=a.actor_user_id
         ORDER BY a.created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({"items": rows.into_iter().map(|r| json!({
        "id":r.id,
        "actor_user_id":r.actor_user_id,
        "actor_username":r.actor_username,
        "action":r.action,
        "target_type":r.target_type,
        "target_id":r.target_id,
        "metadata":r.metadata,
        "created_at":r.created_at
    })).collect::<Vec<_>>()})))
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

async fn audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: Uuid,
    action: &str,
    target_type: &str,
    target_id: Option<String>,
    metadata: serde_json::Value,
) -> Result<(), StatusCode> {
    sqlx::query("INSERT INTO audit_events(actor_user_id,action,target_type,target_id,metadata) VALUES($1,$2,$3,$4,$5)")
        .bind(actor).bind(action).bind(target_type).bind(target_id).bind(metadata)
        .execute(&mut **tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}
