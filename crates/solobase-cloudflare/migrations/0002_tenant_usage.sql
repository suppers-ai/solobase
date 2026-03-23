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
