//! REST-запросы к `/api/...` HTTP-сервера блога.

use reqwest::Client;

use crate::error::BlogClientError;
use crate::model::{
    AuthResponse, CreatePostBody, ListPostsResponse, LoginBody, Post, RegisterBody, UpdatePostBody,
};

fn trim_base(base: &str) -> String {
    base.trim_end_matches('/').to_string()
}

pub async fn register(
    http: &Client,
    base: &str,
    username: &str,
    email: &str,
    password: &str,
) -> Result<AuthResponse, BlogClientError> {
    let url = format!("{}/api/auth/register", trim_base(base));
    let res = http
        .post(url)
        .json(&RegisterBody {
            username: username.to_string(),
            email: email.to_string(),
            password: password.to_string(),
        })
        .send()
        .await?;
    map_auth_response(res).await
}

pub async fn login(
    http: &Client,
    base: &str,
    username: &str,
    password: &str,
) -> Result<AuthResponse, BlogClientError> {
    let url = format!("{}/api/auth/login", trim_base(base));
    let res = http
        .post(url)
        .json(&LoginBody {
            username: username.to_string(),
            password: password.to_string(),
        })
        .send()
        .await?;
    map_auth_response(res).await
}

async fn map_auth_response(res: reqwest::Response) -> Result<AuthResponse, BlogClientError> {
    let status = res.status();
    if status == reqwest::StatusCode::CONFLICT {
        return Err(BlogClientError::Conflict);
    }
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(BlogClientError::Unauthorized);
    }
    if !status.is_success() {
        return Err(BlogClientError::InvalidRequest(format!(
            "HTTP {}",
            status
        )));
    }
    Ok(res.json().await?)
}

pub async fn create_post(
    http: &Client,
    base: &str,
    token: &str,
    title: &str,
    content: &str,
) -> Result<Post, BlogClientError> {
    let url = format!("{}/api/posts", trim_base(base));
    let res = http
        .post(url)
        .header("Authorization", format!("Bearer {}", token))
        .json(&CreatePostBody {
            title: title.to_string(),
            content: content.to_string(),
        })
        .send()
        .await?;
    map_post_response(res).await
}

pub async fn get_post(http: &Client, base: &str, id: i64) -> Result<Post, BlogClientError> {
    let url = format!("{}/api/posts/{}", trim_base(base), id);
    let res = http.get(url).send().await?;
    map_post_response(res).await
}

pub async fn update_post(
    http: &Client,
    base: &str,
    token: &str,
    id: i64,
    title: &str,
    content: &str,
) -> Result<Post, BlogClientError> {
    let url = format!("{}/api/posts/{}", trim_base(base), id);
    let res = http
        .put(url)
        .header("Authorization", format!("Bearer {}", token))
        .json(&UpdatePostBody {
            title: title.to_string(),
            content: content.to_string(),
        })
        .send()
        .await?;
    map_post_response(res).await
}

pub async fn delete_post(
    http: &Client,
    base: &str,
    token: &str,
    id: i64,
) -> Result<(), BlogClientError> {
    let url = format!("{}/api/posts/{}", trim_base(base), id);
    let res = http
        .delete(url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
    let status = res.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(BlogClientError::NotFound);
    }
    if status == reqwest::StatusCode::FORBIDDEN {
        return Err(BlogClientError::Unauthorized);
    }
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(BlogClientError::Unauthorized);
    }
    if !status.is_success() {
        return Err(BlogClientError::InvalidRequest(format!(
            "HTTP {}",
            status
        )));
    }
    Ok(())
}

pub async fn list_posts(
    http: &Client,
    base: &str,
    limit: i64,
    offset: i64,
) -> Result<ListPostsResponse, BlogClientError> {
    let url = format!(
        "{}/api/posts?limit={}&offset={}",
        trim_base(base),
        limit,
        offset
    );
    let res = http.get(url).send().await?;
    let status = res.status();
    if !status.is_success() {
        return Err(BlogClientError::InvalidRequest(format!(
            "HTTP {}",
            status
        )));
    }
    Ok(res.json().await?)
}

async fn map_post_response(res: reqwest::Response) -> Result<Post, BlogClientError> {
    let status = res.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(BlogClientError::NotFound);
    }
    if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(BlogClientError::Unauthorized);
    }
    if !status.is_success() {
        return Err(BlogClientError::InvalidRequest(format!(
            "HTTP {}",
            status
        )));
    }
    Ok(res.json().await?)
}
