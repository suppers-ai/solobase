//! Platform D1 database schema — subscriptions and usage tracking only.
//!
//! Project-specific tables (auth, products, files, etc.) live in each
//! project's own D1 database, managed by the user worker's schema.

use worker::*;

/// Platform-only SQL migrations.
const MIGRATIONS: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS subscriptions (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL UNIQUE,
        stripe_customer_id TEXT DEFAULT '',
        stripe_subscription_id TEXT DEFAULT '',
        plan TEXT DEFAULT 'free',
        status TEXT DEFAULT 'active',
        grace_period_end TEXT,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_subscriptions_user ON subscriptions (user_id)",
    "CREATE INDEX IF NOT EXISTS idx_subscriptions_stripe ON subscriptions (stripe_subscription_id)",

    "CREATE TABLE IF NOT EXISTS project_usage (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL,
        month TEXT NOT NULL,
        requests INTEGER DEFAULT 0,
        r2_bytes INTEGER DEFAULT 0,
        addon_requests INTEGER DEFAULT 0,
        addon_r2_bytes INTEGER DEFAULT 0,
        addon_d1_bytes INTEGER DEFAULT 0,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        UNIQUE(project_id, month)
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_project_usage_pm ON project_usage (project_id, month)",

    "CREATE TABLE IF NOT EXISTS projects (
        id TEXT PRIMARY KEY,
        subdomain TEXT NOT NULL UNIQUE,
        name TEXT DEFAULT '',
        plan TEXT DEFAULT 'free',
        status TEXT DEFAULT 'active',
        owner_user_id TEXT DEFAULT '',
        db_id TEXT DEFAULT '',
        platform INTEGER DEFAULT 0,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_projects_subdomain ON projects (subdomain)",
];

/// Run platform schema migrations on the platform D1 database.
pub async fn run_migrations(db: &D1Database) -> Result<()> {
    for sql in MIGRATIONS {
        db.prepare(*sql).bind(&[])?.run().await?;
    }
    console_log!("Platform migrations applied ({} statements)", MIGRATIONS.len());
    Ok(())
}
