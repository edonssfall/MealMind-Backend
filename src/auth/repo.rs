use crate::auth::repo_types::User;
use sqlx::PgPool;

impl User {
    /// Find a user by email.
    pub async fn find_by_email(db: &PgPool, email: &str) -> anyhow::Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, password_hash, created_at
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(db)
        .await?;
        Ok(user)
    }

    /// Create a new user with hashed password.
    pub async fn create(db: &PgPool, email: &str, password_hash: &str) -> anyhow::Result<User> {
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (email, password_hash)
            VALUES ($1, $2)
            RETURNING id, email, password_hash, created_at
            "#,
        )
        .bind(email)
        .bind(password_hash)
        .fetch_one(db)
        .await?;
        Ok(user)
    }
}
