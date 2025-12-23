-- Solobase Initial Schema for D1

-- Auth users table
CREATE TABLE IF NOT EXISTS auth_users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    password TEXT NOT NULL,
    username TEXT,
    confirmed INTEGER DEFAULT 0,
    first_name TEXT,
    last_name TEXT,
    display_name TEXT,
    phone TEXT,
    location TEXT,
    metadata TEXT,
    last_login TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT
);

-- Auth tokens table
CREATE TABLE IF NOT EXISTS auth_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    type TEXT NOT NULL,
    provider TEXT,
    provider_uid TEXT,
    access_token TEXT,
    refresh_token TEXT,
    expires_at TEXT,
    oauth_expiry TEXT,
    revoked_at TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES auth_users(id)
);

-- Settings table
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT,
    updated_at TEXT
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_auth_users_email ON auth_users(email);
CREATE INDEX IF NOT EXISTS idx_auth_tokens_user_id ON auth_tokens(user_id);
