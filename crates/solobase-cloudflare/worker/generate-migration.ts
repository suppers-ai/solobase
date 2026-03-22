/**
 * Auto-create database tables from block collection declarations.
 *
 * This is the CF Worker equivalent of the native runtime's
 * `ensure_schema_tables()` — it reads collection schemas from block
 * declarations and runs CREATE TABLE IF NOT EXISTS on D1.
 *
 * Called once per worker isolate on first request.
 */

/** Field type → SQLite type mapping. */
function sqlType(fieldType: string): string {
  switch (fieldType) {
    case 'int': case 'integer': case 'int64': case 'bool': case 'boolean':
      return 'INTEGER';
    case 'float': case 'number':
      return 'REAL';
    case 'blob':
      return 'BLOB';
    default: // string, text, datetime, json
      return 'TEXT';
  }
}

/** Parse a default value to SQL. */
function sqlDefault(fieldType: string, defaultValue: string): string {
  if (defaultValue === undefined || defaultValue === null) return '';
  if (defaultValue === '') return " DEFAULT ''";
  if (defaultValue === 'false') return " DEFAULT 0";
  if (defaultValue === 'true') return " DEFAULT 1";
  if (defaultValue === 'CURRENT_TIMESTAMP' || defaultValue === 'NOW()') {
    return " DEFAULT (datetime('now'))";
  }
  // Numeric
  if (/^-?\d+(\.\d+)?$/.test(defaultValue)) {
    return ` DEFAULT ${defaultValue}`;
  }
  // String
  return ` DEFAULT '${defaultValue.replace(/'/g, "''")}'`;
}

export interface CollectionSchema {
  name: string;
  fields: FieldSchema[];
  indexes?: IndexSchema[];
}

export interface FieldSchema {
  name: string;
  fieldType: string;
  unique?: boolean;
  optional?: boolean;
  defaultValue?: string;
  reference?: string;
}

export interface IndexSchema {
  fields: string[];
  unique?: boolean;
}

/**
 * Generate CREATE TABLE + CREATE INDEX SQL for a collection.
 * Every table gets: id TEXT PRIMARY KEY, created_at, updated_at (auto).
 */
function generateCreateTable(col: CollectionSchema): string[] {
  const statements: string[] = [];
  const columns: string[] = [
    'id TEXT PRIMARY KEY',
  ];
  const fks: string[] = [];

  for (const f of col.fields) {
    let def = `${f.name} ${sqlType(f.fieldType)}`;
    if (f.unique) def += ' UNIQUE';
    if (!f.optional && f.fieldType !== 'bool') def += ' NOT NULL';
    def += sqlDefault(f.fieldType, f.defaultValue ?? '');
    columns.push(def);

    if (f.reference) {
      const [refTable, refCol] = f.reference.split('.');
      if (refTable && refCol) {
        fks.push(`FOREIGN KEY (${f.name}) REFERENCES ${refTable}(${refCol}) ON DELETE CASCADE`);
      }
    }
  }

  // Auto timestamps
  columns.push("created_at TEXT DEFAULT (datetime('now'))");
  columns.push("updated_at TEXT DEFAULT (datetime('now'))");

  const allParts = [...columns, ...fks];
  statements.push(`CREATE TABLE IF NOT EXISTS ${col.name} (\n  ${allParts.join(',\n  ')}\n)`);

  // Indexes
  for (const idx of col.indexes ?? []) {
    const idxName = `idx_${col.name}_${idx.fields.join('_')}`;
    const unique = idx.unique ? 'UNIQUE ' : '';
    statements.push(
      `CREATE ${unique}INDEX IF NOT EXISTS ${idxName} ON ${col.name} (${idx.fields.join(', ')})`
    );
  }

  return statements;
}

/**
 * All solobase block collection schemas.
 * This is the single source of truth — matches the Rust CollectionSchema
 * declarations in solobase-core block info() methods.
 */
