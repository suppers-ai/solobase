CREATE TABLE IF NOT EXISTS tenant_usage (
  id TEXT PRIMARY KEY,
  tenant_id TEXT NOT NULL,
  month TEXT NOT NULL,
  requests INTEGER NOT NULL DEFAULT 0,
  r2_bytes INTEGER NOT NULL DEFAULT 0,
  addon_requests INTEGER NOT NULL DEFAULT 0,
  addon_r2_bytes INTEGER NOT NULL DEFAULT 0,
  addon_d1_bytes INTEGER NOT NULL DEFAULT 0,
  UNIQUE (tenant_id, month)
);
CREATE INDEX IF NOT EXISTS idx_tenant_usage_tenant_month ON tenant_usage (tenant_id, month);
