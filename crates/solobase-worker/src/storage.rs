//! Async storage service backed by Cloudflare R2.
//!
//! Implements the shared `StorageService` trait from wafer-core so R2Block
//! can reuse the shared message handler.

use worker::*;

use wafer_core::interfaces::storage::service::{
    FolderInfo, ListOptions, ObjectInfo, ObjectList, StorageError, StorageService,
};

/// Async storage service wrapping Cloudflare R2.
pub struct R2StorageService {
    bucket: Bucket,
    tenant_id: String,
}

// Safety: wasm32-unknown-unknown is single-threaded.
unsafe impl Send for R2StorageService {}
unsafe impl Sync for R2StorageService {}

impl R2StorageService {
    pub fn new(bucket: Bucket, tenant_id: String) -> Self {
        Self { bucket, tenant_id }
    }

    fn prefixed_key(&self, folder: &str, key: &str) -> String {
        format!("{}/{}/{}", self.tenant_id, folder, key)
    }

    fn folder_prefix(&self, folder: &str) -> String {
        format!("{}/{}/", self.tenant_id, folder)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl StorageService for R2StorageService {
    async fn put(
        &self,
        folder: &str,
        key: &str,
        data: &[u8],
        content_type: &str,
    ) -> Result<(), StorageError> {
        let r2_key = self.prefixed_key(folder, key);
        self.bucket
            .put(&r2_key, data.to_vec())
            .http_metadata(HttpMetadata {
                content_type: Some(content_type.to_string()),
                ..Default::default()
            })
            .execute()
            .await
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn get(&self, folder: &str, key: &str) -> Result<(Vec<u8>, ObjectInfo), StorageError> {
        let r2_key = self.prefixed_key(folder, key);
        let obj = self
            .bucket
            .get(&r2_key)
            .execute()
            .await
            .map_err(|e| StorageError::Internal(e.to_string()))?
            .ok_or(StorageError::NotFound)?;

        let body = obj
            .body()
            .ok_or_else(|| StorageError::Internal("no body".into()))?;
        let bytes = body
            .bytes()
            .await
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let info = ObjectInfo {
            key: key.to_string(),
            size: bytes.len() as i64,
            content_type: obj
                .http_metadata()
                .content_type
                .unwrap_or_else(|| "application/octet-stream".to_string()),
            last_modified: chrono::Utc::now(), // R2 doesn't expose last_modified easily via chrono
        };

        Ok((bytes, info))
    }

    async fn delete(&self, folder: &str, key: &str) -> Result<(), StorageError> {
        let r2_key = self.prefixed_key(folder, key);
        self.bucket
            .delete(&r2_key)
            .await
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn list(&self, folder: &str, opts: &ListOptions) -> Result<ObjectList, StorageError> {
        let full_prefix = if opts.prefix.is_empty() {
            self.folder_prefix(folder)
        } else {
            format!("{}{}", self.folder_prefix(folder), opts.prefix)
        };

        let limit = if opts.limit > 0 { opts.limit as u32 } else { 100 };

        let listed = self
            .bucket
            .list()
            .prefix(&full_prefix)
            .limit(limit)
            .execute()
            .await
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let folder_prefix_len = self.folder_prefix(folder).len();

        let objects: Vec<ObjectInfo> = listed
            .objects()
            .iter()
            .map(|obj| {
                let full_key = obj.key();
                let key = if full_key.len() > folder_prefix_len {
                    full_key[folder_prefix_len..].to_string()
                } else {
                    full_key.clone()
                };

                ObjectInfo {
                    key,
                    size: obj.size() as i64,
                    content_type: "application/octet-stream".to_string(),
                    last_modified: chrono::Utc::now(),
                }
            })
            .collect();

        let count = objects.len() as i64;
        Ok(ObjectList {
            objects,
            total_count: count,
        })
    }

    async fn create_folder(&self, _name: &str, _public: bool) -> Result<(), StorageError> {
        // R2 doesn't need explicit folder creation — objects create the path
        Ok(())
    }

    async fn delete_folder(&self, _name: &str) -> Result<(), StorageError> {
        // Would need to list + batch delete; no-op for now
        Ok(())
    }

    async fn list_folders(&self) -> Result<Vec<FolderInfo>, StorageError> {
        // R2 doesn't have a native folder concept
        Ok(Vec::new())
    }
}
