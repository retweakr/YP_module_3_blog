use std::sync::Arc;

use actix_web::dev::ServiceRequest;
use actix_web::error::ErrorUnauthorized;
use actix_web::{Error, HttpMessage};
use actix_web_httpauth::extractors::bearer::BearerAuth;

use crate::infrastructure::jwt::{Claims, JwtService};

/// Пользователь после успешной проверки JWT в bearer-middleware.
#[derive(Clone, Debug)]
pub struct AuthenticatedUser {
    pub user_id: i64,
    #[allow(dead_code)]
    pub username: String,
}

impl From<Claims> for AuthenticatedUser {
    fn from(c: Claims) -> Self {
        AuthenticatedUser {
            user_id: c.user_id,
            username: c.username,
        }
    }
}

/// Проверяет заголовок `Authorization: Bearer …`, декодирует JWT, кладёт [`AuthenticatedUser`] в расширения запроса.
pub async fn jwt_validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let jwt = match req.app_data::<actix_web::web::Data<Arc<JwtService>>>() {
        Some(j) => j.clone(),
        None => {
            return Err((
                ErrorUnauthorized("JWT service not configured"),
                req,
            ))
        }
    };

    let claims = match jwt.verify_token(credentials.token()) {
        Ok(c) => c,
        Err(_) => {
            return Err((ErrorUnauthorized("invalid or expired token"), req));
        }
    };

    req.extensions_mut().insert(AuthenticatedUser::from(claims));
    Ok(req)
}
