//! Подпись JWT (HS256) для Bearer в HTTP и метаданных gRPC.

use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, Algorithm};
use serde::{Deserialize, Serialize};

/// Поля внутри access-токена (проверяются на защищённых маршрутах).
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: i64,
    pub username: String,
    pub exp: i64,
}

pub struct JwtService {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl JwtService {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    pub fn generate_token(
        &self,
        user_id: i64,
        username: &str,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let exp = chrono::Utc::now().timestamp() + crate::config::JWT_EXPIRY_SECS;
        let claims = Claims {
            user_id,
            username: username.to_string(),
            exp,
        };
        jsonwebtoken::encode(&Header::default(), &claims, &self.encoding)
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true; // отклонять просроченные токены
        let data = jsonwebtoken::decode::<Claims>(token, &self.decoding, &validation)?;
        Ok(data.claims)
    }
}
