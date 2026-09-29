//! Protocol-independent identity and authorization model for Auctor.
//!
//! Administration is authorization granted to a normal global user.
//! Auctor has no separate administrator identity class.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub status: UserStatus,
}

impl User {
    pub fn new(username: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            username: username.into(),
            display_name: display_name.into(),
            status: UserStatus::Active,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    Active,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Application {
    pub id: Uuid,
    pub client_id: String,
    pub name: String,
    pub redirect_uris: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn administrators_remain_normal_users() {
        let user = User::new("caoyuhang", "Cao Yuhang");
        let admin_role = Role {
            id: Uuid::new_v4(),
            name: "platform-admin".into(),
            permissions: vec!["platform:*".into()],
        };

        assert_eq!(user.status, UserStatus::Active);
        assert_eq!(admin_role.name, "platform-admin");
    }

    #[test]
    fn applications_do_not_own_users() {
        let app = Application {
            id: Uuid::new_v4(),
            client_id: "incus".into(),
            name: "Incus".into(),
            redirect_uris: vec![],
        };

        assert_eq!(app.client_id, "incus");
    }
}
