use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use uuid::Uuid;

use super::claims::Claims;
use crate::state::AppState;

pub struct AuthUser(pub Uuid);

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or((
                StatusCode::UNAUTHORIZED,
                "missing Authorization header".into(),
            ))?;

        let token = auth
            .strip_prefix("Bearer ")
            .ok_or((StatusCode::UNAUTHORIZED, "invalid auth scheme".into()))?;

        let cfg = &state.config.jwt;
        let mut validation = Validation::default();
        validation.set_audience(std::slice::from_ref(&cfg.audience));
        validation.set_issuer(std::slice::from_ref(&cfg.issuer));

        let decoding = DecodingKey::from_secret(cfg.secret.as_bytes());

        let data = decode::<Claims>(token, &decoding, &validation)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid or expired token".into()))?;

        Ok(AuthUser(data.claims.sub))
    }
}
