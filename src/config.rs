use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub issuer: String,
    pub audience: String,
    pub ttl_minutes: i64,
    pub refresh_ttl_minutes: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt: JwtConfig,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let database_url = std::env::var("DATABASE_URL")?;
        let jwt = JwtConfig {
            secret: std::env::var("JWT_SECRET")?,
            issuer: std::env::var("JWT_ISSUER").unwrap_or_else(|_| "mealmind".into()),
            audience: std::env::var("JWT_AUDIENCE").unwrap_or_else(|_| "mealmind-users".into()),
            ttl_minutes: std::env::var("JWT_TTL_MINUTES")
                .ok()
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(60),
            refresh_ttl_minutes: std::env::var("JWT_REFRESH_TTL_MINUTES")
                .ok()
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(60 * 24 * 14),
        };
        Ok(Self { database_url, jwt })
    }
}
