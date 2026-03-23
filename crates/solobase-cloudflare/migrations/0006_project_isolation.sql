-- Migration: Add project_id column for project isolation.
--
-- Every project-scoped table gets a `project_id` column that the D1 service
-- layer filters on automatically. Platform-level tables (project_usage,
-- subscriptions, projects) are NOT modified — they are scoped to the
-- platform context.
--
-- Existing rows are backfilled with project_id = 'platform' (the platform
-- context). New rows will be tagged with the project ID by the D1 service
-- layer.

-- ---------------------------------------------------------------------------
-- 1. Add project_id to all project-scoped tables
-- ---------------------------------------------------------------------------

ALTER TABLE auth_users ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE auth_tokens ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE api_keys ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE iam_roles ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE iam_permissions ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE iam_user_roles ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE settings ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE audit_logs ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE storage_buckets ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE storage_objects ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE storage_views ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE cloud_shares ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE cloud_access_logs ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE cloud_quotas ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE block_products_products ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE block_products_groups ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE block_products_types ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE block_products_pricing_templates ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE block_products_purchases ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE block_products_line_items ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE block_products_group_templates ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE block_products_product_templates ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE block_products_variables ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';
ALTER TABLE block_legalpages_legal_documents ADD COLUMN project_id TEXT NOT NULL DEFAULT 'platform';

-- ---------------------------------------------------------------------------
-- 2. Create indexes on project_id for efficient filtering
-- ---------------------------------------------------------------------------

CREATE INDEX IF NOT EXISTS idx_auth_users_project ON auth_users (project_id);
CREATE INDEX IF NOT EXISTS idx_auth_tokens_project ON auth_tokens (project_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_project ON api_keys (project_id);
CREATE INDEX IF NOT EXISTS idx_iam_roles_project ON iam_roles (project_id);
CREATE INDEX IF NOT EXISTS idx_iam_permissions_project ON iam_permissions (project_id);
CREATE INDEX IF NOT EXISTS idx_iam_user_roles_project ON iam_user_roles (project_id);
CREATE INDEX IF NOT EXISTS idx_settings_project ON settings (project_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_project ON audit_logs (project_id);
CREATE INDEX IF NOT EXISTS idx_storage_buckets_project ON storage_buckets (project_id);
CREATE INDEX IF NOT EXISTS idx_storage_objects_project ON storage_objects (project_id);
CREATE INDEX IF NOT EXISTS idx_storage_views_project ON storage_views (project_id);
CREATE INDEX IF NOT EXISTS idx_cloud_shares_project ON cloud_shares (project_id);
CREATE INDEX IF NOT EXISTS idx_cloud_access_logs_project ON cloud_access_logs (project_id);
CREATE INDEX IF NOT EXISTS idx_cloud_quotas_project ON cloud_quotas (project_id);
CREATE INDEX IF NOT EXISTS idx_block_products_products_project ON block_products_products (project_id);
CREATE INDEX IF NOT EXISTS idx_block_products_purchases_project ON block_products_purchases (project_id);
CREATE INDEX IF NOT EXISTS idx_block_legalpages_project ON block_legalpages_legal_documents (project_id);

-- ---------------------------------------------------------------------------
-- 3. Per-project unique constraints
-- ---------------------------------------------------------------------------

-- Email must be unique within a project (not globally).
-- The original column-level UNIQUE on email enforces global uniqueness.
-- We add a composite index; the column-level constraint will be relaxed
-- when the table is recreated (see 0007_auth_users_rebuild.sql).
CREATE UNIQUE INDEX IF NOT EXISTS idx_auth_users_email_project ON auth_users (project_id, email);

-- Settings key must be unique within a project (not globally).
CREATE UNIQUE INDEX IF NOT EXISTS idx_settings_key_project ON settings (project_id, key);

-- Cloud quotas user_id must be unique within a project (not globally).
CREATE UNIQUE INDEX IF NOT EXISTS idx_cloud_quotas_user_project ON cloud_quotas (project_id, user_id);

-- ---------------------------------------------------------------------------
-- 4. Create used_one_time_tokens table for single-use token tracking
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS used_one_time_tokens (
  id TEXT PRIMARY KEY,
  jti TEXT NOT NULL DEFAULT '',
  type TEXT NOT NULL DEFAULT '',
  consumed_at TEXT DEFAULT '',
  project_id TEXT NOT NULL DEFAULT 'platform',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_used_one_time_tokens_jti ON used_one_time_tokens (jti);
CREATE INDEX IF NOT EXISTS idx_used_one_time_tokens_project ON used_one_time_tokens (project_id);

-- ---------------------------------------------------------------------------
-- 5. Create rate_limits table (optional D1-based rate limiting backup)
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS rate_limits (
  id TEXT PRIMARY KEY,
  key TEXT UNIQUE NOT NULL,
  count INTEGER NOT NULL DEFAULT 0,
  window_start INTEGER NOT NULL DEFAULT 0,
  created_at TEXT DEFAULT (datetime('now'))
);
