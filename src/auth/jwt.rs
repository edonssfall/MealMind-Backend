use std::time::Duration;

use axum::{extract::{FromRef, FromRequestParts}, http::{request::Parts, StatusCode}};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use time::{Duration as TimeDuration, OffsetDateTime};
use uuid::Uuid;

use crate::{config::JwtConfig, db::AppState};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: usize,
    pub iat: usize,
    pub iss: String,
    pub aud: String,
}

#[derive(Clone)]
pub struct JwtKeys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
    pub issuer: String,
    pub audience: String,
    pub ttl: Duration,
}

impl FromRef<AppState> for JwtKeys {
    fn from_ref(state: &AppState) -> Self {
        let JwtConfig { secret, issuer, audience, ttl_minutes } = state.config.jwt.clone();
        Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
            issuer,
            audience,
            ttl: Duration::from_secs((ttl_minutes as u64) * 60),
        }
    }
}

impl JwtKeys {
    pub fn sign(&self, user_id: Uuid) -> anyhow::Result<String> {
        let now = OffsetDateTime::now_utc();
        let exp = now + TimeDuration::seconds(self.ttl.as_secs() as i64);
        let claims = Claims {
            sub: user_id,
            iat: now.unix_timestamp() as usize,
            exp: exp.unix_timestamp() as usize,
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
        };
        Ok(encode(&Header::default(), &claims, &self.encoding)?)
    }

    pub fn verify(&self, token: &str) -> anyhow::Result<Claims> {
        let mut validation = Validation::default();
        validation.set_audience(&[self.audience.clone()]);
        validation.set_issuer(&[self.issuer.clone()]);
        let data = decode::<Claims>(token, &self.decoding, &validation)?;
        Ok(data.claims)
    }
}

pub struct AuthUser(pub Uuid);

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    JwtKeys: FromRef<S>,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let keys = JwtKeys::from_ref(state);
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "Missing Authorization header".to_string()))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or((StatusCode::UNAUTHORIZED, "Invalid Authorization header".to_string()))?;

        let claims = keys
            .verify(token)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired token".to_string()))?;

        Ok(AuthUser(claims.sub))
    }
}
