-- Auth tables for users, tokens, and API keys

CREATE TABLE IF NOT EXISTS auth_users (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL,
    username TEXT,
    confirmed INTEGER DEFAULT 0,
    first_name TEXT,
    last_name TEXT,
    display_name TEXT,
    phone TEXT,
    location TEXT,
    confirm_token TEXT,
    confirm_selector TEXT,
    recover_token TEXT,
    recover_token_exp DATETIME,
    recover_selector TEXT,
    attempt_count INTEGER DEFAULT 0,
    last_attempt DATETIME,
    last_login DATETIME,
    metadata TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at DATETIME,
    -- 2FA fields
    totp_secret TEXT,
    totp_secret_backup TEXT,
    sms_phone_number TEXT,
    recovery_codes TEXT
);

CREATE INDEX IF NOT EXISTS idx_auth_users_confirm_selector ON auth_users(confirm_selector);
CREATE INDEX IF NOT EXISTS idx_auth_users_recover_selector ON auth_users(recover_selector);
CREATE INDEX IF NOT EXISTS idx_auth_users_deleted_at ON auth_users(deleted_at);

CREATE TABLE IF NOT EXISTS auth_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    token_hash TEXT,
    token TEXT,
    type TEXT NOT NULL,
    family_id TEXT,
    -- OAuth fields
    provider TEXT,
    provider_uid TEXT,
    access_token TEXT,
    oauth_expiry DATETIME,
    -- Lifecycle
    expires_at DATETIME NOT NULL,
    used_at DATETIME,
    revoked_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- Audit fields
    device_info TEXT,
    ip_address TEXT,
    FOREIGN KEY (user_id) REFERENCES auth_users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_auth_tokens_user_id ON auth_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_auth_tokens_token_hash ON auth_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_auth_tokens_token ON auth_tokens(token);
CREATE INDEX IF NOT EXISTS idx_auth_tokens_type ON auth_tokens(type);
CREATE INDEX IF NOT EXISTS idx_auth_tokens_family_id ON auth_tokens(family_id);
CREATE INDEX IF NOT EXISTS idx_auth_tokens_provider_uid ON auth_tokens(provider_uid);
CREATE INDEX IF NOT EXISTS idx_auth_tokens_expires_at ON auth_tokens(expires_at);
CREATE INDEX IF NOT EXISTS idx_auth_tokens_revoked_at ON auth_tokens(revoked_at);

CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    key_prefix TEXT NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,
    scopes TEXT,
    expires_at DATETIME,
    last_used_at DATETIME,
    last_used_ip TEXT,
    revoked_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES auth_users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_key_prefix ON api_keys(key_prefix);
CREATE INDEX IF NOT EXISTS idx_api_keys_revoked_at ON api_keys(revoked_at);
