use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

/// User record in the database.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,                     // unique user ID
    pub email: String,                // user email
    #[serde(skip_serializing)]
    pub password_hash: String,        // Argon2 hash, not exposed in JSON
    pub created_at: OffsetDateTime,   // creation timestamp
}