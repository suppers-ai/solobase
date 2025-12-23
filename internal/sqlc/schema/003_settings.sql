-- System settings table

CREATE TABLE IF NOT EXISTS sys_settings (
    id TEXT PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    value TEXT,
    type TEXT DEFAULT 'string',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at DATETIME
);

CREATE INDEX IF NOT EXISTS idx_sys_settings_deleted_at ON sys_settings(deleted_at);
