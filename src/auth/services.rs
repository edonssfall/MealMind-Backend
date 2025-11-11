pub(crate) use crate::auth::dto::{Claims, JwtKeys, TokenKind};
use crate::config::JwtConfig;
use crate::state::AppState;

use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use lazy_static::lazy_static;
use rand::rngs::OsRng;
use regex::Regex;
use std::time::Duration;
use time::{Duration as TimeDuration, OffsetDateTime};
use tracing::{debug, warn};
use uuid::Uuid;

// -------------------- Utils --------------------

pub(crate) fn is_valid_email(email: &str) -> bool {
    lazy_static! {
        static ref EMAIL_RE: Regex = Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$").unwrap();
    }
    EMAIL_RE.is_match(email)
}

pub fn hash_password(plain: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(plain.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?
        .to_string();
    Ok(hash)
}

pub fn verify_password(plain: &str, hash: &str) -> anyhow::Result<bool> {
    let parsed = PasswordHash::new(hash).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    Ok(Argon2::default()
        .verify_password(plain.as_bytes(), &parsed)
        .is_ok())
}

// -------------------- JWT keys --------------------

impl FromRef<AppState> for JwtKeys {
    fn from_ref(state: &AppState) -> Self {
        let JwtConfig {
            secret,
            issuer,
            audience,
            ttl_minutes,
            refresh_ttl_minutes,
        } = state.config.jwt.clone();

        JwtKeys {
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
        // Build iat/exp according to TTLs
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

        // Be explicit about HS256 to avoid silent alg mismatches
        let header = Header::new(Algorithm::HS256);
        let token = encode(&header, &claims, &self.encoding)?;
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
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = 60;

        // enable checks only when configured
        if !self.issuer.is_empty() {
            validation.set_issuer(std::slice::from_ref(&self.issuer));
        }
        if !self.audience.is_empty() {
            validation.set_audience(std::slice::from_ref(&self.audience));
        }

        let data = decode::<Claims>(token, &self.decoding, &validation)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        debug!(user_id = %data.claims.sub, kind = ?data.claims.kind, "jwt verified");
        Ok(data.claims)
    }

    pub fn verify_refresh(&self, token: &str) -> anyhow::Result<Claims> {
        // Ensure token is valid and specifically a refresh token
        let claims = self.verify(token)?;
        if claims.kind != TokenKind::Refresh {
            anyhow::bail!("not a refresh token");
        }
        Ok(claims)
    }
}

// -------------------- Auth extractor --------------------

pub struct AuthUser(pub Uuid);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    JwtKeys: FromRef<S>,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Pull JWT verification keys from state
        let keys = JwtKeys::from_ref(state);

        // Read and normalize Authorization header
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or((
                StatusCode::UNAUTHORIZED,
                "missing Authorization header".to_string(),
            ))?;

        // Be tolerant to casing and extra whitespace
        let auth_trimmed = auth_header.trim();
        let token = auth_trimmed
            .strip_prefix("Bearer ")
            .or_else(|| auth_trimmed.strip_prefix("bearer "))
            .ok_or((StatusCode::UNAUTHORIZED, "invalid auth scheme".to_string()))?;

        // Verify token and ensure it is an access token
        let claims = match keys.verify(token) {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "token verification failed");
                return Err((
                    StatusCode::UNAUTHORIZED,
                    "invalid or expired token".to_string(),
                ));
            }
        };

        if claims.kind != TokenKind::Access {
            return Err((
                StatusCode::UNAUTHORIZED,
                "access token required".to_string(),
            ));
        }

        Ok(AuthUser(claims.sub))
    }
}

// -------------------- Tests --------------------

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    // Helper to build JwtKeys from a fake AppState-like config.
    // If you already have AppState::fake(), you can use FromRef<AppState>.
    fn make_keys_with(
        secret: &str,
        issuer: &str,
        audience: &str,
        access_min: u64,
        refresh_min: u64,
    ) -> JwtKeys {
        JwtKeys {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
            issuer: issuer.to_string(),
            audience: audience.to_string(),
            access_ttl: Duration::from_secs(access_min * 60),
            refresh_ttl: Duration::from_secs(refresh_min * 60),
        }
    }

    fn make_keys() -> JwtKeys {
        make_keys_with("test-secret", "test-issuer", "test-aud", 15, 60 * 24 * 7)
    }

    #[test]
    fn email_validation_basic() {
        assert!(is_valid_email("user@example.com"));
        assert!(!is_valid_email("bad@"));
        assert!(!is_valid_email("no-at.example.com"));
    }

    #[test]
    fn hash_and_verify_roundtrip() {
        let password = "Secur3P@ssw0rd!";
        let hash = hash_password(password).expect("hashing should succeed");
        assert!(verify_password(password, &hash).expect("verify should succeed"));
    }

    #[test]
    fn verify_rejects_wrong_password() {
        let password = "correct-horse-battery-staple";
        let hash = hash_password(password).expect("hashing should succeed");
        assert!(!verify_password("wrong-password", &hash).expect("verify should not error"));
    }

    #[test]
    fn verify_errors_on_malformed_hash() {
        let err = verify_password("anything", "not-a-valid-hash").unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[tokio::test]
    async fn sign_and_verify_access_token() {
        let keys = make_keys();
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
        let keys = make_keys();
        let user_id = Uuid::new_v4();
        let token = keys.sign_refresh(user_id).expect("sign refresh");
        let claims = keys.verify_refresh(&token).expect("verify refresh");
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.kind, TokenKind::Refresh);
    }

    #[tokio::test]
    async fn verify_refresh_rejects_access_token() {
        let keys = make_keys();
        let token = keys.sign_access(Uuid::new_v4()).expect("sign access");
        let err = keys.verify_refresh(&token).unwrap_err();
        assert!(err.to_string().contains("not a refresh token"));
    }

    #[tokio::test]
    async fn verify_rejects_wrong_issuer_or_audience() {
        let good = make_keys_with("s", "iss-a", "aud-a", 15, 60);
        let bad = make_keys_with("s", "iss-b", "aud-b", 15, 60); // same secret, different iss/aud
        let token = good.sign_access(Uuid::new_v4()).expect("sign");
        let err = bad.verify(&token).unwrap_err();
        let msg = err.to_string();
        assert!(!msg.is_empty());
    }

    #[tokio::test]
    async fn expired_token_is_rejected() {
        // 0-minute TTL ensures immediate expiration
        let keys = make_keys_with("s", "iss", "aud", 0, 0);
        let token = keys.sign_access(Uuid::new_v4()).expect("sign");
        // Small sleep is defensive; usually not required
        let res = keys.verify(&token);
        assert!(res.is_err(), "expired token should be rejected");
    }
}
