//! Регистрация и вход: хеш пароля (Argon2) и выдача JWT.

use std::sync::Arc;

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand_core::OsRng;

use crate::domain::error::DomainError;
use crate::domain::user::{LoginRequest, RegisterRequest, UserPublic};
use crate::data::user_repository::PostgresUserRepository;
use crate::infrastructure::jwt::JwtService;

pub struct AuthService {
    users: Arc<PostgresUserRepository>,
    jwt: Arc<JwtService>,
}

impl AuthService {
    pub fn new(users: Arc<PostgresUserRepository>, jwt: Arc<JwtService>) -> Self {
        Self { users, jwt }
    }

    pub async fn register(
        &self,
        req: RegisterRequest,
    ) -> Result<(String, UserPublic), DomainError> {
        if req.username.is_empty() || req.email.is_empty() || req.password.is_empty() {
            return Err(DomainError::Internal("empty fields".into()));
        }
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        // В БД только хеш Argon2 в `users.password_hash`, пароль в открытом виде не храним.
        let password_hash = argon2
            .hash_password(req.password.as_bytes(), &salt)
            .map_err(|e| DomainError::Internal(e.to_string()))?
            .to_string();

        let user = self
            .users
            .insert(&req.username, &req.email, &password_hash)
            .await?;

        let token = self
            .jwt
            .generate_token(user.id, &user.username)
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok((token, UserPublic::from(&user)))
    }

    pub async fn login(&self, req: LoginRequest) -> Result<(String, UserPublic), DomainError> {
        let user = self
            .users
            .find_by_username(&req.username)
            .await?
            .ok_or(DomainError::InvalidCredentials)?;

        let parsed = PasswordHash::new(&user.password_hash)
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        Argon2::default()
            .verify_password(req.password.as_bytes(), &parsed)
            .map_err(|_| DomainError::InvalidCredentials)?;

        let token = self
            .jwt
            .generate_token(user.id, &user.username)
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok((token, UserPublic::from(&user)))
    }
}
