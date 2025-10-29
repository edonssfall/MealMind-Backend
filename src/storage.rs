use anyhow::Context;
use async_trait::async_trait;
use aws_sdk_s3::{config::{Credentials, Region}, Client};
use bytes::Bytes;

#[async_trait]
pub trait StorageClient: Send + Sync {
    async fn put_object(&self, key: &str, body: Bytes, content_type: &str) -> anyhow::Result<()>;
    async fn delete_object(&self, key: &str) -> anyhow::Result<()>;
    async fn presign_get(&self, key: &str, seconds: u64) -> anyhow::Result<String>;
}

#[derive(Clone)]
pub struct Storage {
    client: Client,
    bucket: String,
}

impl Storage {
    pub async fn new(
        endpoint: &str,
        bucket: &str,
        access_key: &str,
        secret_key: &str,
        region: &str,
    ) -> anyhow::Result<Self> {
        let region = Region::new(region.to_string());
        let creds = Credentials::new(access_key, secret_key, None, None, "static");
        let conf = aws_config::from_env()
            .region(region)
            .credentials_provider(creds)
            .endpoint_url(endpoint)
            .load()
            .await;
        Ok(Self {
            client: Client::new(&conf),
            bucket: bucket.to_string(),
        })
    }
}

#[async_trait]
impl StorageClient for Storage {
    async fn put_object(&self, key: &str, body: Bytes, content_type: &str) -> anyhow::Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body.into())
            .content_type(content_type)
            .send()
            .await
            .context("s3 put_object")?;
        Ok(())
    }

    async fn delete_object(&self, key: &str) -> anyhow::Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .context("s3 delete_object")?;
        Ok(())
    }

    async fn presign_get(&self, key: &str, seconds: u64) -> anyhow::Result<String> {
        use aws_sdk_s3::presigning::PresigningConfig;
        let req = self.client.get_object().bucket(&self.bucket).key(key);
        let presigned = req
            .presigned(PresigningConfig::expires_in(
                std::time::Duration::from_secs(seconds),
            )?)
            .await
            .context("s3 presign")?;
        Ok(presigned.uri().to_string())
    }
}