const BLOCK_COLLECTIONS: CollectionSchema[] = [
  // AUTH
  {
    name: 'auth_users',
    fields: [
      { name: 'email', fieldType: 'string', unique: true },
      { name: 'password_hash', fieldType: 'string', defaultValue: '' },
      { name: 'name', fieldType: 'string', defaultValue: '' },
      { name: 'disabled', fieldType: 'bool', defaultValue: 'false' },
      { name: 'avatar_url', fieldType: 'string', defaultValue: '' },
      { name: 'oauth_provider', fieldType: 'string', defaultValue: '' },
      { name: 'last_login_at', fieldType: 'datetime', optional: true },
      { name: 'deleted_at', fieldType: 'datetime', optional: true },
    ],
  },
  {
    name: 'auth_tokens',
    fields: [
      { name: 'user_id', fieldType: 'string', reference: 'auth_users.id' },
      { name: 'token', fieldType: 'string' },
    ],
    indexes: [{ fields: ['user_id'] }],
  },
  {
    name: 'api_keys',
    fields: [
      { name: 'user_id', fieldType: 'string', reference: 'auth_users.id' },
      { name: 'name', fieldType: 'string', defaultValue: '' },
      { name: 'key_hash', fieldType: 'string' },
      { name: 'key_prefix', fieldType: 'string', defaultValue: '' },
      { name: 'last_used', fieldType: 'datetime', optional: true },
      { name: 'revoked_at', fieldType: 'datetime', optional: true },
      { name: 'expires_at', fieldType: 'datetime', optional: true },
    ],
    indexes: [{ fields: ['user_id'] }],
  },
  // IAM
  {
    name: 'iam_roles',
    fields: [
      { name: 'name', fieldType: 'string' },
      { name: 'description', fieldType: 'string', defaultValue: '' },
      { name: 'permissions', fieldType: 'json', defaultValue: '[]' },
      { name: 'is_system', fieldType: 'bool', defaultValue: 'false' },
    ],
  },
  {
    name: 'iam_permissions',
    fields: [
      { name: 'name', fieldType: 'string' },
      { name: 'resource', fieldType: 'string', defaultValue: '' },
      { name: 'actions', fieldType: 'json', defaultValue: '[]' },
    ],
  },
  {
    name: 'iam_user_roles',
    fields: [
      { name: 'user_id', fieldType: 'string', reference: 'auth_users.id' },
      { name: 'role', fieldType: 'string' },
      { name: 'assigned_at', fieldType: 'datetime', optional: true },
      { name: 'assigned_by', fieldType: 'string', defaultValue: '' },
    ],
    indexes: [{ fields: ['user_id'] }],
  },
  // ADMIN
  {
    name: 'settings',
    fields: [
      { name: 'key', fieldType: 'string', unique: true },
      { name: 'value', fieldType: 'string', defaultValue: '' },
      { name: 'updated_by', fieldType: 'string', defaultValue: '' },
    ],
  },
  {
    name: 'audit_logs',
    fields: [
      { name: 'user_id', fieldType: 'string', defaultValue: '' },
      { name: 'action', fieldType: 'string' },
      { name: 'resource', fieldType: 'string', defaultValue: '' },
      { name: 'ip_address', fieldType: 'string', defaultValue: '' },
    ],
    indexes: [{ fields: ['created_at'] }],
  },
  // FILES
  {
    name: 'storage_buckets',
    fields: [
      { name: 'name', fieldType: 'string' },
      { name: 'public', fieldType: 'bool', defaultValue: 'false' },
      { name: 'created_by', fieldType: 'string', defaultValue: '' },
    ],
  },
  {
    name: 'storage_objects',
    fields: [
      { name: 'bucket', fieldType: 'string' },
      { name: 'key', fieldType: 'string' },
      { name: 'size', fieldType: 'int', defaultValue: '0' },
      { name: 'content_type', fieldType: 'string', defaultValue: 'application/octet-stream' },
      { name: 'uploaded_by', fieldType: 'string', defaultValue: '' },
      { name: 'uploaded_at', fieldType: 'datetime', optional: true },
    ],
    indexes: [{ fields: ['bucket'] }],
  },
  {
    name: 'storage_views',
    fields: [
      { name: 'bucket', fieldType: 'string' },
      { name: 'key', fieldType: 'string' },
      { name: 'user_id', fieldType: 'string', defaultValue: '' },
      { name: 'viewed_at', fieldType: 'datetime', optional: true },
    ],
  },
  {
    name: 'cloud_shares',
    fields: [
      { name: 'token', fieldType: 'string' },
      { name: 'bucket', fieldType: 'string' },
      { name: 'key', fieldType: 'string' },
      { name: 'created_by', fieldType: 'string', defaultValue: '' },
      { name: 'expires_at', fieldType: 'datetime', optional: true },
      { name: 'access_count', fieldType: 'int', defaultValue: '0' },
      { name: 'max_access_count', fieldType: 'int', optional: true },
    ],
    indexes: [{ fields: ['token'] }],
  },
  {
    name: 'cloud_access_logs',
    fields: [
      { name: 'share_id', fieldType: 'string' },
      { name: 'accessed_at', fieldType: 'datetime', optional: true },
      { name: 'ip_address', fieldType: 'string', defaultValue: '' },
      { name: 'user_agent', fieldType: 'string', defaultValue: '' },
    ],
    indexes: [{ fields: ['share_id'] }],
  },
  {
    name: 'cloud_quotas',
    fields: [
      { name: 'user_id', fieldType: 'string', unique: true },
      { name: 'max_storage_bytes', fieldType: 'int', defaultValue: '1073741824' },
      { name: 'max_file_size_bytes', fieldType: 'int', defaultValue: '104857600' },
      { name: 'max_files_per_bucket', fieldType: 'int', defaultValue: '10000' },
      { name: 'reset_period_days', fieldType: 'int', defaultValue: '0' },
    ],
  },
  // PRODUCTS
  {
    name: 'block_products_products',
    fields: [
      { name: 'name', fieldType: 'string' },
      { name: 'description', fieldType: 'string', defaultValue: '' },
      { name: 'slug', fieldType: 'string', defaultValue: '' },
      { name: 'price', fieldType: 'float', defaultValue: '0' },
      { name: 'base_price', fieldType: 'float', defaultValue: '0' },
      { name: 'currency', fieldType: 'string', defaultValue: 'USD' },
      { name: 'status', fieldType: 'string', defaultValue: 'draft' },
      { name: 'category', fieldType: 'string', defaultValue: '' },
      { name: 'tags', fieldType: 'json', defaultValue: '[]' },
      { name: 'metadata', fieldType: 'json', defaultValue: '{}' },
      { name: 'image_url', fieldType: 'string', defaultValue: '' },
      { name: 'stock', fieldType: 'int', defaultValue: '0' },
      { name: 'group_id', fieldType: 'string', defaultValue: '' },
      { name: 'type_id', fieldType: 'string', defaultValue: '' },
      { name: 'group_template_id', fieldType: 'string', defaultValue: '' },
      { name: 'product_template_id', fieldType: 'string', defaultValue: '' },
      { name: 'pricing_template_id', fieldType: 'string', defaultValue: '' },
      { name: 'created_by', fieldType: 'string', defaultValue: '' },
      { name: 'deleted_at', fieldType: 'datetime', optional: true },
    ],
    indexes: [
      { fields: ['status'] },
      { fields: ['group_id'] },
      { fields: ['created_by'] },
    ],
  },
  {
    name: 'block_products_groups',
    fields: [
      { name: 'name', fieldType: 'string' },
      { name: 'description', fieldType: 'string', defaultValue: '' },
      { name: 'template_id', fieldType: 'string', defaultValue: '' },
      { name: 'group_template_id', fieldType: 'string', defaultValue: '' },
      { name: 'user_id', fieldType: 'string', defaultValue: '' },
      { name: 'status', fieldType: 'string', defaultValue: 'active' },
      { name: 'created_by', fieldType: 'string', defaultValue: '' },
    ],
  },
  {
    name: 'block_products_types',
    fields: [
      { name: 'name', fieldType: 'string' },
      { name: 'description', fieldType: 'string', defaultValue: '' },
      { name: 'is_system', fieldType: 'bool', defaultValue: 'false' },
    ],
  },
  {
    name: 'block_products_pricing_templates',
    fields: [
      { name: 'name', fieldType: 'string' },
      { name: 'price_formula', fieldType: 'string', defaultValue: '' },
      { name: 'template_data', fieldType: 'json', defaultValue: '{}' },
    ],
  },
  {
    name: 'block_products_purchases',
    fields: [
      { name: 'user_id', fieldType: 'string', reference: 'auth_users.id' },
      { name: 'status', fieldType: 'string', defaultValue: 'pending' },
      { name: 'total_cents', fieldType: 'int', defaultValue: '0' },
      { name: 'amount_cents', fieldType: 'int', defaultValue: '0' },
      { name: 'currency', fieldType: 'string', defaultValue: 'USD' },
      { name: 'provider', fieldType: 'string', defaultValue: 'manual' },
      { name: 'metadata', fieldType: 'json', defaultValue: '{}' },
      { name: 'stripe_payment_intent_id', fieldType: 'string', defaultValue: '' },
      { name: 'refunded_at', fieldType: 'datetime', optional: true },
      { name: 'refunded_by', fieldType: 'string', defaultValue: '' },
      { name: 'refund_reason', fieldType: 'string', defaultValue: '' },
      { name: 'payment_at', fieldType: 'datetime', optional: true },
    ],
    indexes: [
      { fields: ['user_id'] },
      { fields: ['status'] },
    ],
  },
  {
    name: 'block_products_line_items',
    fields: [
      { name: 'purchase_id', fieldType: 'string' },
      { name: 'product_id', fieldType: 'string' },
      { name: 'product_name', fieldType: 'string', defaultValue: '' },
      { name: 'quantity', fieldType: 'int', defaultValue: '1' },
      { name: 'unit_price', fieldType: 'float', defaultValue: '0' },
      { name: 'total_price', fieldType: 'float', defaultValue: '0' },
      { name: 'variables', fieldType: 'json', defaultValue: '{}' },
    ],
    indexes: [{ fields: ['purchase_id'] }],
  },
  {
    name: 'block_products_group_templates',
    fields: [
      { name: 'name', fieldType: 'string' },
      { name: 'display_name', fieldType: 'string', defaultValue: '' },
    ],
  },
  {
    name: 'block_products_product_templates',
    fields: [
      { name: 'name', fieldType: 'string' },
      { name: 'display_name', fieldType: 'string', defaultValue: '' },
    ],
  },
  {
    name: 'block_products_variables',
    fields: [
      { name: 'name', fieldType: 'string' },
      { name: 'var_type', fieldType: 'string', defaultValue: 'number' },
      { name: 'default_value', fieldType: 'string', optional: true },
      { name: 'scope', fieldType: 'string', defaultValue: 'system' },
      { name: 'product_id', fieldType: 'string', defaultValue: '' },
    ],
  },
  // LEGAL PAGES
  {
    name: 'block_legalpages_legal_documents',
    fields: [
      { name: 'doc_type', fieldType: 'string' },
      { name: 'title', fieldType: 'string' },
      { name: 'content', fieldType: 'text', defaultValue: '' },
      { name: 'status', fieldType: 'string', defaultValue: 'draft' },
      { name: 'version', fieldType: 'int', defaultValue: '1' },
      { name: 'created_by', fieldType: 'string', defaultValue: '' },
      { name: 'published_at', fieldType: 'datetime', optional: true },
    ],
    indexes: [{ fields: ['doc_type', 'status'] }],
  },
  // DEPLOYMENTS
  {
    name: 'block_deployments',
    fields: [
      { name: 'user_id', fieldType: 'string', reference: 'auth_users.id' },
      { name: 'name', fieldType: 'string' },
      { name: 'slug', fieldType: 'string', defaultValue: '' },
      { name: 'status', fieldType: 'string', defaultValue: 'pending' },
      { name: 'config', fieldType: 'json', defaultValue: '{}' },
      { name: 'plan_id', fieldType: 'string', defaultValue: '' },
      { name: 'purchase_id', fieldType: 'string', defaultValue: '' },
      { name: 'tenant_id', fieldType: 'string', defaultValue: '' },
      { name: 'subdomain', fieldType: 'string', defaultValue: '' },
      { name: 'provision_error', fieldType: 'string', optional: true },
      { name: 'deprovision_error', fieldType: 'string', optional: true },
      { name: 'deleted_at', fieldType: 'datetime', optional: true },
    ],
    indexes: [
      { fields: ['user_id'] },
      { fields: ['status'] },
    ],
  },
];

