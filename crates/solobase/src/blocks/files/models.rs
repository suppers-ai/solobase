use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaConfig {
    pub max_storage_bytes: i64,
    pub max_file_size_bytes: i64,
    pub max_files_per_bucket: i64,
    pub reset_period_days: i64,
}

impl Default for QuotaConfig {
    fn default() -> Self {
        Self {
            max_storage_bytes: 1_073_741_824, // 1GB
            max_file_size_bytes: 104_857_600,  // 100MB
            max_files_per_bucket: 10_000,
            reset_period_days: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareToken {
    pub id: String,
    pub bucket: String,
    pub key: String,
    pub created_by: String,
    pub expires_at: Option<String>,
    pub access_count: i64,
    pub max_access_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessLogEntry {
    pub share_id: String,
    pub accessed_at: String,
    pub ip_address: String,
    pub user_agent: String,
}
