//! D1 database schema definitions and migration.
//!
//! Each tenant has their own D1 database, so tables do not need a tenant_id
//! column — all data in the database belongs to that single tenant.
//!
//! Table names and columns must match what solobase-core blocks expect.
//! The D1DatabaseService dynamically builds INSERT/SELECT SQL from HashMap
//! keys, so every column referenced in block code must exist here.

use worker::*;

/// SQL statements to create the standard Solobase tables.
///
/// Grouped by block for clarity. All tables use `id TEXT PRIMARY KEY`
/// (the D1DatabaseService generates UUIDs). Timestamps are TEXT (RFC 3339).
const MIGRATIONS: &[&str] = &[
    // =========================================================================
    // AUTH BLOCK — auth_users, auth_tokens, api_keys
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS auth_users (
        id TEXT PRIMARY KEY,
        email TEXT NOT NULL UNIQUE,
        password_hash TEXT NOT NULL,
        name TEXT DEFAULT '',
        disabled INTEGER DEFAULT 0,
        avatar_url TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        deleted_at TEXT
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_auth_users_email ON auth_users (email)",

    "CREATE TABLE IF NOT EXISTS auth_tokens (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        token TEXT NOT NULL,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        FOREIGN KEY (user_id) REFERENCES auth_users(id) ON DELETE CASCADE
    )",
    "CREATE INDEX IF NOT EXISTS idx_auth_tokens_user ON auth_tokens (user_id)",

    "CREATE TABLE IF NOT EXISTS api_keys (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        name TEXT DEFAULT '',
        key_hash TEXT NOT NULL,
        last_used TEXT,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        revoked_at TEXT,
        FOREIGN KEY (user_id) REFERENCES auth_users(id) ON DELETE CASCADE
    )",
    "CREATE INDEX IF NOT EXISTS idx_api_keys_user ON api_keys (user_id)",

    // =========================================================================
    // IAM BLOCK — iam_roles, iam_permissions, iam_user_roles
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS iam_roles (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT DEFAULT '',
        permissions TEXT DEFAULT '[]',
        is_system INTEGER DEFAULT 0,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    "CREATE TABLE IF NOT EXISTS iam_permissions (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        resource TEXT DEFAULT '',
        actions TEXT DEFAULT '[]',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    "CREATE TABLE IF NOT EXISTS iam_user_roles (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        role TEXT NOT NULL,
        assigned_at TEXT DEFAULT (datetime('now')),
        assigned_by TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        FOREIGN KEY (user_id) REFERENCES auth_users(id) ON DELETE CASCADE
    )",
    "CREATE INDEX IF NOT EXISTS idx_iam_user_roles_user ON iam_user_roles (user_id)",

    // =========================================================================
    // ADMIN BLOCK — settings, audit_logs
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS settings (
        id TEXT PRIMARY KEY,
        key TEXT NOT NULL UNIQUE,
        value TEXT DEFAULT '',
        updated_by TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_settings_key ON settings (key)",

    "CREATE TABLE IF NOT EXISTS audit_logs (
        id TEXT PRIMARY KEY,
        user_id TEXT DEFAULT '',
        action TEXT NOT NULL,
        resource TEXT DEFAULT '',
        ip_address TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE INDEX IF NOT EXISTS idx_audit_logs_created ON audit_logs (created_at)",

    // =========================================================================
    // STORAGE / FILES BLOCK — storage_buckets, storage_objects, storage_views
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS storage_buckets (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        public INTEGER DEFAULT 0,
        created_by TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    "CREATE TABLE IF NOT EXISTS storage_objects (
        id TEXT PRIMARY KEY,
        bucket TEXT NOT NULL,
        key TEXT NOT NULL,
        size INTEGER DEFAULT 0,
        content_type TEXT DEFAULT 'application/octet-stream',
        uploaded_by TEXT DEFAULT '',
        uploaded_at TEXT DEFAULT (datetime('now')),
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE INDEX IF NOT EXISTS idx_storage_objects_bucket ON storage_objects (bucket)",

    "CREATE TABLE IF NOT EXISTS storage_views (
        id TEXT PRIMARY KEY,
        bucket TEXT NOT NULL,
        key TEXT NOT NULL,
        user_id TEXT DEFAULT '',
        viewed_at TEXT DEFAULT (datetime('now')),
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    // =========================================================================
    // CLOUD STORAGE — cloud_shares, cloud_access_logs, cloud_quotas
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS cloud_shares (
        id TEXT PRIMARY KEY,
        token TEXT NOT NULL,
        bucket TEXT NOT NULL,
        key TEXT NOT NULL,
        created_by TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        expires_at TEXT,
        access_count INTEGER DEFAULT 0,
        max_access_count INTEGER
    )",
    "CREATE INDEX IF NOT EXISTS idx_cloud_shares_token ON cloud_shares (token)",

    "CREATE TABLE IF NOT EXISTS cloud_access_logs (
        id TEXT PRIMARY KEY,
        share_id TEXT NOT NULL,
        accessed_at TEXT DEFAULT (datetime('now')),
        ip_address TEXT DEFAULT '',
        user_agent TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE INDEX IF NOT EXISTS idx_cloud_access_share ON cloud_access_logs (share_id)",

    "CREATE TABLE IF NOT EXISTS cloud_quotas (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL UNIQUE,
        max_storage_bytes INTEGER DEFAULT 1073741824,
        max_file_size_bytes INTEGER DEFAULT 104857600,
        max_files_per_bucket INTEGER DEFAULT 10000,
        reset_period_days INTEGER DEFAULT 0,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_cloud_quotas_user ON cloud_quotas (user_id)",

    // =========================================================================
    // PRODUCTS BLOCK — products, groups, types, pricing, purchases, line items,
    //                   group templates, product templates, variables
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS block_products_products (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT DEFAULT '',
        slug TEXT DEFAULT '',
        price REAL DEFAULT 0,
        base_price REAL DEFAULT 0,
        currency TEXT DEFAULT 'USD',
        status TEXT DEFAULT 'draft',
        category TEXT DEFAULT '',
        tags TEXT DEFAULT '[]',
        metadata TEXT DEFAULT '{}',
        image_url TEXT DEFAULT '',
        stock INTEGER DEFAULT 0,
        group_id TEXT DEFAULT '',
        type_id TEXT DEFAULT '',
        group_template_id TEXT DEFAULT '',
        product_template_id TEXT DEFAULT '',
        pricing_template_id TEXT DEFAULT '',
        created_by TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        deleted_at TEXT
    )",
    "CREATE INDEX IF NOT EXISTS idx_products_status ON block_products_products (status)",
    "CREATE INDEX IF NOT EXISTS idx_products_group ON block_products_products (group_id)",
    "CREATE INDEX IF NOT EXISTS idx_products_created_by ON block_products_products (created_by)",

    "CREATE TABLE IF NOT EXISTS block_products_groups (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT DEFAULT '',
        template_id TEXT DEFAULT '',
        group_template_id TEXT DEFAULT '',
        user_id TEXT DEFAULT '',
        status TEXT DEFAULT 'active',
        created_by TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    "CREATE TABLE IF NOT EXISTS block_products_types (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT DEFAULT '',
        is_system INTEGER DEFAULT 0,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    "CREATE TABLE IF NOT EXISTS block_products_pricing_templates (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        price_formula TEXT DEFAULT '',
        template_data TEXT DEFAULT '{}',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    "CREATE TABLE IF NOT EXISTS block_products_purchases (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        status TEXT DEFAULT 'pending',
        total_cents INTEGER DEFAULT 0,
        amount_cents INTEGER DEFAULT 0,
        currency TEXT DEFAULT 'USD',
        provider TEXT DEFAULT 'manual',
        metadata TEXT DEFAULT '{}',
        stripe_payment_intent_id TEXT DEFAULT '',
        refunded_at TEXT,
        refunded_by TEXT DEFAULT '',
        refund_reason TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        payment_at TEXT
    )",
    "CREATE INDEX IF NOT EXISTS idx_purchases_user ON block_products_purchases (user_id)",
    "CREATE INDEX IF NOT EXISTS idx_purchases_status ON block_products_purchases (status)",

    "CREATE TABLE IF NOT EXISTS block_products_line_items (
        id TEXT PRIMARY KEY,
        purchase_id TEXT NOT NULL,
        product_id TEXT NOT NULL,
        product_name TEXT DEFAULT '',
        quantity INTEGER DEFAULT 1,
        unit_price REAL DEFAULT 0,
        total_price REAL DEFAULT 0,
        variables TEXT DEFAULT '{}',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE INDEX IF NOT EXISTS idx_line_items_purchase ON block_products_line_items (purchase_id)",

    "CREATE TABLE IF NOT EXISTS block_products_group_templates (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        display_name TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    "CREATE TABLE IF NOT EXISTS block_products_product_templates (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        display_name TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    "CREATE TABLE IF NOT EXISTS block_products_variables (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        var_type TEXT DEFAULT 'number',
        default_value TEXT,
        scope TEXT DEFAULT 'system',
        product_id TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    // =========================================================================
    // LEGAL PAGES BLOCK
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS block_legalpages_legal_documents (
        id TEXT PRIMARY KEY,
        doc_type TEXT NOT NULL,
        title TEXT NOT NULL,
        content TEXT DEFAULT '',
        status TEXT DEFAULT 'draft',
        version INTEGER DEFAULT 1,
        created_by TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        published_at TEXT
    )",
    "CREATE INDEX IF NOT EXISTS idx_legalpages_type_status ON block_legalpages_legal_documents (doc_type, status)",

    // =========================================================================
    // DEPLOYMENTS BLOCK
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS block_deployments (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        name TEXT NOT NULL,
        slug TEXT DEFAULT '',
        status TEXT DEFAULT 'pending',
        config TEXT DEFAULT '{}',
        plan_id TEXT DEFAULT '',
        purchase_id TEXT DEFAULT '',
        tenant_id TEXT DEFAULT '',
        subdomain TEXT DEFAULT '',
        provision_error TEXT,
        deprovision_error TEXT,
        deleted_at TEXT,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE INDEX IF NOT EXISTS idx_deployments_user ON block_deployments (user_id)",
    "CREATE INDEX IF NOT EXISTS idx_deployments_status ON block_deployments (status)",
];

/// Run all schema migrations on a D1 database.
pub async fn run_migrations(db: &D1Database) -> Result<()> {
    for sql in MIGRATIONS {
        db.prepare(*sql).bind(&[])?.run().await?;
    }
    console_log!("Schema migrations applied ({} statements)", MIGRATIONS.len());
    Ok(())
}
