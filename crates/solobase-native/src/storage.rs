//! Storage platform-service factories for native targets.

use std::sync::Arc;

use wafer_core::interfaces::storage::service::StorageService;

/// Initialise a local-filesystem storage service rooted at `root`.
pub fn make_local_storage_service(root: &str) -> Arc<dyn StorageService> {
    let svc = wafer_block_local_storage::service::LocalStorageService::new(root)
        .unwrap_or_else(|e| panic!("failed to init local storage at {root}: {e}"));
    Arc::new(svc)
}

/// Configuration for an S3-compatible storage backend.
#[cfg(feature = "s3")]
pub struct S3Config {
    pub bucket: String,
    pub prefix: String,
    /// Optional custom endpoint URL (for MinIO, Tigris, Cloudflare R2, etc.).
    pub endpoint: Option<String>,
    /// AWS region; defaults to `us-east-1` when using a custom endpoint.
    pub region: Option<String>,
}

/// Construct an S3-backed storage service. Feature-gated.
///
/// Uses `S3StorageService::with_endpoint` when `config.endpoint` is set,
/// otherwise falls back to default AWS config (env vars / IAM role).
#[cfg(feature = "s3")]
pub async fn make_s3_storage_service(config: S3Config) -> Arc<dyn StorageService> {
    use wafer_block_s3::service::S3StorageService;

    let svc = match config.endpoint {
        Some(ref endpoint) => {
            let region = config.region.as_deref().unwrap_or("us-east-1");
            S3StorageService::with_endpoint(&config.bucket, &config.prefix, endpoint, region).await
        }
        None => S3StorageService::new(&config.bucket, &config.prefix).await,
    }
    .unwrap_or_else(|e| panic!("failed to init S3 storage: {e}"));

    Arc::new(svc)
}
