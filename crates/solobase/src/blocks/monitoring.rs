use std::sync::{Arc, RwLock};
use wafer_run::block::{Block, BlockInfo};
use wafer_run::context::Context;
use wafer_run::types::*;
use wafer_run::helpers::*;
use wafer_run::services::database::{DatabaseService, ListOptions, SortField};

pub struct MonitoringBlock {
    stats: Arc<RwLock<MonitoringStats>>,
}

#[derive(Clone, serde::Serialize)]
struct MonitoringStats {
    uptime_seconds: u64,
    start_time: String,
    total_requests: u64,
    active_connections: u64,
    memory_usage_bytes: u64,
    cpu_usage_percent: f64,
    requests_per_minute: f64,
    error_rate: f64,
    avg_response_ms: f64,
}

impl MonitoringBlock {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(MonitoringStats {
                uptime_seconds: 0,
                start_time: chrono::Utc::now().to_rfc3339(),
                total_requests: 0,
                active_connections: 0,
                memory_usage_bytes: 0,
                cpu_usage_percent: 0.0,
                requests_per_minute: 0.0,
                error_rate: 0.0,
                avg_response_ms: 0.0,
            })),
        }
    }

    fn handle_live(&self, msg: &mut Message) -> Result_ {
        let mut stats = self.stats.write().unwrap_or_else(|p| p.into_inner());
        let start: chrono::DateTime<chrono::Utc> = stats.start_time.parse().unwrap_or_else(|_| chrono::Utc::now());
        stats.uptime_seconds = (chrono::Utc::now() - start).num_seconds().max(0) as u64;
        let s = stats.clone();
        drop(stats);
        json_respond(msg.clone(), 200, &s)
    }

    fn handle_history(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let db = match get_db(ctx) {
            Ok(db) => db,
            Err(r) => return r,
        };

        let (_, page_size, offset) = msg.pagination_params(50);
        let opts = ListOptions {
            sort: vec![SortField { field: "created_at".to_string(), desc: true }],
            limit: page_size as i64,
            offset: offset as i64,
            ..Default::default()
        };

        match db.list("monitoring_snapshots", &opts) {
            Ok(result) => json_respond(msg.clone(), 200, &result),
            Err(e) => err_internal(msg.clone(), &format!("Failed to fetch history: {e}")),
        }
    }
}

fn get_db(ctx: &dyn Context) -> Result<&Arc<dyn DatabaseService>, Result_> {
    ctx.services()
        .and_then(|s| s.database.as_ref())
        .ok_or_else(|| Result_::error(WaferError::new("unavailable", "Database service unavailable")))
}

impl Block for MonitoringBlock {
    fn info(&self) -> BlockInfo {
        BlockInfo {
            name: "monitoring-feature".to_string(),
            version: "1.0.0".to_string(),
            interface: "http.handler".to_string(),
            summary: "Monitoring dashboard with live stats and history".to_string(),
            instance_mode: InstanceMode::Singleton,
            allowed_modes: vec![InstanceMode::Singleton],
            admin_ui: None,
        }
    }

    fn handle(&self, ctx: &dyn Context, msg: &mut Message) -> Result_ {
        let path = msg.path();
        match path {
            "/admin/monitoring/live" => self.handle_live(msg),
            "/admin/monitoring/history" => self.handle_history(ctx, msg),
            _ => err_not_found(msg.clone(), "not found"),
        }
    }

    fn lifecycle(&self, _ctx: &dyn Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
