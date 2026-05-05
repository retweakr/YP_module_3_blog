//! Сценарии для постов: проверка автора при изменении/удалении; сохранение через репозитории.

use std::sync::Arc;

use crate::data::post_repository::PostgresPostRepository;
use crate::domain::error::DomainError;
use crate::domain::post::{CreatePostRequest, Post, UpdatePostRequest};

pub struct BlogService {
    posts: Arc<PostgresPostRepository>,
}

impl BlogService {
    pub fn new(posts: Arc<PostgresPostRepository>) -> Self {
        Self { posts }
    }

    pub async fn create_post(
        &self,
        author_id: i64,
        req: CreatePostRequest,
    ) -> Result<Post, DomainError> {
        if req.title.trim().is_empty() || req.content.trim().is_empty() {
            return Err(DomainError::Internal("title and content required".into()));
        }
        self.posts
            .insert(&req.title, &req.content, author_id)
            .await
    }

    pub async fn get_post(&self, id: i64) -> Result<Post, DomainError> {
        self.posts
            .find_by_id(id)
            .await?
            .ok_or(DomainError::PostNotFound)
    }

    pub async fn update_post(
        &self,
        id: i64,
        user_id: i64,
        req: UpdatePostRequest,
    ) -> Result<Post, DomainError> {
        let existing = self
            .posts
            .find_by_id(id)
            .await?
            .ok_or(DomainError::PostNotFound)?;
        // Редактировать может только автор (`user_id` из JWT).
        if existing.author_id != user_id {
            return Err(DomainError::Forbidden);
        }
        self.posts
            .update(id, &req.title, &req.content)
            .await?
            .ok_or(DomainError::PostNotFound)
    }

    pub async fn delete_post(&self, id: i64, user_id: i64) -> Result<(), DomainError> {
        let existing = self
            .posts
            .find_by_id(id)
            .await?
            .ok_or(DomainError::PostNotFound)?;
        // Как при обновлении: удалить может только автор.
        if existing.author_id != user_id {
            return Err(DomainError::Forbidden);
        }
        let deleted = self.posts.delete(id).await?;
        if deleted {
            Ok(())
        } else {
            Err(DomainError::PostNotFound)
        }
    }

    pub async fn list_posts(&self, limit: i64, offset: i64) -> Result<(Vec<Post>, i64), DomainError> {
        let posts = self.posts.list(limit, offset).await?;
        let total = self.posts.count().await?;
        Ok((posts, total))
    }
}
