-- IAM tables for roles, user roles, policies, and audit logs

CREATE TABLE IF NOT EXISTS iam_roles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT,
    description TEXT,
    type TEXT,
    metadata TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS iam_user_roles (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    role_id TEXT NOT NULL,
    granted_by TEXT,
    granted_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME,
    UNIQUE(user_id, role_id)
);

CREATE INDEX IF NOT EXISTS idx_iam_user_roles_user_id ON iam_user_roles(user_id);
CREATE INDEX IF NOT EXISTS idx_iam_user_roles_role_id ON iam_user_roles(role_id);

CREATE TABLE IF NOT EXISTS iam_policies (
    id TEXT PRIMARY KEY,
    ptype TEXT NOT NULL,
    v0 TEXT,
    v1 TEXT,
    v2 TEXT,
    v3 TEXT,
    v4 TEXT,
    v5 TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_iam_policies_ptype ON iam_policies(ptype);
CREATE INDEX IF NOT EXISTS idx_iam_policies_v0 ON iam_policies(v0);

CREATE TABLE IF NOT EXISTS iam_audit_logs (
    id TEXT PRIMARY KEY,
    user_id TEXT,
    action TEXT,
    resource TEXT,
    result TEXT,
    reason TEXT,
    ip_address TEXT,
    user_agent TEXT,
    metadata TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_iam_audit_logs_user_id ON iam_audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_iam_audit_logs_created_at ON iam_audit_logs(created_at);
