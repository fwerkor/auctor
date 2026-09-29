use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct UserRecord {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SessionUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub roles: Vec<String>,
}

impl SessionUser {
    pub fn is_admin(&self) -> bool {
        self.roles.iter().any(|role| role == "platform-admin")
    }
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub q: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub password: String,
    #[serde(default)]
    pub roles: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    pub username: Option<String>,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetRoles {
    pub roles: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PasswordChange {
    pub password: String,
}
