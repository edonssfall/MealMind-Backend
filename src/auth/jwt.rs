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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, JwtConfig};
    use sqlx::postgres::PgPoolOptions;
    use std::sync::Arc;

    fn make_state_with_jwt(secret: &str, issuer: &str, audience: &str) -> AppState {
        // Use a lazily connecting pool to avoid touching a real DB during unit tests
        let db = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost:5432/postgres")
            .expect("lazy pool should construct");
        let config = Arc::new(AppConfig {
            database_url: "postgres://postgres:postgres@localhost:5432/postgres".into(),
            jwt: JwtConfig {
                secret: secret.into(),
                issuer: issuer.into(),
                audience: audience.into(),
                ttl_minutes: 5,
                refresh_ttl_minutes: 60,
            },
        });
        AppState { db, config }
    }

    fn make_keys(secret: &str, issuer: &str, audience: &str) -> JwtKeys {
        let state = make_state_with_jwt(secret, issuer, audience);
        JwtKeys::from_ref(&state)
    }

    #[tokio::test]
    async fn sign_and_verify_access_token() {
        let keys = make_keys("dev-secret", "test-issuer", "test-aud");
        let user_id = Uuid::new_v4();
        let token = keys.sign_access(user_id).expect("sign access");
        let claims = keys.verify(&token).expect("verify token");
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.iss, "test-issuer");
        assert_eq!(claims.aud, "test-aud");
        assert_eq!(claims.kind, TokenKind::Access);
    }

    #[tokio::test]
    async fn sign_and_verify_refresh_token_and_verify_refresh() {
        let keys = make_keys("dev-secret", "iss", "aud");
        let user_id = Uuid::new_v4();
        let token = keys.sign_refresh(user_id).expect("sign refresh");
        let claims = keys.verify_refresh(&token).expect("verify refresh");
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.kind, TokenKind::Refresh);
    }

    #[tokio::test]
    async fn verify_refresh_rejects_access_token() {
        let keys = make_keys("dev-secret", "iss", "aud");
        let token = keys.sign_access(Uuid::new_v4()).expect("sign access");
        let err = keys.verify_refresh(&token).unwrap_err();
        assert!(err.to_string().contains("not a refresh token"));
    }

    #[tokio::test]
    async fn verify_rejects_wrong_issuer_or_audience() {
        let good_keys = make_keys("same-secret", "good-iss", "good-aud");
        let bad_keys = make_keys("same-secret", "bad-iss", "bad-aud");
        let token = good_keys.sign_access(Uuid::new_v4()).expect("sign access");
        // Using different issuer/audience in validation should fail
        let err = bad_keys.verify(&token).unwrap_err();
        let msg = err.to_string();
        assert!(msg.len() > 0);
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
