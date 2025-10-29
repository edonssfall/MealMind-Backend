use std::sync::Arc;
use sqlx::PgPool;
use crate::config::AppConfig;
use crate::storage::{Storage, StorageClient}; // ← импорт трейта

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<AppConfig>,
    pub storage: Arc<dyn StorageClient>, // ← главное изменение
}

impl AppState {
    pub async fn init() -> anyhow::Result<Self> {
        let config = Arc::new(AppConfig::from_env()?);

        let db = sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .connect(&config.database_url)
            .await?;

        // Реальный S3/MinIO
        let storage = Arc::new(
            Storage::new(
                &config.minio_endpoint,
                &config.minio_bucket,
                &config.minio_access_key,
                &config.minio_secret_key,
                "us-east-1",
            ).await?
        ) as Arc<dyn StorageClient>; // ← привести к трейту

        Ok(Self { db, config, storage })
    }

    pub fn from_parts(
        db: PgPool,
        config: Arc<AppConfig>,
        storage: Arc<dyn StorageClient>, // ← сигнатура тоже меняется
    ) -> Self {
        Self { db, config, storage }
    }

    pub fn fake() -> Self {
        use bytes::Bytes;
        use async_trait::async_trait;

        #[derive(Clone)]
        struct FakeStorage;
        #[async_trait]
        impl StorageClient for FakeStorage {
            async fn put_object(&self, _k: &str, _b: Bytes, _ct: &str) -> anyhow::Result<()> { Ok(()) }
            async fn delete_object(&self, _k: &str) -> anyhow::Result<()> { Ok(()) }
            async fn presign_get(&self, k: &str, _s: u64) -> anyhow::Result<String> {
                Ok(format!("https://fake.local/{}", k))
            }
        }

        let db = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost:5432/postgres")
            .expect("lazy pool ok");

        let config = Arc::new(AppConfig {
            database_url: "postgres://postgres:postgres@localhost:5432/postgres".into(),
            jwt: crate::config::JwtConfig {
                secret: "test".into(), issuer: "test".into(), audience: "test".into(),
                ttl_minutes: 5, refresh_ttl_minutes: 60,
            },
            minio_endpoint: "fake".into(),
            minio_bucket: "fake".into(),
            minio_access_key: "fake".into(),
            minio_secret_key: "fake".into(),
        });

        let storage = Arc::new(FakeStorage) as Arc<dyn StorageClient>;
        Self { db, config, storage }
    }
}
