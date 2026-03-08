//! D1 database schema definitions and migration.
//!
//! Defines the standard tables used by Solobase. Tables include a `tenant_id`
//! column for multi-tenant data isolation within a single D1 database.

use worker::*;

/// SQL statements to create the standard Solobase tables.
const MIGRATIONS: &[&str] = &[
    // Users table
    "CREATE TABLE IF NOT EXISTS users (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        email TEXT NOT NULL,
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
    "CREATE INDEX IF NOT EXISTS idx_users_tenant ON users (tenant_id)",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email_tenant ON users (tenant_id, email)",

    // Sessions table (for refresh tokens)
    "CREATE TABLE IF NOT EXISTS sessions (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        user_id TEXT NOT NULL,
        token_hash TEXT NOT NULL,
        expires_at TEXT NOT NULL,
        created_at TEXT DEFAULT (datetime('now')),
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
    )",
    "CREATE INDEX IF NOT EXISTS idx_sessions_tenant ON sessions (tenant_id)",
    "CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions (user_id)",

    // Products table
    "CREATE TABLE IF NOT EXISTS products (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
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
    "CREATE INDEX IF NOT EXISTS idx_products_tenant ON products (tenant_id)",
    "CREATE INDEX IF NOT EXISTS idx_products_slug ON products (tenant_id, slug)",

    // Files metadata table
    "CREATE TABLE IF NOT EXISTS files (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
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
    "CREATE INDEX IF NOT EXISTS idx_files_tenant ON files (tenant_id)",
    "CREATE INDEX IF NOT EXISTS idx_files_folder ON files (tenant_id, folder)",

    // Legal pages table
    "CREATE TABLE IF NOT EXISTS legal_pages (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        slug TEXT NOT NULL,
        title TEXT NOT NULL,
        content TEXT DEFAULT '',
        published INTEGER DEFAULT 0,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE INDEX IF NOT EXISTS idx_legalpages_tenant ON legal_pages (tenant_id)",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_legalpages_slug_tenant ON legal_pages (tenant_id, slug)",

    // Settings / config table
    "CREATE TABLE IF NOT EXISTS settings (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        key TEXT NOT NULL,
        value TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE INDEX IF NOT EXISTS idx_settings_tenant ON settings (tenant_id)",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_settings_key_tenant ON settings (tenant_id, key)",

    // Monitoring / audit log table
    "CREATE TABLE IF NOT EXISTS audit_log (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        user_id TEXT DEFAULT '',
        action TEXT NOT NULL,
        resource TEXT DEFAULT '',
        details TEXT DEFAULT '{}',
        ip_address TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE INDEX IF NOT EXISTS idx_audit_tenant ON audit_log (tenant_id)",
    "CREATE INDEX IF NOT EXISTS idx_audit_created ON audit_log (created_at)",

    // User profiles table (public profile data)
    "CREATE TABLE IF NOT EXISTS profiles (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        user_id TEXT NOT NULL,
        display_name TEXT DEFAULT '',
        bio TEXT DEFAULT '',
        website TEXT DEFAULT '',
        avatar_url TEXT DEFAULT '',
        social_links TEXT DEFAULT '{}',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
    )",
    "CREATE INDEX IF NOT EXISTS idx_profiles_tenant ON profiles (tenant_id)",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_profiles_user ON profiles (tenant_id, user_id)",
];

/// Run all schema migrations on a D1 database.
pub async fn run_migrations(db: &D1Database) -> Result<()> {
    for sql in MIGRATIONS {
        db.prepare(*sql).bind(&[])?.run().await?;
    }
    console_log!("Schema migrations applied ({} statements)", MIGRATIONS.len());
    Ok(())
}
