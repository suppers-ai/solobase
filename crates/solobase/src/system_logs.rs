//! Background system log writer for request telemetry.
//!
//! Uses a tokio mpsc channel to bridge sync observability hooks to async DB writes.
//! The writer batches inserts using a dedicated SQLite connection (no contention
//! with the WAFER runtime's connection).

use tokio::sync::mpsc;

/// A single request log entry extracted from observability hooks.
pub struct SystemLogEntry {
    pub flow_id: String,
    pub method: String,
    pub path: String,
    pub status: String,
    pub status_code: u16,
    pub duration_ms: u64,
    pub error_message: String,
    pub client_ip: String,
    pub user_id: String,
}

/// Sender handle — cloneable, passed into observability hooks.
pub type SystemLogSender = mpsc::UnboundedSender<SystemLogEntry>;

/// Spawn a background task that drains the channel and writes to the DB.
/// Returns the sender handle.
pub fn spawn_writer(db_path: String) -> SystemLogSender {
    let (tx, mut rx) = mpsc::unbounded_channel::<SystemLogEntry>();

    tokio::spawn(async move {
        // Open a dedicated SQLite connection for log writing
        let conn = match rusqlite::Connection::open(&db_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("system_log_writer: failed to open DB: {e}");
                return;
            }
        };

        // Ensure WAL mode for concurrent read/write
        let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");

        // Ensure table exists (may not be created yet by WAFER lifecycle)
        let _ = conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS request_logs (
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
            )"
        );

        let mut batch = Vec::with_capacity(64);
        loop {
            // Wait for the first entry
            match rx.recv().await {
                Some(entry) => batch.push(entry),
                None => break,
            }
            // Drain any additional buffered entries (up to 64)
            while batch.len() < 64 {
                match rx.try_recv() {
                    Ok(entry) => batch.push(entry),
                    Err(_) => break,
                }
            }
            let count = batch.len();
            write_batch(&conn, &batch);
            tracing::debug!(count, "system_log_writer: wrote batch");
            batch.clear();
        }
    });

    tx
}

fn write_batch(conn: &rusqlite::Connection, entries: &[SystemLogEntry]) {
    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("system_log_writer: transaction error: {e}");
            return;
        }
    };
    for entry in entries {
        let id = format!("rlog_{}", uuid::Uuid::new_v4());
        let _ = tx.execute(
            "INSERT INTO request_logs (id, flow_id, method, path, status, status_code, duration_ms, error_message, client_ip, user_id, created_at, updated_at) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,datetime('now'),datetime('now'))",
            rusqlite::params![
                id, entry.flow_id, entry.method, entry.path,
                entry.status, entry.status_code, entry.duration_ms,
                entry.error_message, entry.client_ip, entry.user_id
            ],
        );
    }
    let _ = tx.commit();
}
