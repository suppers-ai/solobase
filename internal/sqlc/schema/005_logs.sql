-- Logging tables

CREATE TABLE IF NOT EXISTS sys_logs (
    id TEXT PRIMARY KEY,
    level TEXT NOT NULL,
    message TEXT NOT NULL,
    fields TEXT,
    user_id TEXT,
    trace_id TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_sys_logs_level ON sys_logs(level);
CREATE INDEX IF NOT EXISTS idx_sys_logs_user_id ON sys_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_sys_logs_trace_id ON sys_logs(trace_id);
CREATE INDEX IF NOT EXISTS idx_sys_logs_created_at ON sys_logs(created_at);

CREATE TABLE IF NOT EXISTS sys_request_logs (
    id TEXT PRIMARY KEY,
    level TEXT NOT NULL,
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    query TEXT,
    status_code INTEGER NOT NULL,
    exec_time_ms INTEGER NOT NULL,
    user_ip TEXT NOT NULL,
    user_agent TEXT,
    user_id TEXT,
    trace_id TEXT,
    error TEXT,
    request_body TEXT,
    response_body TEXT,
    headers TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_sys_request_logs_method ON sys_request_logs(method);
CREATE INDEX IF NOT EXISTS idx_sys_request_logs_path ON sys_request_logs(path);
CREATE INDEX IF NOT EXISTS idx_sys_request_logs_status_code ON sys_request_logs(status_code);
CREATE INDEX IF NOT EXISTS idx_sys_request_logs_user_id ON sys_request_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_sys_request_logs_created_at ON sys_request_logs(created_at);
