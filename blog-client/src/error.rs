//! Ошибки [`BlogClient`]: HTTP, транспорт gRPC и сопоставление кодов ответа.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlogClientError {
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    GrpcTransport(#[from] tonic::transport::Error),
    #[error("gRPC error: {0}")]
    GrpcStatus(#[from] tonic::Status),
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("conflict: resource already exists")]
    Conflict,
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("wrong transport for this client")]
    WrongTransport,
    #[error("authentication required (missing token)")]
    MissingToken,
}
