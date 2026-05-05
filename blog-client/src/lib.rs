//! Общий клиент API для CLI и других бинарников.
//!
//! - [`Transport::Http`] — базовый URL, например `http://localhost:8080`.
//! - [`Transport::Grpc`] — адрес tonic, например `http://localhost:50051`.
//! После `register` / `login` JWT хранится в памяти для защищённых запросов.

pub mod error;
pub mod grpc_client;
pub mod http_client;
mod model;

pub use error::BlogClientError;
pub use model::{AuthResponse, ListPostsResponse, Post, User};

pub mod pb {
    tonic::include_proto!("blog.v1");
}

use std::time::Duration;

use tonic::transport::Endpoint;

use crate::model::AuthResponse as AuthModel;

#[derive(Clone, Debug)]
pub enum Transport {
    Http(String),
    Grpc(String),
}

pub struct BlogClient {
    transport: Transport,
    http: Option<reqwest::Client>,
    grpc: Option<pb::blog_service_client::BlogServiceClient<tonic::transport::Channel>>,
    /// Последний JWT после успешной аутентификации; уходит в Bearer для HTTP/gRPC.
    token: Option<String>,
}

impl BlogClient {
    pub async fn new(transport: Transport) -> Result<Self, BlogClientError> {
        match &transport {
            Transport::Http(_) => {
                let http = reqwest::Client::builder()
                    .timeout(Duration::from_secs(30))
                    .build()?;
                Ok(Self {
                    transport,
                    http: Some(http),
                    grpc: None,
                    token: None,
                })
            }
            Transport::Grpc(endpoint) => {
                let channel = Endpoint::from_shared(endpoint.clone())?
                    .connect()
                    .await?;
                let grpc = pb::blog_service_client::BlogServiceClient::new(channel);
                Ok(Self {
                    transport,
                    http: None,
                    grpc: Some(grpc),
                    token: None,
                })
            }
        }
    }

    pub fn set_token(&mut self, token: impl Into<String>) {
        self.token = Some(token.into());
    }

    pub fn get_token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    pub fn transport(&self) -> &Transport {
        &self.transport
    }

    fn token_required(&self) -> Result<&str, BlogClientError> {
        self.token
            .as_deref()
            .ok_or(BlogClientError::MissingToken)
    }

    pub async fn register(
        &mut self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<AuthModel, BlogClientError> {
        let auth = match &mut self.transport {
            Transport::Http(base) => {
                let http = self.http.as_ref().ok_or(BlogClientError::WrongTransport)?;
                http_client::register(http, base, username, email, password).await?
            }
            Transport::Grpc(_) => {
                let grpc = self.grpc.as_mut().ok_or(BlogClientError::WrongTransport)?;
                grpc_client::register(grpc, username, email, password).await?
            }
        };
        self.token = Some(auth.token.clone());
        Ok(auth)
    }

    pub async fn login(&mut self, username: &str, password: &str) -> Result<AuthModel, BlogClientError> {
        let auth = match &mut self.transport {
            Transport::Http(base) => {
                let http = self.http.as_ref().ok_or(BlogClientError::WrongTransport)?;
                http_client::login(http, base, username, password).await?
            }
            Transport::Grpc(_) => {
                let grpc = self.grpc.as_mut().ok_or(BlogClientError::WrongTransport)?;
                grpc_client::login(grpc, username, password).await?
            }
        };
        self.token = Some(auth.token.clone());
        Ok(auth)
    }

    pub async fn create_post(&mut self, title: &str, content: &str) -> Result<Post, BlogClientError> {
        let token = self.token_required()?.to_string();
        match &mut self.transport {
            Transport::Http(base) => {
                let http = self.http.as_ref().ok_or(BlogClientError::WrongTransport)?;
                http_client::create_post(http, base, &token, title, content).await
            }
            Transport::Grpc(_) => {
                let grpc = self.grpc.as_mut().ok_or(BlogClientError::WrongTransport)?;
                grpc_client::create_post(grpc, &token, title, content).await
            }
        }
    }

    pub async fn get_post(&mut self, id: i64) -> Result<Post, BlogClientError> {
        match &mut self.transport {
            Transport::Http(base) => {
                let http = self.http.as_ref().ok_or(BlogClientError::WrongTransport)?;
                http_client::get_post(http, base, id).await
            }
            Transport::Grpc(_) => {
                let grpc = self.grpc.as_mut().ok_or(BlogClientError::WrongTransport)?;
                grpc_client::get_post(grpc, id).await
            }
        }
    }

    pub async fn update_post(
        &mut self,
        id: i64,
        title: &str,
        content: &str,
    ) -> Result<Post, BlogClientError> {
        let token = self.token_required()?.to_string();
        match &mut self.transport {
            Transport::Http(base) => {
                let http = self.http.as_ref().ok_or(BlogClientError::WrongTransport)?;
                http_client::update_post(http, base, &token, id, title, content).await
            }
            Transport::Grpc(_) => {
                let grpc = self.grpc.as_mut().ok_or(BlogClientError::WrongTransport)?;
                grpc_client::update_post(grpc, &token, id, title, content).await
            }
        }
    }

    pub async fn delete_post(&mut self, id: i64) -> Result<(), BlogClientError> {
        let token = self.token_required()?.to_string();
        match &mut self.transport {
            Transport::Http(base) => {
                let http = self.http.as_ref().ok_or(BlogClientError::WrongTransport)?;
                http_client::delete_post(http, base, &token, id).await
            }
            Transport::Grpc(_) => {
                let grpc = self.grpc.as_mut().ok_or(BlogClientError::WrongTransport)?;
                grpc_client::delete_post(grpc, &token, id).await
            }
        }
    }

    pub async fn list_posts(&mut self, limit: i64, offset: i64) -> Result<ListPostsResponse, BlogClientError> {
        match &mut self.transport {
            Transport::Http(base) => {
                let http = self.http.as_ref().ok_or(BlogClientError::WrongTransport)?;
                http_client::list_posts(http, base, limit, offset).await
            }
            Transport::Grpc(_) => {
                let grpc = self.grpc.as_mut().ok_or(BlogClientError::WrongTransport)?;
                grpc_client::list_posts(grpc, limit as i32, offset as i32).await
            }
        }
    }
}
