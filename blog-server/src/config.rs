//! Порты прослушивания и время жизни JWT для всего бинарника.

pub const HTTP_BIND: &str = "0.0.0.0:8080";
pub const GRPC_BIND: &str = "0.0.0.0:50051";

/// Срок действия JWT (24 часа).
pub const JWT_EXPIRY_SECS: i64 = 24 * 60 * 60;
