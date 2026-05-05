use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Данные пользователя в ответах API (без хеша пароля).
#[derive(Debug, Serialize)]
pub struct UserPublic {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

impl From<&User> for UserPublic {
    fn from(u: &User) -> Self {
        UserPublic {
            id: u.id,
            username: u.username.clone(),
            email: u.email.clone(),
            created_at: u.created_at,
        }
    }
}
