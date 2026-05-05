//! HTTP-обработчики: JSON ↔ домен, преобразование `DomainError` в коды ответа HTTP.

use std::sync::Arc;

use actix_web::{web, HttpResponse, Result};
use serde::Serialize;

use crate::application::auth_service::AuthService;
use crate::application::blog_service::BlogService;
use crate::domain::error::DomainError;
use crate::domain::post::{CreatePostRequest, UpdatePostRequest};
use crate::domain::user::{LoginRequest, RegisterRequest, UserPublic};
use crate::presentation::middleware::AuthenticatedUser;

#[derive(Serialize)]
pub struct AuthJsonResponse {
    pub token: String,
    pub user: UserPublic,
}

#[derive(Serialize)]
pub struct PostJson<'a> {
    pub id: i64,
    pub title: &'a str,
    pub content: &'a str,
    pub author_id: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl<'a> From<&'a crate::domain::post::Post> for PostJson<'a> {
    fn from(p: &'a crate::domain::post::Post) -> Self {
        PostJson {
            id: p.id,
            title: &p.title,
            content: &p.content,
            author_id: p.author_id,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[derive(Serialize)]
pub struct ListPostsResponse<'a> {
    pub posts: Vec<PostJson<'a>>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Единая точка: доменная ошибка → ответ Actix (статус и тело).
fn map_domain_err(e: DomainError) -> actix_web::Error {
    match e {
        DomainError::UserAlreadyExists => {
            actix_web::error::ErrorConflict("user already exists")
        }
        DomainError::InvalidCredentials => {
            actix_web::error::ErrorUnauthorized("invalid credentials")
        }
        DomainError::PostNotFound | DomainError::UserNotFound => {
            actix_web::error::ErrorNotFound("not found")
        }
        DomainError::Forbidden => actix_web::error::ErrorForbidden("forbidden"),
        DomainError::Database(msg) | DomainError::Internal(msg) => {
            tracing::error!("domain error: {msg}");
            actix_web::error::ErrorInternalServerError("internal error")
        }
    }
}

pub async fn register(
    auth: web::Data<Arc<AuthService>>,
    body: web::Json<RegisterRequest>,
) -> Result<HttpResponse> {
    match auth.register(body.into_inner()).await {
        Ok((token, user)) => Ok(HttpResponse::Created().json(AuthJsonResponse { token, user })),
        Err(e) => Err(map_domain_err(e)),
    }
}

pub async fn login(
    auth: web::Data<Arc<AuthService>>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse> {
    match auth.login(body.into_inner()).await {
        Ok((token, user)) => Ok(HttpResponse::Ok().json(AuthJsonResponse { token, user })),
        Err(e) => Err(map_domain_err(e)),
    }
}

pub async fn create_post(
    blog: web::Data<Arc<BlogService>>,
    user: web::ReqData<AuthenticatedUser>,
    body: web::Json<CreatePostRequest>,
) -> Result<HttpResponse> {
    match blog.create_post(user.user_id, body.into_inner()).await {
        Ok(post) => {
            let pj = PostJson::from(&post);
            Ok(HttpResponse::Created().json(pj))
        }
        Err(e) => Err(map_domain_err(e)),
    }
}

pub async fn get_post(
    blog: web::Data<Arc<BlogService>>,
    path: web::Path<(i64,)>,
) -> Result<HttpResponse> {
    match blog.get_post(path.0).await {
        Ok(post) => {
            let pj = PostJson::from(&post);
            Ok(HttpResponse::Ok().json(pj))
        }
        Err(e) => Err(map_domain_err(e)),
    }
}

pub async fn update_post(
    blog: web::Data<Arc<BlogService>>,
    user: web::ReqData<AuthenticatedUser>,
    path: web::Path<(i64,)>,
    body: web::Json<UpdatePostRequest>,
) -> Result<HttpResponse> {
    match blog.update_post(path.0, user.user_id, body.into_inner()).await {
        Ok(post) => {
            let pj = PostJson::from(&post);
            Ok(HttpResponse::Ok().json(pj))
        }
        Err(e) => Err(map_domain_err(e)),
    }
}

pub async fn delete_post(
    blog: web::Data<Arc<BlogService>>,
    user: web::ReqData<AuthenticatedUser>,
    path: web::Path<(i64,)>,
) -> Result<HttpResponse> {
    match blog.delete_post(path.0, user.user_id).await {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Err(map_domain_err(e)),
    }
}

#[derive(serde::Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    10
}

pub async fn list_posts(
    blog: web::Data<Arc<BlogService>>,
    q: web::Query<ListQuery>,
) -> Result<HttpResponse> {
    let limit = q.limit.max(1).min(500);
    let offset = q.offset.max(0);
    match blog.list_posts(limit, offset).await {
        Ok((posts, total)) => {
            let posts: Vec<PostJson<'_>> = posts.iter().map(PostJson::from).collect();
            Ok(HttpResponse::Ok().json(ListPostsResponse {
                posts,
                total,
                limit,
                offset,
            }))
        }
        Err(e) => Err(map_domain_err(e)),
    }
}
