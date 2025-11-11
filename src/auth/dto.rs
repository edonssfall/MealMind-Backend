use jsonwebtoken::{DecodingKey, EncodingKey};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

/// Token type used to distinguish Access and Refresh JWTs.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TokenKind {
    #[serde(alias = "Access")]
    Access,
    #[serde(alias = "Refresh")]
    Refresh,
}

/// Standard JWT claims used in the app.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: Uuid,       // user ID
    pub exp: usize,      // expiration time
    pub iat: usize,      // issued at
    pub iss: String,     // issuer
    pub aud: String,     // audience
    pub kind: TokenKind, // access or refresh
}

/// Holds JWT signing and verification keys with config data.
#[derive(Clone)]
pub struct JwtKeys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
    pub issuer: String,
    pub audience: String,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
}

/// Request body for user registration.
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

/// Request body for login.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Request body for token refresh.
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Response returned after login, register or refresh.
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user: PublicUser,
}

/// Public part of the user returned to the client.
#[derive(Debug, Serialize)]
pub struct PublicUser {
    pub id: Uuid,
    pub email: String,
}
