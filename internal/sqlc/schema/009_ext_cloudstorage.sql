-- Cloud storage extension tables

CREATE TABLE IF NOT EXISTS ext_cloudstorage_storage_shares (
    id TEXT PRIMARY KEY,
    object_id TEXT NOT NULL,
    shared_with_user_id TEXT,
    shared_with_email TEXT,
    permission_level TEXT NOT NULL DEFAULT 'view',
    inherit_to_children INTEGER DEFAULT 1 NOT NULL,
    share_token TEXT UNIQUE,
    is_public INTEGER DEFAULT 0 NOT NULL,
    expires_at DATETIME,
    created_by TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_ext_cloudstorage_storage_shares_object_id ON ext_cloudstorage_storage_shares(object_id);
CREATE INDEX IF NOT EXISTS idx_ext_cloudstorage_storage_shares_shared_with_user_id ON ext_cloudstorage_storage_shares(shared_with_user_id);

CREATE TABLE IF NOT EXISTS ext_cloudstorage_storage_access_logs (
    id TEXT PRIMARY KEY,
    object_id TEXT NOT NULL,
    user_id TEXT,
    ip_address TEXT,
    action TEXT NOT NULL,
    user_agent TEXT,
    metadata TEXT DEFAULT '{}',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_ext_cloudstorage_storage_access_logs_object_id ON ext_cloudstorage_storage_access_logs(object_id);
CREATE INDEX IF NOT EXISTS idx_ext_cloudstorage_storage_access_logs_user_id ON ext_cloudstorage_storage_access_logs(user_id);

CREATE TABLE IF NOT EXISTS ext_cloudstorage_storage_quotas (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL UNIQUE,
    max_storage_bytes INTEGER NOT NULL DEFAULT 5368709120,
    max_bandwidth_bytes INTEGER NOT NULL DEFAULT 10737418240,
    storage_used INTEGER NOT NULL DEFAULT 0,
    bandwidth_used INTEGER NOT NULL DEFAULT 0,
    reset_bandwidth_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS ext_cloudstorage_role_quotas (
    id TEXT PRIMARY KEY,
    role_id TEXT NOT NULL UNIQUE,
    role_name TEXT NOT NULL,
    max_storage_bytes INTEGER NOT NULL DEFAULT 5368709120,
    max_bandwidth_bytes INTEGER NOT NULL DEFAULT 10737418240,
    max_upload_size INTEGER NOT NULL DEFAULT 104857600,
    max_files_count INTEGER NOT NULL DEFAULT 1000,
    allowed_extensions TEXT,
    blocked_extensions TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_ext_cloudstorage_role_quotas_role_name ON ext_cloudstorage_role_quotas(role_name);

CREATE TABLE IF NOT EXISTS ext_cloudstorage_user_quota_overrides (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL UNIQUE,
    max_storage_bytes INTEGER,
    max_bandwidth_bytes INTEGER,
    max_upload_size INTEGER,
    max_files_count INTEGER,
    allowed_extensions TEXT,
    blocked_extensions TEXT,
    reason TEXT,
    expires_at DATETIME,
    created_by TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_ext_cloudstorage_user_quota_overrides_expires_at ON ext_cloudstorage_user_quota_overrides(expires_at);
CREATE INDEX IF NOT EXISTS idx_ext_cloudstorage_user_quota_overrides_created_by ON ext_cloudstorage_user_quota_overrides(created_by);
