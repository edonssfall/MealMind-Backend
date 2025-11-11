use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of JWT: access or refresh.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TokenKind {
    #[serde(alias = "Access")]
    Access,
    #[serde(alias = "Refresh")]
    Refresh,
}

/// JWT payload used for authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,      // user ID
    pub iat: usize,     // issued at (unix timestamp)
    pub exp: usize,     // expires at (unix timestamp)
    pub iss: String,    // issuer
    pub aud: String,    // audience
    pub kind: TokenKind // token type
}
