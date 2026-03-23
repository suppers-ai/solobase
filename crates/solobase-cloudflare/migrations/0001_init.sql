-- Auto-generated from block collection declarations.
-- Do not edit manually — regenerate with:
--   npx tsx worker/generate-migration.ts > migrations/0001_init.sql

CREATE TABLE IF NOT EXISTS auth_users (
  id TEXT PRIMARY KEY,
  email TEXT UNIQUE NOT NULL DEFAULT '',
  password_hash TEXT NOT NULL DEFAULT '',
  name TEXT NOT NULL DEFAULT '',
  disabled INTEGER DEFAULT 0,
  avatar_url TEXT NOT NULL DEFAULT '',
  oauth_provider TEXT NOT NULL DEFAULT '',
  last_login_at TEXT DEFAULT '',
  deleted_at TEXT DEFAULT '',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS auth_tokens (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL DEFAULT '',
  token TEXT NOT NULL DEFAULT '',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now')),
  FOREIGN KEY (user_id) REFERENCES auth_users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_auth_tokens_user_id ON auth_tokens (user_id);

CREATE TABLE IF NOT EXISTS api_keys (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL DEFAULT '',
  name TEXT NOT NULL DEFAULT '',
  key_hash TEXT NOT NULL DEFAULT '',
  key_prefix TEXT NOT NULL DEFAULT '',
  last_used TEXT DEFAULT '',
  revoked_at TEXT DEFAULT '',
  expires_at TEXT DEFAULT '',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now')),
  FOREIGN KEY (user_id) REFERENCES auth_users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys (user_id);

CREATE TABLE IF NOT EXISTS iam_roles (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL DEFAULT '',
  description TEXT NOT NULL DEFAULT '',
  permissions TEXT NOT NULL DEFAULT '[]',
  is_system INTEGER DEFAULT 0,
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS iam_permissions (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL DEFAULT '',
  resource TEXT NOT NULL DEFAULT '',
  actions TEXT NOT NULL DEFAULT '[]',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS iam_user_roles (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL DEFAULT '',
  role TEXT NOT NULL DEFAULT '',
  assigned_at TEXT DEFAULT '',
  assigned_by TEXT NOT NULL DEFAULT '',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now')),
  FOREIGN KEY (user_id) REFERENCES auth_users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_iam_user_roles_user_id ON iam_user_roles (user_id);

CREATE TABLE IF NOT EXISTS settings (
  id TEXT PRIMARY KEY,
  key TEXT UNIQUE NOT NULL DEFAULT '',
  value TEXT NOT NULL DEFAULT '',
  updated_by TEXT NOT NULL DEFAULT '',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS audit_logs (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL DEFAULT '',
  action TEXT NOT NULL DEFAULT '',
  resource TEXT NOT NULL DEFAULT '',
  ip_address TEXT NOT NULL DEFAULT '',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs (created_at);

CREATE TABLE IF NOT EXISTS storage_buckets (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL DEFAULT '',
  public INTEGER DEFAULT 0,
  created_by TEXT NOT NULL DEFAULT '',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS storage_objects (
  id TEXT PRIMARY KEY,
  bucket TEXT NOT NULL DEFAULT '',
  key TEXT NOT NULL DEFAULT '',
  size INTEGER NOT NULL DEFAULT 0,
  content_type TEXT NOT NULL DEFAULT 'application/octet-stream',
  uploaded_by TEXT NOT NULL DEFAULT '',
  uploaded_at TEXT DEFAULT '',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_storage_objects_bucket ON storage_objects (bucket);

CREATE TABLE IF NOT EXISTS storage_views (
  id TEXT PRIMARY KEY,
  bucket TEXT NOT NULL DEFAULT '',
  key TEXT NOT NULL DEFAULT '',
  user_id TEXT NOT NULL DEFAULT '',
  viewed_at TEXT DEFAULT '',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS cloud_shares (
  id TEXT PRIMARY KEY,
  token TEXT NOT NULL DEFAULT '',
  bucket TEXT NOT NULL DEFAULT '',
  key TEXT NOT NULL DEFAULT '',
  created_by TEXT NOT NULL DEFAULT '',
  expires_at TEXT DEFAULT '',
  access_count INTEGER NOT NULL DEFAULT 0,
  max_access_count INTEGER DEFAULT '',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_cloud_shares_token ON cloud_shares (token);

CREATE TABLE IF NOT EXISTS cloud_access_logs (
  id TEXT PRIMARY KEY,
  share_id TEXT NOT NULL DEFAULT '',
  accessed_at TEXT DEFAULT '',
  ip_address TEXT NOT NULL DEFAULT '',
  user_agent TEXT NOT NULL DEFAULT '',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_cloud_access_logs_share_id ON cloud_access_logs (share_id);

CREATE TABLE IF NOT EXISTS cloud_quotas (
  id TEXT PRIMARY KEY,
  user_id TEXT UNIQUE NOT NULL DEFAULT '',
  max_storage_bytes INTEGER NOT NULL DEFAULT 1073741824,
  max_file_size_bytes INTEGER NOT NULL DEFAULT 104857600,
  max_files_per_bucket INTEGER NOT NULL DEFAULT 10000,
  reset_period_days INTEGER NOT NULL DEFAULT 0,
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS block_products_products (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL DEFAULT '',
  description TEXT NOT NULL DEFAULT '',
  slug TEXT NOT NULL DEFAULT '',
  price REAL NOT NULL DEFAULT 0,
  base_price REAL NOT NULL DEFAULT 0,
  currency TEXT NOT NULL DEFAULT 'USD',
  status TEXT NOT NULL DEFAULT 'draft',
  category TEXT NOT NULL DEFAULT '',
  tags TEXT NOT NULL DEFAULT '[]',
  metadata TEXT NOT NULL DEFAULT '{}',
  image_url TEXT NOT NULL DEFAULT '',
  stock INTEGER NOT NULL DEFAULT 0,
  group_id TEXT NOT NULL DEFAULT '',
  type_id TEXT NOT NULL DEFAULT '',
  group_template_id TEXT NOT NULL DEFAULT '',
  product_template_id TEXT NOT NULL DEFAULT '',
  pricing_template_id TEXT NOT NULL DEFAULT '',
  created_by TEXT NOT NULL DEFAULT '',
  deleted_at TEXT DEFAULT '',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_block_products_products_status ON block_products_products (status);

CREATE INDEX IF NOT EXISTS idx_block_products_products_group_id ON block_products_products (group_id);

CREATE INDEX IF NOT EXISTS idx_block_products_products_created_by ON block_products_products (created_by);

CREATE TABLE IF NOT EXISTS block_products_groups (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL DEFAULT '',
  description TEXT NOT NULL DEFAULT '',
  template_id TEXT NOT NULL DEFAULT '',
  group_template_id TEXT NOT NULL DEFAULT '',
  user_id TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'active',
  created_by TEXT NOT NULL DEFAULT '',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS block_products_types (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL DEFAULT '',
  description TEXT NOT NULL DEFAULT '',
  is_system INTEGER DEFAULT 0,
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS block_products_pricing_templates (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL DEFAULT '',
  price_formula TEXT NOT NULL DEFAULT '',
  template_data TEXT NOT NULL DEFAULT '{}',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS block_products_purchases (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'pending',
  total_cents INTEGER NOT NULL DEFAULT 0,
  amount_cents INTEGER NOT NULL DEFAULT 0,
  currency TEXT NOT NULL DEFAULT 'USD',
  provider TEXT NOT NULL DEFAULT 'manual',
  metadata TEXT NOT NULL DEFAULT '{}',
  stripe_payment_intent_id TEXT NOT NULL DEFAULT '',
  refunded_at TEXT DEFAULT '',
  refunded_by TEXT NOT NULL DEFAULT '',
  refund_reason TEXT NOT NULL DEFAULT '',
  payment_at TEXT DEFAULT '',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now')),
  FOREIGN KEY (user_id) REFERENCES auth_users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_block_products_purchases_user_id ON block_products_purchases (user_id);

CREATE INDEX IF NOT EXISTS idx_block_products_purchases_status ON block_products_purchases (status);

CREATE TABLE IF NOT EXISTS block_products_line_items (
  id TEXT PRIMARY KEY,
  purchase_id TEXT NOT NULL DEFAULT '',
  product_id TEXT NOT NULL DEFAULT '',
  product_name TEXT NOT NULL DEFAULT '',
  quantity INTEGER NOT NULL DEFAULT 1,
  unit_price REAL NOT NULL DEFAULT 0,
  total_price REAL NOT NULL DEFAULT 0,
  variables TEXT NOT NULL DEFAULT '{}',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_block_products_line_items_purchase_id ON block_products_line_items (purchase_id);

CREATE TABLE IF NOT EXISTS block_products_group_templates (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL DEFAULT '',
  display_name TEXT NOT NULL DEFAULT '',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS block_products_product_templates (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL DEFAULT '',
  display_name TEXT NOT NULL DEFAULT '',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS block_products_variables (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL DEFAULT '',
  var_type TEXT NOT NULL DEFAULT 'number',
  default_value TEXT DEFAULT '',
  scope TEXT NOT NULL DEFAULT 'system',
  product_id TEXT NOT NULL DEFAULT '',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS block_legalpages_legal_documents (
  id TEXT PRIMARY KEY,
  doc_type TEXT NOT NULL DEFAULT '',
  title TEXT NOT NULL DEFAULT '',
  content TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'draft',
  version INTEGER NOT NULL DEFAULT 1,
  created_by TEXT NOT NULL DEFAULT '',
  published_at TEXT DEFAULT '',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_block_legalpages_legal_documents_doc_type_status ON block_legalpages_legal_documents (doc_type, status);

CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL DEFAULT '',
  name TEXT NOT NULL DEFAULT '',
  slug TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'pending',
  config TEXT NOT NULL DEFAULT '{}',
  plan_id TEXT NOT NULL DEFAULT '',
  purchase_id TEXT NOT NULL DEFAULT '',
  project_id TEXT NOT NULL DEFAULT '',
  subdomain TEXT NOT NULL DEFAULT '',
  provision_error TEXT DEFAULT '',
  deprovision_error TEXT DEFAULT '',
  deleted_at TEXT DEFAULT '',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now')),
  FOREIGN KEY (user_id) REFERENCES auth_users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_projects_user_id ON projects (user_id);

CREATE INDEX IF NOT EXISTS idx_projects_status ON projects (status);

CREATE TABLE IF NOT EXISTS project_usage (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  month TEXT NOT NULL,
  requests INTEGER NOT NULL DEFAULT 0,
  r2_bytes INTEGER NOT NULL DEFAULT 0,
  addon_requests INTEGER NOT NULL DEFAULT 0,
  addon_r2_bytes INTEGER NOT NULL DEFAULT 0,
  addon_d1_bytes INTEGER NOT NULL DEFAULT 0,
  UNIQUE (project_id, month)
);

CREATE INDEX IF NOT EXISTS idx_project_usage_project_month ON project_usage (project_id, month);

CREATE TABLE IF NOT EXISTS subscriptions (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  plan TEXT NOT NULL DEFAULT 'starter',
  stripe_customer_id TEXT NOT NULL DEFAULT '',
  stripe_subscription_id TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'active',
  current_period_end TEXT,
  grace_period_end TEXT,
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now')),
  FOREIGN KEY (user_id) REFERENCES auth_users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_subscriptions_user_id ON subscriptions (user_id);

CREATE INDEX IF NOT EXISTS idx_subscriptions_stripe_sub ON subscriptions (stripe_subscription_id);
