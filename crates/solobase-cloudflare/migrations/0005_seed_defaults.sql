-- Default IAM roles
INSERT OR IGNORE INTO iam_roles (id, name, description, is_system) VALUES ('role-admin', 'admin', 'Full platform access', 1);
INSERT OR IGNORE INTO iam_roles (id, name, description, is_system) VALUES ('role-user', 'user', 'Standard user access', 1);

-- Default settings
INSERT OR IGNORE INTO settings (id, key, value) VALUES ('setting-app-name', 'APP_NAME', 'Solobase');
INSERT OR IGNORE INTO settings (id, key, value) VALUES ('setting-allow-signup', 'ALLOW_SIGNUP', 'true');
