use anyhow::Context;
use aws_config::{defaults, BehaviorVersion};
use aws_credential_types::Credentials;
use aws_sdk_s3::{
    config::{Builder as S3ConfigBuilder, Region},
    presigning::PresigningConfig,
    Client,
};
use aws_smithy_types::byte_stream::ByteStream;
use axum::async_trait;
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
        let shared = defaults(BehaviorVersion::latest())
            .region(Region::new(region.to_string()))
            .credentials_provider(Credentials::new(
                access_key, secret_key, None, None, "static",
            ))
            .endpoint_url(endpoint)
            .load()
            .await;

        let conf = S3ConfigBuilder::from(&shared)
            .endpoint_url(endpoint)
            .force_path_style(true)
            .build();

        Ok(Self {
            client: Client::from_conf(conf),
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
            .body(ByteStream::from(body))
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
        let req = self.client.get_object().bucket(&self.bucket).key(key);
        let presigned = req
            .presigned(PresigningConfig::expires_in(
                std::time::Duration::from_secs(seconds),
            )?)
            .await
            .context("s3 presign_get")?;
        Ok(presigned.uri().to_string())
    }
}
