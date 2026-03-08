//! Async storage service backed by Cloudflare R2.
//!
//! Files are stored in R2 with tenant-scoped key prefixes:
//! `{tenant_id}/{folder}/{key}`

use serde::{Deserialize, Serialize};
use worker::*;

/// Object metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInfo {
    pub key: String,
    pub size: i64,
    pub content_type: String,
    pub last_modified: String,
}

/// Paginated object listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectList {
    pub objects: Vec<ObjectInfo>,
    pub total_count: i64,
}

/// Folder metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderInfo {
    pub name: String,
    pub public: bool,
}

/// Async storage service wrapping Cloudflare R2.
pub struct R2StorageService {
    bucket: Bucket,
    tenant_id: String,
}

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

    /// Store an object in a folder.
    pub async fn put(
        &self,
        folder: &str,
        key: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> Result<()> {
        let r2_key = self.prefixed_key(folder, key);
        self.bucket
            .put(&r2_key, data)
            .http_metadata(HttpMetadata {
                content_type: Some(content_type.to_string()),
                ..Default::default()
            })
            .execute()
            .await?;
        Ok(())
    }

    /// Get an object and its metadata.
    pub async fn get(&self, folder: &str, key: &str) -> Result<(Vec<u8>, ObjectInfo)> {
        let r2_key = self.prefixed_key(folder, key);
        let obj = self
            .bucket
            .get(&r2_key)
            .execute()
            .await?
            .ok_or_else(|| Error::RustError("object not found".into()))?;

        let body = obj.body().ok_or_else(|| Error::RustError("no body".into()))?;
        let bytes = body.bytes().await?;

        let info = ObjectInfo {
            key: key.to_string(),
            size: bytes.len() as i64,
            content_type: obj
                .http_metadata()
                .content_type
                .unwrap_or_else(|| "application/octet-stream".to_string()),
            last_modified: obj.uploaded().to_string(),
        };

        Ok((bytes, info))
    }

    /// Delete an object from a folder.
    pub async fn delete(&self, folder: &str, key: &str) -> Result<()> {
        let r2_key = self.prefixed_key(folder, key);
        self.bucket.delete(&r2_key).await?;
        Ok(())
    }

    /// List objects in a folder.
    pub async fn list(
        &self,
        folder: &str,
        prefix: &str,
        limit: u32,
    ) -> Result<ObjectList> {
        let full_prefix = if prefix.is_empty() {
            self.folder_prefix(folder)
        } else {
            format!("{}{}", self.folder_prefix(folder), prefix)
        };

        let listed = self
            .bucket
            .list()
            .prefix(&full_prefix)
            .limit(limit)
            .execute()
            .await?;

        let folder_prefix_len = self.folder_prefix(folder).len();

        let objects: Vec<ObjectInfo> = listed
            .objects()
            .iter()
            .map(|obj| {
                let full_key = obj.key();
                // Strip the tenant/folder prefix to get the user-facing key
                let key = if full_key.len() > folder_prefix_len {
                    full_key[folder_prefix_len..].to_string()
                } else {
                    full_key.clone()
                };

                ObjectInfo {
                    key,
                    size: obj.size() as i64,
                    content_type: "application/octet-stream".to_string(),
                    last_modified: obj.uploaded().to_string(),
                }
            })
            .collect();

        let count = objects.len() as i64;
        Ok(ObjectList {
            objects,
            total_count: count,
        })
    }
}
