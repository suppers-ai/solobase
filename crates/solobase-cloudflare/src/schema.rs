//! D1 database schema definitions and migration.
//!
//! Each tenant has their own D1 database, so tables do not need a tenant_id
//! column — all data in the database belongs to that single tenant.

use worker::*;

/// SQL statements to create the standard Solobase tables.
const MIGRATIONS: &[&str] = &[
    // Users table
    "CREATE TABLE IF NOT EXISTS users (
        id TEXT PRIMARY KEY,
        email TEXT NOT NULL UNIQUE,
        password_hash TEXT NOT NULL,
        name TEXT DEFAULT '',
        role TEXT DEFAULT 'user',
        avatar_url TEXT DEFAULT '',
        metadata TEXT DEFAULT '{}',
        email_verified INTEGER DEFAULT 0,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        deleted_at TEXT
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email ON users (email)",

    // Sessions table (for refresh tokens)
    "CREATE TABLE IF NOT EXISTS sessions (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        token_hash TEXT NOT NULL,
        expires_at TEXT NOT NULL,
        created_at TEXT DEFAULT (datetime('now')),
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
    )",
    "CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions (user_id)",

    // Products table
    "CREATE TABLE IF NOT EXISTS products (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT DEFAULT '',
        slug TEXT DEFAULT '',
        price REAL DEFAULT 0,
        currency TEXT DEFAULT 'USD',
        status TEXT DEFAULT 'draft',
        category TEXT DEFAULT '',
        tags TEXT DEFAULT '[]',
        metadata TEXT DEFAULT '{}',
        image_url TEXT DEFAULT '',
        stock INTEGER DEFAULT 0,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        deleted_at TEXT
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_products_slug ON products (slug)",

    // Files metadata table
    "CREATE TABLE IF NOT EXISTS files (
        id TEXT PRIMARY KEY,
        folder TEXT NOT NULL,
        key TEXT NOT NULL,
        filename TEXT NOT NULL,
        content_type TEXT DEFAULT 'application/octet-stream',
        size INTEGER DEFAULT 0,
        public INTEGER DEFAULT 0,
        uploaded_by TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        deleted_at TEXT
    )",
    "CREATE INDEX IF NOT EXISTS idx_files_folder ON files (folder)",

    // Legal pages table
    "CREATE TABLE IF NOT EXISTS legal_pages (
        id TEXT PRIMARY KEY,
        slug TEXT NOT NULL UNIQUE,
        title TEXT NOT NULL,
        content TEXT DEFAULT '',
        published INTEGER DEFAULT 0,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_legalpages_slug ON legal_pages (slug)",

    // Settings / config table
    "CREATE TABLE IF NOT EXISTS settings (
        id TEXT PRIMARY KEY,
        key TEXT NOT NULL UNIQUE,
        value TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_settings_key ON settings (key)",

    // Monitoring / audit log table
    "CREATE TABLE IF NOT EXISTS audit_log (
        id TEXT PRIMARY KEY,
        user_id TEXT DEFAULT '',
        action TEXT NOT NULL,
        resource TEXT DEFAULT '',
        details TEXT DEFAULT '{}',
        ip_address TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE INDEX IF NOT EXISTS idx_audit_created ON audit_log (created_at)",

    // User profiles table (public profile data)
    "CREATE TABLE IF NOT EXISTS profiles (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL UNIQUE,
        display_name TEXT DEFAULT '',
        bio TEXT DEFAULT '',
        website TEXT DEFAULT '',
        avatar_url TEXT DEFAULT '',
        social_links TEXT DEFAULT '{}',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_profiles_user ON profiles (user_id)",
];

/// Run all schema migrations on a D1 database.
pub async fn run_migrations(db: &D1Database) -> Result<()> {
    for sql in MIGRATIONS {
        db.prepare(*sql).bind(&[])?.run().await?;
    }
    console_log!("Schema migrations applied ({} statements)", MIGRATIONS.len());
    Ok(())
}
