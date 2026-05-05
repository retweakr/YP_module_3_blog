use sqlx::PgPool;

use crate::domain::error::DomainError;
use crate::domain::user::User;

pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>, DomainError> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"SELECT id, username, email, password_hash, created_at FROM users WHERE username = $1"#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }

    #[allow(dead_code)]
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"SELECT id, username, email, password_hash, created_at FROM users WHERE email = $1"#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }

    pub async fn insert(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
    ) -> Result<User, DomainError> {
        let result = sqlx::query_as::<_, UserRow>(
            r#"
            INSERT INTO users (username, email, password_hash)
            VALUES ($1, $2, $3)
            RETURNING id, username, email, password_hash, created_at
            "#,
        )
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await;

        match result {
            Ok(row) => Ok(row.into()),
            Err(sqlx::Error::Database(db)) => {
                if db.constraint() == Some("users_username_key")
                    || db.constraint() == Some("users_email_key")
                {
                    Err(DomainError::UserAlreadyExists)
                } else {
                    Err(DomainError::Database(db.to_string()))
                }
            }
            Err(e) => Err(DomainError::Database(e.to_string())),
        }
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: i64,
    username: String,
    email: String,
    password_hash: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<UserRow> for User {
    fn from(r: UserRow) -> Self {
        User {
            id: r.id,
            username: r.username,
            email: r.email,
            password_hash: r.password_hash,
            created_at: r.created_at,
        }
    }
}
