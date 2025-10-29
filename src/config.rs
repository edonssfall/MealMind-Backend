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

    pub minio_endpoint: String,
    pub minio_bucket: String,
    pub minio_access_key: String,
    pub minio_secret_key: String,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let minio_endpoint =
            std::env::var("MINIO_ENDPOINT").unwrap_or_else(|_| "http://127.0.0.1:9000".into());
        let minio_bucket = std::env::var("MINIO_BUCKET").unwrap_or_else(|_| "mealmind".into());
        let minio_access_key =
            std::env::var("MINIO_ACCESS_KEY").unwrap_or_else(|_| "minioadmin".into());
        let minio_secret_key =
            std::env::var("MINIO_SECRET_KEY").unwrap_or_else(|_| "minioadmin".into());

        let jwt = JwtConfig {
            secret: std::env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
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

        Ok(Self {
            database_url,
            jwt,
            minio_endpoint,
            minio_bucket,
            minio_access_key,
            minio_secret_key,
        })
    }
}
