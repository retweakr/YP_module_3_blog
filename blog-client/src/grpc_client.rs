//! Вызовы через tonic; для защищённых RPC в метаданные добавляется `authorization: Bearer <token>`.

use tonic::metadata::MetadataValue;
use tonic::Request;

use crate::error::BlogClientError;
use crate::model::{AuthResponse, Post, User};
use crate::pb;

fn bearer_meta(token: &str) -> Result<MetadataValue<tonic::metadata::Ascii>, BlogClientError> {
    let s = format!("Bearer {}", token);
    MetadataValue::try_from(s).map_err(|_| BlogClientError::InvalidRequest("bad token".into()))
}

fn map_status(e: tonic::Status) -> BlogClientError {
    use tonic::Code;
    match e.code() {
        Code::NotFound => BlogClientError::NotFound,
        Code::Unauthenticated => BlogClientError::Unauthorized,
        Code::AlreadyExists => BlogClientError::Conflict,
        Code::PermissionDenied => BlogClientError::Unauthorized,
        Code::InvalidArgument => BlogClientError::InvalidRequest(e.message().to_string()),
        _ => BlogClientError::GrpcStatus(e),
    }
}

fn pb_user_to_model(u: pb::User) -> User {
    User {
        id: u.id,
        username: u.username,
        email: u.email,
        // У сообщения protobuf `User` нет `created_at`; подставляем время для отображения.
        created_at: chrono::Utc::now(),
    }
}

fn pb_post_to_model(p: pb::Post) -> Result<Post, BlogClientError> {
    Ok(Post {
        id: p.id,
        title: p.title,
        content: p.content,
        author_id: p.author_id,
        created_at: chrono::DateTime::from_timestamp(p.created_at_unix, 0)
            .unwrap_or_else(|| chrono::DateTime::UNIX_EPOCH),
        updated_at: chrono::DateTime::from_timestamp(p.updated_at_unix, 0)
            .unwrap_or_else(|| chrono::DateTime::UNIX_EPOCH),
    })
}

pub async fn register(
    client: &mut pb::blog_service_client::BlogServiceClient<tonic::transport::Channel>,
    username: &str,
    email: &str,
    password: &str,
) -> Result<AuthResponse, BlogClientError> {
    let req = pb::RegisterRequest {
        username: username.into(),
        email: email.into(),
        password: password.into(),
    };
    let res = client.register(Request::new(req)).await.map_err(map_status)?;
    let inner = res.into_inner();
    let user_pb = inner.user.ok_or_else(|| {
        BlogClientError::InvalidRequest("missing user in auth response".into())
    })?;
    Ok(AuthResponse {
        token: inner.token,
        user: pb_user_to_model(user_pb),
    })
}

pub async fn login(
    client: &mut pb::blog_service_client::BlogServiceClient<tonic::transport::Channel>,
    username: &str,
    password: &str,
) -> Result<AuthResponse, BlogClientError> {
    let req = pb::LoginRequest {
        username: username.into(),
        password: password.into(),
    };
    let res = client.login(Request::new(req)).await.map_err(map_status)?;
    let inner = res.into_inner();
    let user_pb = inner.user.ok_or_else(|| {
        BlogClientError::InvalidRequest("missing user in auth response".into())
    })?;
    Ok(AuthResponse {
        token: inner.token,
        user: pb_user_to_model(user_pb),
    })
}

pub async fn create_post(
    client: &mut pb::blog_service_client::BlogServiceClient<tonic::transport::Channel>,
    token: &str,
    title: &str,
    content: &str,
) -> Result<Post, BlogClientError> {
    let mut req = Request::new(pb::CreatePostRequest {
        title: title.into(),
        content: content.into(),
    });
    req.metadata_mut().insert("authorization", bearer_meta(token)?);
    let res = client.create_post(req).await.map_err(map_status)?;
    let post = res
        .into_inner()
        .post
        .ok_or_else(|| BlogClientError::InvalidRequest("missing post".into()))?;
    pb_post_to_model(post)
}

pub async fn get_post(
    client: &mut pb::blog_service_client::BlogServiceClient<tonic::transport::Channel>,
    id: i64,
) -> Result<Post, BlogClientError> {
    let req = pb::GetPostRequest { id };
    let res = client.get_post(Request::new(req)).await.map_err(map_status)?;
    let post = res
        .into_inner()
        .post
        .ok_or_else(|| BlogClientError::NotFound)?;
    pb_post_to_model(post)
}

pub async fn update_post(
    client: &mut pb::blog_service_client::BlogServiceClient<tonic::transport::Channel>,
    token: &str,
    id: i64,
    title: &str,
    content: &str,
) -> Result<Post, BlogClientError> {
    let mut req = Request::new(pb::UpdatePostRequest {
        id,
        title: title.into(),
        content: content.into(),
    });
    req.metadata_mut().insert("authorization", bearer_meta(token)?);
    let res = client.update_post(req).await.map_err(map_status)?;
    let post = res
        .into_inner()
        .post
        .ok_or_else(|| BlogClientError::InvalidRequest("missing post".into()))?;
    pb_post_to_model(post)
}

pub async fn delete_post(
    client: &mut pb::blog_service_client::BlogServiceClient<tonic::transport::Channel>,
    token: &str,
    id: i64,
) -> Result<(), BlogClientError> {
    let mut req = Request::new(pb::DeletePostRequest { id });
    req.metadata_mut().insert("authorization", bearer_meta(token)?);
    client.delete_post(req).await.map_err(map_status)?;
    Ok(())
}

pub async fn list_posts(
    client: &mut pb::blog_service_client::BlogServiceClient<tonic::transport::Channel>,
    limit: i32,
    offset: i32,
) -> Result<crate::model::ListPostsResponse, BlogClientError> {
    let req = pb::ListPostsRequest { limit, offset };
    let res = client.list_posts(Request::new(req)).await.map_err(map_status)?;
    let inner = res.into_inner();
    let mut posts = Vec::new();
    for p in inner.posts {
        posts.push(pb_post_to_model(p)?);
    }
    Ok(crate::model::ListPostsResponse {
        posts,
        total: inner.total,
        limit: inner.limit as i64,
        offset: inner.offset as i64,
    })
}