// ---------------------------------------------------------------------------
// Platform tables (not block-owned — managed by the worker itself)
// ---------------------------------------------------------------------------

const PLATFORM_TABLES: string[] = [
  // Usage tracking per tenant per month
  `CREATE TABLE IF NOT EXISTS tenant_usage (
  id TEXT PRIMARY KEY,
  tenant_id TEXT NOT NULL,
  month TEXT NOT NULL,
  requests INTEGER NOT NULL DEFAULT 0,
  r2_bytes INTEGER NOT NULL DEFAULT 0,
  addon_requests INTEGER NOT NULL DEFAULT 0,
  addon_r2_bytes INTEGER NOT NULL DEFAULT 0,
  addon_d1_bytes INTEGER NOT NULL DEFAULT 0,
  UNIQUE (tenant_id, month)
)`,
  `CREATE INDEX IF NOT EXISTS idx_tenant_usage_tenant_month ON tenant_usage (tenant_id, month)`,
  // Subscriptions
  `CREATE TABLE IF NOT EXISTS subscriptions (
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
)`,
  `CREATE INDEX IF NOT EXISTS idx_subscriptions_user_id ON subscriptions (user_id)`,
  `CREATE INDEX IF NOT EXISTS idx_subscriptions_stripe_sub ON subscriptions (stripe_subscription_id)`,
];

/**
 * Generate all SQL statements from block collection declarations.
 */
export function generateAllStatements(): string[] {
  const statements: string[] = [];
  for (const col of BLOCK_COLLECTIONS) {
    statements.push(...generateCreateTable(col));
  }
  // Platform tables
  statements.push(...PLATFORM_TABLES);
  return statements;
}

// ---------------------------------------------------------------------------
// CLI: npx tsx worker/generate-migration.ts > migrations/0001_init.sql
// ---------------------------------------------------------------------------

const isMain = typeof process !== 'undefined' && process.argv[1]?.endsWith('generate-migration.ts');
if (isMain) {
  const header = `-- Auto-generated from block collection declarations.\n-- Do not edit manually — regenerate with:\n--   npx tsx worker/generate-migration.ts > migrations/0001_init.sql\n`;
  const sql = generateAllStatements().map(s => s + ';').join('\n\n');
  console.log(header);
  console.log(sql);
}
