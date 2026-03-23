use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub tenant_id: i64,
    pub exp: usize,
}

pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    expiration: i64,
}

impl JwtService {
    pub fn new(secret: &str, expiration: i64) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            expiration,
        }
    }

    pub fn generate(&self, user_id: &str, tenant_id: i64) -> Result<String, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?;

        let exp = (now.as_secs() as i64 + self.expiration) as usize;

        let claims = Claims {
            sub: user_id.to_string(),
            tenant_id,
            exp,
        };

        encode(&Header::default(), &claims, &self.encoding_key).map_err(|e| e.to_string())
    }

    pub fn verify(&self, token: &str) -> Result<Claims, String> {
        let token_data = decode::<Claims>(
            token,
            &self.decoding_key,
            &Validation::default(),
        )
        .map_err(|e| e.to_string())?;

        Ok(token_data.claims)
    }
}
