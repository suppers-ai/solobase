//! Per-project D1 database schema and migrations.
//!
//! Each project has its own D1 database, so tables do not need a tenant_id
//! column — all data in the database belongs to that single project.
//!
//! Platform-only tables (subscriptions, project_usage) live in the platform
//! DB managed by the dispatch worker — they are NOT included here.

use worker::*;

/// SQL statements to create the standard Solobase tables.
const MIGRATIONS: &[&str] = &[
    // =========================================================================
    // AUTH BLOCK — suppers_ai__auth__users, tokens, api_keys, rate_limits
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS suppers_ai__auth__users (
        id TEXT PRIMARY KEY,
        email TEXT NOT NULL UNIQUE,
        password_hash TEXT NOT NULL,
        name TEXT DEFAULT '',
        disabled INTEGER DEFAULT 0,
        avatar_url TEXT DEFAULT '',
        oauth_provider TEXT DEFAULT '',
        email_verified INTEGER DEFAULT 0,
        verification_token TEXT DEFAULT '',
        reset_token TEXT DEFAULT '',
        reset_token_expires TEXT,
        last_verification_sent TEXT,
        last_login_at TEXT,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        deleted_at TEXT
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_suppers_ai__auth__users_email ON suppers_ai__auth__users (email)",

    "CREATE TABLE IF NOT EXISTS suppers_ai__auth__tokens (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        token TEXT NOT NULL,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        FOREIGN KEY (user_id) REFERENCES suppers_ai__auth__users(id) ON DELETE CASCADE
    )",
    "CREATE INDEX IF NOT EXISTS idx_suppers_ai__auth__tokens_user ON suppers_ai__auth__tokens (user_id)",

    "CREATE TABLE IF NOT EXISTS suppers_ai__auth__api_keys (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        name TEXT DEFAULT '',
        key_hash TEXT NOT NULL,
        key_prefix TEXT DEFAULT '',
        last_used TEXT,
        expires_at TEXT,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        revoked_at TEXT,
        FOREIGN KEY (user_id) REFERENCES suppers_ai__auth__users(id) ON DELETE CASCADE
    )",
    "CREATE INDEX IF NOT EXISTS idx_suppers_ai__auth__api_keys_user ON suppers_ai__auth__api_keys (user_id)",

    "CREATE TABLE IF NOT EXISTS suppers_ai__auth__rate_limits (
        id TEXT PRIMARY KEY,
        key TEXT NOT NULL UNIQUE,
        count INTEGER DEFAULT 0,
        window_start INTEGER DEFAULT 0,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    // =========================================================================
    // ADMIN BLOCK — roles, permissions, user_roles
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS suppers_ai__admin__roles (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT DEFAULT '',
        permissions TEXT DEFAULT '[]',
        is_system INTEGER DEFAULT 0,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    "CREATE TABLE IF NOT EXISTS suppers_ai__admin__permissions (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        resource TEXT DEFAULT '',
        actions TEXT DEFAULT '[]',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    "CREATE TABLE IF NOT EXISTS suppers_ai__admin__user_roles (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        role TEXT NOT NULL,
        assigned_at TEXT DEFAULT (datetime('now')),
        assigned_by TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        FOREIGN KEY (user_id) REFERENCES suppers_ai__auth__users(id) ON DELETE CASCADE
    )",
    "CREATE INDEX IF NOT EXISTS idx_suppers_ai__admin__user_roles_user ON suppers_ai__admin__user_roles (user_id)",

    // =========================================================================
    // ADMIN BLOCK — variables, audit_logs
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS suppers_ai__admin__variables (
        id TEXT PRIMARY KEY,
        key TEXT NOT NULL UNIQUE,
        name TEXT DEFAULT '',
        description TEXT DEFAULT '',
        value TEXT DEFAULT '',
        warning TEXT DEFAULT '',
        sensitive INTEGER DEFAULT 0,
        updated_by TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_suppers_ai__admin__variables_key ON suppers_ai__admin__variables (key)",

    "CREATE TABLE IF NOT EXISTS suppers_ai__admin__audit_logs (
        id TEXT PRIMARY KEY,
        user_id TEXT DEFAULT '',
        action TEXT NOT NULL,
        resource TEXT DEFAULT '',
        ip_address TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE INDEX IF NOT EXISTS idx_suppers_ai__admin__audit_logs_created ON suppers_ai__admin__audit_logs (created_at)",

    // =========================================================================
    // FILES BLOCK
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS suppers_ai__files__buckets (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        public INTEGER DEFAULT 0,
        created_by TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    "CREATE TABLE IF NOT EXISTS suppers_ai__files__objects (
        id TEXT PRIMARY KEY,
        bucket TEXT NOT NULL,
        key TEXT NOT NULL,
        size INTEGER DEFAULT 0,
        content_type TEXT DEFAULT 'application/octet-stream',
        status TEXT DEFAULT 'complete',
        uploaded_by TEXT DEFAULT '',
        uploaded_at TEXT DEFAULT (datetime('now')),
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE INDEX IF NOT EXISTS idx_suppers_ai__files__objects_bucket ON suppers_ai__files__objects (bucket)",

    "CREATE TABLE IF NOT EXISTS suppers_ai__files__views (
        id TEXT PRIMARY KEY,
        bucket TEXT NOT NULL,
        key TEXT NOT NULL,
        user_id TEXT DEFAULT '',
        viewed_at TEXT DEFAULT (datetime('now')),
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    // =========================================================================
    // FILES BLOCK — cloud shares, access logs, quotas
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS suppers_ai__files__cloud_shares (
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
    "CREATE INDEX IF NOT EXISTS idx_suppers_ai__files__cloud_shares_token ON suppers_ai__files__cloud_shares (token)",

    "CREATE TABLE IF NOT EXISTS suppers_ai__files__cloud_access_logs (
        id TEXT PRIMARY KEY,
        share_id TEXT NOT NULL,
        accessed_at TEXT DEFAULT (datetime('now')),
        ip_address TEXT DEFAULT '',
        user_agent TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    "CREATE TABLE IF NOT EXISTS suppers_ai__files__cloud_quotas (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL UNIQUE,
        max_storage_bytes INTEGER DEFAULT 1073741824,
        max_file_size_bytes INTEGER DEFAULT 104857600,
        max_files_per_bucket INTEGER DEFAULT 10000,
        reset_period_days INTEGER DEFAULT 0,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_suppers_ai__files__cloud_quotas_user ON suppers_ai__files__cloud_quotas (user_id)",

    // =========================================================================
    // PRODUCTS BLOCK
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS suppers_ai__products__products (
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
        sort_order INTEGER DEFAULT 0,
        created_by TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        deleted_at TEXT
    )",
    "CREATE INDEX IF NOT EXISTS idx_suppers_ai__products__products_status ON suppers_ai__products__products (status)",
    "CREATE INDEX IF NOT EXISTS idx_suppers_ai__products__products_group ON suppers_ai__products__products (group_id)",

    "CREATE TABLE IF NOT EXISTS suppers_ai__products__groups (
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

    "CREATE TABLE IF NOT EXISTS suppers_ai__products__types (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT DEFAULT '',
        is_system INTEGER DEFAULT 0,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    "CREATE TABLE IF NOT EXISTS suppers_ai__products__pricing_templates (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        price_formula TEXT DEFAULT '',
        template_data TEXT DEFAULT '{}',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    "CREATE TABLE IF NOT EXISTS suppers_ai__products__purchases (
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
    "CREATE INDEX IF NOT EXISTS idx_suppers_ai__products__purchases_user ON suppers_ai__products__purchases (user_id)",
    "CREATE INDEX IF NOT EXISTS idx_suppers_ai__products__purchases_status ON suppers_ai__products__purchases (status)",

    "CREATE TABLE IF NOT EXISTS suppers_ai__products__line_items (
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
    "CREATE INDEX IF NOT EXISTS idx_suppers_ai__products__line_items_purchase ON suppers_ai__products__line_items (purchase_id)",

    "CREATE TABLE IF NOT EXISTS suppers_ai__products__group_templates (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        display_name TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    "CREATE TABLE IF NOT EXISTS suppers_ai__products__product_templates (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        display_name TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    "CREATE TABLE IF NOT EXISTS suppers_ai__products__variables (
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

    "CREATE TABLE IF NOT EXISTS suppers_ai__legalpages__documents (
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
    "CREATE INDEX IF NOT EXISTS idx_suppers_ai__legalpages__documents_type_status ON suppers_ai__legalpages__documents (doc_type, status)",

    // =========================================================================
    // PROJECTS BLOCK
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS suppers_ai__projects__deployments (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        name TEXT NOT NULL,
        slug TEXT DEFAULT '',
        status TEXT DEFAULT 'pending',
        config TEXT DEFAULT '{}',
        plan TEXT DEFAULT 'free',
        plan_id TEXT DEFAULT '',
        purchase_id TEXT DEFAULT '',
        tenant_id TEXT DEFAULT '',
        subdomain TEXT DEFAULT '',
        provision_error TEXT,
        deprovision_error TEXT,
        grace_period_end TEXT,
        deleted_at TEXT,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE INDEX IF NOT EXISTS idx_suppers_ai__projects__deployments_user ON suppers_ai__projects__deployments (user_id)",
    "CREATE INDEX IF NOT EXISTS idx_suppers_ai__projects__deployments_status ON suppers_ai__projects__deployments (status)",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_suppers_ai__projects__deployments_subdomain ON suppers_ai__projects__deployments (subdomain) WHERE subdomain != ''",

    // =========================================================================
    // USERPORTAL BLOCK
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS suppers_ai__userportal__profiles (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL UNIQUE,
        display_name TEXT DEFAULT '',
        bio TEXT DEFAULT '',
        avatar_url TEXT DEFAULT '',
        preferences TEXT DEFAULT '{}',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_suppers_ai__userportal__profiles_user ON suppers_ai__userportal__profiles (user_id)",

    // =========================================================================
    // ADMIN BLOCK — per-block enable/disable
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS suppers_ai__admin__block_settings (
        block_name TEXT PRIMARY KEY,
        enabled INTEGER DEFAULT 1,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",

    // =========================================================================
    // ADMIN BLOCK — request telemetry
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS suppers_ai__admin__request_logs (
        id TEXT PRIMARY KEY,
        flow_id TEXT DEFAULT '',
        method TEXT DEFAULT '',
        path TEXT DEFAULT '',
        status TEXT DEFAULT '',
        status_code INTEGER DEFAULT 0,
        duration_ms INTEGER DEFAULT 0,
        error_message TEXT DEFAULT '',
        client_ip TEXT DEFAULT '',
        user_id TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE INDEX IF NOT EXISTS idx_suppers_ai__admin__request_logs_created ON suppers_ai__admin__request_logs (created_at)",

    // =========================================================================
    // ADMIN BLOCK — network rules
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS suppers_ai__admin__network_rules (
        id TEXT PRIMARY KEY,
        scope TEXT DEFAULT 'global',
        block_name TEXT DEFAULT '',
        rule_type TEXT NOT NULL,
        pattern TEXT NOT NULL,
        priority INTEGER DEFAULT 0,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE INDEX IF NOT EXISTS idx_suppers_ai__admin__network_rules_scope ON suppers_ai__admin__network_rules (scope)",

    // =========================================================================
    // ADMIN BLOCK — storage access logs and rules
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS suppers_ai__admin__storage_rules (
        id TEXT PRIMARY KEY,
        rule_type TEXT DEFAULT 'allow',
        source_block TEXT DEFAULT '*',
        target_path TEXT NOT NULL,
        access TEXT DEFAULT 'readwrite',
        priority INTEGER DEFAULT 0,
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE INDEX IF NOT EXISTS idx_suppers_ai__admin__storage_rules_source ON suppers_ai__admin__storage_rules (source_block)",

    "CREATE TABLE IF NOT EXISTS suppers_ai__admin__storage_access_logs (
        id TEXT PRIMARY KEY,
        source_block TEXT DEFAULT '',
        operation TEXT DEFAULT '',
        path TEXT DEFAULT '',
        status TEXT DEFAULT '',
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now'))
    )",
    "CREATE INDEX IF NOT EXISTS idx_suppers_ai__admin__storage_access_logs_created ON suppers_ai__admin__storage_access_logs (created_at)",

    // =========================================================================
    // SEED DATA — default IAM roles, settings, block states
    // =========================================================================

    "INSERT OR IGNORE INTO suppers_ai__admin__roles (id, name, description, permissions, is_system) VALUES
        ('role_admin', 'admin', 'Full administrative access', '[\"*\"]', 1),
        ('role_user', 'user', 'Standard user access', '[\"read\",\"write\"]', 1)",

    // =========================================================================
    // CUSTOM BLOCKS — installed third-party WASM blocks
    // =========================================================================

    "CREATE TABLE IF NOT EXISTS custom_blocks (
        name TEXT PRIMARY KEY,
        version TEXT NOT NULL,
        uploaded_at TEXT NOT NULL DEFAULT (datetime('now')),
        capabilities TEXT NOT NULL DEFAULT '{}'
    )",

    // Seed block defaults — projects, legalpages, userportal disabled by default
    "INSERT OR IGNORE INTO suppers_ai__admin__block_settings (block_name, enabled) VALUES
        ('suppers-ai/auth', 1),
        ('suppers-ai/admin', 1),
        ('suppers-ai/files', 1),
        ('suppers-ai/products', 1),
        ('suppers-ai/projects', 0),
        ('suppers-ai/legalpages', 0),
        ('suppers-ai/userportal', 0),
        ('suppers-ai/system', 1)",
];

/// Run all schema migrations on a D1 database.
pub async fn run_migrations(db: &D1Database) -> Result<()> {
    for sql in MIGRATIONS {
        db.prepare(*sql).bind(&[])?.run().await?;
    }

    console_log!(
        "Schema migrations applied ({} statements)",
        MIGRATIONS.len()
    );
    Ok(())
}
