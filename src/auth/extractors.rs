use crate::auth::dto::{JwtKeys, TokenKind};
use axum::extract::FromRef;
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use uuid::Uuid;

/// Extracts and validates JWT, returning the user ID.
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
            Err(_) => {
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
