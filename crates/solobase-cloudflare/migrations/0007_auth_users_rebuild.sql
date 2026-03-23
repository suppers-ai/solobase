-- Migration: Rebuild auth_users to replace global UNIQUE(email) with
-- per-project UNIQUE(project_id, email).
--
-- SQLite cannot ALTER column constraints, so we recreate the table.

-- 1. Create new table with the correct constraints
CREATE TABLE IF NOT EXISTS auth_users_new (
  id TEXT PRIMARY KEY,
  email TEXT NOT NULL DEFAULT '',
  password_hash TEXT NOT NULL DEFAULT '',
  name TEXT NOT NULL DEFAULT '',
  disabled INTEGER DEFAULT 0,
  email_verified INTEGER DEFAULT 0,
  avatar_url TEXT NOT NULL DEFAULT '',
  oauth_provider TEXT NOT NULL DEFAULT '',
  last_login_at TEXT DEFAULT '',
  deleted_at TEXT DEFAULT '',
  project_id TEXT NOT NULL DEFAULT 'platform',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

-- 2. Copy data from old table
INSERT INTO auth_users_new (id, email, password_hash, name, disabled, avatar_url, oauth_provider, last_login_at, deleted_at, project_id, created_at, updated_at)
  SELECT id, email, password_hash, name, disabled, avatar_url, oauth_provider, last_login_at, deleted_at, COALESCE(project_id, 'platform'), created_at, updated_at
  FROM auth_users;

-- 3. Drop old table
DROP TABLE auth_users;

-- 4. Rename new table
ALTER TABLE auth_users_new RENAME TO auth_users;

-- 5. Recreate indexes with per-project uniqueness
CREATE UNIQUE INDEX idx_auth_users_email_project ON auth_users (project_id, email);
CREATE INDEX idx_auth_users_project ON auth_users (project_id);
