use sqlx::PgPool;

use crate::domain::error::DomainError;
use crate::domain::post::Post;

pub struct PostgresPostRepository {
    pool: PgPool,
}

impl PostgresPostRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<Post>, DomainError> {
        let row = sqlx::query_as::<_, PostRow>(
            r#"
            SELECT id, title, content, author_id, created_at, updated_at
            FROM posts WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }

    pub async fn insert(
        &self,
        title: &str,
        content: &str,
        author_id: i64,
    ) -> Result<Post, DomainError> {
        let row = sqlx::query_as::<_, PostRow>(
            r#"
            INSERT INTO posts (title, content, author_id)
            VALUES ($1, $2, $3)
            RETURNING id, title, content, author_id, created_at, updated_at
            "#,
        )
        .bind(title)
        .bind(content)
        .bind(author_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    pub async fn update(
        &self,
        id: i64,
        title: &str,
        content: &str,
    ) -> Result<Option<Post>, DomainError> {
        let row = sqlx::query_as::<_, PostRow>(
            r#"
            UPDATE posts
            SET title = $2, content = $3, updated_at = NOW()
            WHERE id = $1
            RETURNING id, title, content, author_id, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(title)
        .bind(content)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }

    pub async fn delete(&self, id: i64) -> Result<bool, DomainError> {
        let res = sqlx::query(r#"DELETE FROM posts WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Post>, DomainError> {
        let rows = sqlx::query_as::<_, PostRow>(
            r#"
            SELECT id, title, content, author_id, created_at, updated_at
            FROM posts
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn count(&self) -> Result<i64, DomainError> {
        let row: (i64,) = sqlx::query_as(r#"SELECT COUNT(*)::bigint FROM posts"#)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }
}

#[derive(sqlx::FromRow)]
struct PostRow {
    id: i64,
    title: String,
    content: String,
    author_id: i64,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PostRow> for Post {
    fn from(r: PostRow) -> Self {
        Post {
            id: r.id,
            title: r.title,
            content: r.content,
            author_id: r.author_id,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
