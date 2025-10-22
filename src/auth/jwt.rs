use std::time::Duration;

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use time::{Duration as TimeDuration, OffsetDateTime};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::{config::JwtConfig, db::AppState};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Access,
    Refresh,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: usize,
    pub iat: usize,
    pub iss: String,
    pub aud: String,
    pub kind: TokenKind,
}

#[derive(Clone)]
pub struct JwtKeys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
    pub issuer: String,
    pub audience: String,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
}

impl FromRef<AppState> for JwtKeys {
    fn from_ref(state: &AppState) -> Self {
        let JwtConfig {
            secret,
            issuer,
            audience,
            ttl_minutes,
            refresh_ttl_minutes,
        } = state.config.jwt.clone();
        Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
            issuer,
            audience,
            access_ttl: Duration::from_secs((ttl_minutes as u64) * 60),
            refresh_ttl: Duration::from_secs((refresh_ttl_minutes as u64) * 60),
        }
    }
}

impl JwtKeys {
    fn sign_with_kind(&self, user_id: Uuid, kind: TokenKind) -> anyhow::Result<String> {
        let now = OffsetDateTime::now_utc();
        let ttl = match kind {
            TokenKind::Access => self.access_ttl,
            TokenKind::Refresh => self.refresh_ttl,
        };
        let exp = now + TimeDuration::seconds(ttl.as_secs() as i64);
        let claims = Claims {
            sub: user_id,
            iat: now.unix_timestamp() as usize,
            exp: exp.unix_timestamp() as usize,
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            kind,
        };
        let token = encode(&Header::default(), &claims, &self.encoding)?;
        debug!(user_id = %user_id, kind = ?kind, "jwt signed");
        Ok(token)
    }

    pub fn sign_access(&self, user_id: Uuid) -> anyhow::Result<String> {
        self.sign_with_kind(user_id, TokenKind::Access)
    }
    pub fn sign_refresh(&self, user_id: Uuid) -> anyhow::Result<String> {
        self.sign_with_kind(user_id, TokenKind::Refresh)
    }

    pub fn verify(&self, token: &str) -> anyhow::Result<Claims> {
        let mut validation = Validation::default();
        validation.set_audience(std::slice::from_ref(&self.audience));
        validation.set_issuer(std::slice::from_ref(&self.issuer));
        let data = decode::<Claims>(token, &self.decoding, &validation)?;
        debug!(user_id = %data.claims.sub, kind = ?data.claims.kind, "jwt verified");
        Ok(data.claims)
    }

    pub fn verify_refresh(&self, token: &str) -> anyhow::Result<Claims> {
        let claims = self.verify(token)?;
        if claims.kind != TokenKind::Refresh {
            anyhow::bail!("not a refresh token");
        }
        Ok(claims)
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
            .ok_or((
                StatusCode::UNAUTHORIZED,
                "Missing Authorization header".to_string(),
            ))?;

        let token = auth_header.strip_prefix("Bearer ").ok_or((
            StatusCode::UNAUTHORIZED,
            "Invalid Authorization header".to_string(),
        ))?;

        let claims = match keys.verify(token) {
            Ok(c) => c,
            Err(_) => {
                warn!("invalid or expired token");
                return Err((
                    StatusCode::UNAUTHORIZED,
                    "Invalid or expired token".to_string(),
                ));
            }
        };

        if claims.kind != TokenKind::Access {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Access token required".to_string(),
            ));
        }

        Ok(AuthUser(claims.sub))
    }
}
