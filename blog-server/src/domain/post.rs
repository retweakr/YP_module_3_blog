use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub author_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePostRequest {
    pub title: String,
    pub content: String,
}

impl Post {
    /// Вспомогательный конструктор по ТЗ курса (id и время задаёт БД при сохранении).
    #[allow(dead_code)]
    pub fn new(title: String, content: String, author_id: i64) -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            title,
            content,
            author_id,
            created_at: now,
            updated_at: now,
        }
    }
}
