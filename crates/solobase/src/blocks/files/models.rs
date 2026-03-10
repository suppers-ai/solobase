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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_quota() {
        let quota = QuotaConfig::default();
        assert_eq!(quota.max_storage_bytes, 1_073_741_824); // 1GB
        assert_eq!(quota.max_file_size_bytes, 104_857_600);  // 100MB
        assert_eq!(quota.max_files_per_bucket, 10_000);
        assert_eq!(quota.reset_period_days, 0);
    }

    #[test]
    fn test_quota_serialization() {
        let quota = QuotaConfig {
            max_storage_bytes: 500_000,
            max_file_size_bytes: 10_000,
            max_files_per_bucket: 100,
            reset_period_days: 30,
        };
        let json = serde_json::to_string(&quota).unwrap();
        let deserialized: QuotaConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.max_storage_bytes, 500_000);
        assert_eq!(deserialized.max_file_size_bytes, 10_000);
        assert_eq!(deserialized.max_files_per_bucket, 100);
        assert_eq!(deserialized.reset_period_days, 30);
    }
}
