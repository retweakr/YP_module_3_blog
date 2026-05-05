//! gRPC-обёртка: те же сервисы приложения, что и у HTTP — JWT в метаданных `authorization` (Bearer).

use std::sync::Arc;

use tonic::{metadata::MetadataMap, Request, Response, Status};

use crate::application::auth_service::AuthService;
use crate::application::blog_service::BlogService;
use crate::domain::error::DomainError;
use crate::domain::post::{CreatePostRequest, UpdatePostRequest};
use crate::domain::user::{LoginRequest, RegisterRequest};
use crate::infrastructure::jwt::JwtService;

pub mod pb {
    tonic::include_proto!("blog.v1");
}

use pb::blog_service_server::BlogService as GrpcBlog;
use pb::{
    AuthResponse, DeleteResponse, ListPostsResponse, LoginRequest as PbLogin,
    PostResponse, RegisterRequest as PbRegister, User as PbUser,
};

pub struct BlogGrpcService {
    pub auth: Arc<AuthService>,
    pub blog: Arc<BlogService>,
    pub jwt: Arc<JwtService>,
}

/// Ожидается заголовок `authorization: Bearer <jwt>` (как HTTP Bearer).
fn bearer_from_metadata(meta: &MetadataMap) -> Result<String, Status> {
    let val = meta
        .get("authorization")
        .ok_or_else(|| Status::unauthenticated("missing authorization metadata"))?
        .to_str()
        .map_err(|_| Status::unauthenticated("invalid authorization header"))?;
    let prefix = "Bearer ";
    if val.starts_with(prefix) {
        Ok(val[prefix.len()..].trim().to_string())
    } else {
        Err(Status::unauthenticated("expected Bearer token"))
    }
}

fn map_domain_grpc(e: DomainError) -> Status {
    match e {
        DomainError::UserNotFound => Status::not_found("user not found"),
        DomainError::UserAlreadyExists => Status::already_exists("user already exists"),
        DomainError::InvalidCredentials => Status::unauthenticated("invalid credentials"),
        DomainError::PostNotFound => Status::not_found("post not found"),
        DomainError::Forbidden => Status::permission_denied("forbidden"),
        DomainError::Database(msg) | DomainError::Internal(msg) => {
            tracing::error!("grpc domain error: {msg}");
            Status::internal("internal error")
        }
    }
}

/// Преобразование публичных полей пользователя в сообщение protobuf `User`.
fn user_public_to_pb(id: i64, username: String, email: String) -> PbUser {
    PbUser {
        id,
        username,
        email,
    }
}

fn post_to_pb(p: &crate::domain::post::Post) -> pb::Post {
    pb::Post {
        id: p.id,
        title: p.title.clone(),
        content: p.content.clone(),
        author_id: p.author_id,
        created_at_unix: p.created_at.timestamp(),
        updated_at_unix: p.updated_at.timestamp(),
    }
}

#[tonic::async_trait]
impl GrpcBlog for BlogGrpcService {
    async fn register(
        &self,
        request: Request<PbRegister>,
    ) -> Result<Response<AuthResponse>, Status> {
        let r = request.into_inner();
        if r.username.is_empty() || r.email.is_empty() || r.password.is_empty() {
            return Err(Status::invalid_argument("username, email, password required"));
        }
        let req = RegisterRequest {
            username: r.username,
            email: r.email,
            password: r.password,
        };
        match self.auth.register(req).await {
            Ok((token, public)) => Ok(Response::new(AuthResponse {
                token,
                user: Some(user_public_to_pb(public.id, public.username, public.email)),
            })),
            Err(e) => Err(map_domain_grpc(e)),
        }
    }

    async fn login(
        &self,
        request: Request<PbLogin>,
    ) -> Result<Response<AuthResponse>, Status> {
        let r = request.into_inner();
        let req = LoginRequest {
            username: r.username,
            password: r.password,
        };
        match self.auth.login(req).await {
            Ok((token, public)) => Ok(Response::new(AuthResponse {
                token,
                user: Some(user_public_to_pb(public.id, public.username, public.email)),
            })),
            Err(e) => Err(map_domain_grpc(e)),
        }
    }

    async fn create_post(
        &self,
        request: Request<pb::CreatePostRequest>,
    ) -> Result<Response<PostResponse>, Status> {
        let meta = request.metadata().clone();
        let token = bearer_from_metadata(&meta)?;
        let claims = self
            .jwt
            .verify_token(&token)
            .map_err(|_| Status::unauthenticated("invalid token"))?;
        let body = request.into_inner();
        if body.title.is_empty() || body.content.is_empty() {
            return Err(Status::invalid_argument("title and content required"));
        }
        let req = CreatePostRequest {
            title: body.title,
            content: body.content,
        };
        match self.blog.create_post(claims.user_id, req).await {
            Ok(post) => Ok(Response::new(PostResponse {
                post: Some(post_to_pb(&post)),
            })),
            Err(e) => Err(map_domain_grpc(e)),
        }
    }

    async fn get_post(
        &self,
        request: Request<pb::GetPostRequest>,
    ) -> Result<Response<PostResponse>, Status> {
        let id = request.into_inner().id;
        match self.blog.get_post(id).await {
            Ok(post) => Ok(Response::new(PostResponse {
                post: Some(post_to_pb(&post)),
            })),
            Err(e) => Err(map_domain_grpc(e)),
        }
    }

    async fn update_post(
        &self,
        request: Request<pb::UpdatePostRequest>,
    ) -> Result<Response<PostResponse>, Status> {
        let meta = request.metadata().clone();
        let token = bearer_from_metadata(&meta)?;
        let claims = self
            .jwt
            .verify_token(&token)
            .map_err(|_| Status::unauthenticated("invalid token"))?;
        let r = request.into_inner();
        let req = UpdatePostRequest {
            title: r.title,
            content: r.content,
        };
        match self.blog.update_post(r.id, claims.user_id, req).await {
            Ok(post) => Ok(Response::new(PostResponse {
                post: Some(post_to_pb(&post)),
            })),
            Err(e) => Err(map_domain_grpc(e)),
        }
    }

    async fn delete_post(
        &self,
        request: Request<pb::DeletePostRequest>,
    ) -> Result<Response<DeleteResponse>, Status> {
        let meta = request.metadata().clone();
        let token = bearer_from_metadata(&meta)?;
        let claims = self
            .jwt
            .verify_token(&token)
            .map_err(|_| Status::unauthenticated("invalid token"))?;
        let id = request.into_inner().id;
        match self.blog.delete_post(id, claims.user_id).await {
            Ok(()) => Ok(Response::new(DeleteResponse {})),
            Err(e) => Err(map_domain_grpc(e)),
        }
    }

    async fn list_posts(
        &self,
        request: Request<pb::ListPostsRequest>,
    ) -> Result<Response<ListPostsResponse>, Status> {
        let r = request.into_inner();
        let limit = if r.limit <= 0 { 10 } else { r.limit as i64 }.min(500);
        let offset = r.offset.max(0) as i64;
        match self.blog.list_posts(limit, offset).await {
            Ok((posts, total)) => Ok(Response::new(ListPostsResponse {
                posts: posts.iter().map(post_to_pb).collect(),
                total,
                limit: r.limit.max(1).min(500),
                offset: r.offset.max(0),
            })),
            Err(e) => Err(map_domain_grpc(e)),
        }
    }
}
