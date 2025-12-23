-- Storage tables for buckets, objects, and tokens

CREATE TABLE IF NOT EXISTS storage_buckets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    public INTEGER DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS storage_objects (
    id TEXT PRIMARY KEY,
    bucket_name TEXT NOT NULL,
    object_name TEXT NOT NULL,
    parent_folder_id TEXT,
    size INTEGER,
    content_type TEXT,
    checksum TEXT,
    metadata TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_viewed DATETIME,
    user_id TEXT,
    app_id TEXT
);

CREATE INDEX IF NOT EXISTS idx_storage_objects_bucket_name ON storage_objects(bucket_name);
CREATE INDEX IF NOT EXISTS idx_storage_objects_object_name ON storage_objects(object_name);
CREATE INDEX IF NOT EXISTS idx_storage_objects_parent_folder_id ON storage_objects(parent_folder_id);
CREATE INDEX IF NOT EXISTS idx_storage_objects_checksum ON storage_objects(checksum);
CREATE INDEX IF NOT EXISTS idx_storage_objects_last_viewed ON storage_objects(last_viewed);
CREATE INDEX IF NOT EXISTS idx_storage_objects_user_id ON storage_objects(user_id);
CREATE INDEX IF NOT EXISTS idx_storage_objects_app_id ON storage_objects(app_id);

CREATE TABLE IF NOT EXISTS storage_upload_tokens (
    id TEXT PRIMARY KEY,
    token TEXT NOT NULL UNIQUE,
    bucket TEXT NOT NULL,
    parent_folder_id TEXT,
    object_name TEXT NOT NULL,
    user_id TEXT,
    max_size INTEGER,
    content_type TEXT,
    bytes_uploaded INTEGER DEFAULT 0,
    completed INTEGER DEFAULT 0,
    object_id TEXT,
    expires_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    client_ip TEXT
);

CREATE TABLE IF NOT EXISTS storage_download_tokens (
    id TEXT PRIMARY KEY,
    token TEXT NOT NULL UNIQUE,
    file_id TEXT NOT NULL,
    bucket TEXT NOT NULL,
    parent_folder_id TEXT,
    object_name TEXT NOT NULL,
    user_id TEXT,
    file_size INTEGER,
    bytes_served INTEGER DEFAULT 0,
    completed INTEGER DEFAULT 0,
    expires_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    callback_at DATETIME,
    client_ip TEXT
);
