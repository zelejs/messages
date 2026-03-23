use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub tenant_id: i64,
    pub org_id: Option<i64>,
    pub exp: usize,
}

pub struct JwtService {
    #[allow(dead_code)]
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn generate(&self, user_id: &str, tenant_id: i64, org_id: Option<i64>) -> Result<String, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?;

        let exp = (now.as_secs() as i64 + self.expiration) as usize;

        let claims = Claims {
            sub: user_id.to_string(),
            tenant_id,
            org_id,
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

    pub fn get_user_id(&self, token: &str) -> Result<i64, String> {
        let claims = self.verify(token)?;
        claims.sub.parse::<i64>().map_err(|e| e.to_string())
    }

    pub fn get_tenant_id(&self, token: &str) -> Result<i64, String> {
        let claims = self.verify(token)?;
        Ok(claims.tenant_id)
    }

    pub fn get_org_id(&self, token: &str) -> Result<Option<i64>, String> {
        let claims = self.verify(token)?;
        Ok(claims.org_id)
    }
}

pub struct JwtKit;

impl JwtKit {
    pub fn get_user_id_from_claims(claims: &Claims) -> Result<i64, String> {
        claims.sub.parse::<i64>().map_err(|e| e.to_string())
    }

    pub fn get_org_id_from_claims(claims: &Claims) -> Option<i64> {
        claims.org_id
    }

    pub fn get_tenant_id_from_claims(claims: &Claims) -> i64 {
        claims.tenant_id
    }
}
