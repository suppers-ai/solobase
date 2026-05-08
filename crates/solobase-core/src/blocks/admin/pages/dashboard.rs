use maud::html;
use wafer_core::clients::database::{self as db, Filter, FilterOp, ListOptions, SortField};
use wafer_run::{context::Context, types::*, OutputStream};
use wafer_sql_utils::{aggregate, query, value::sea_values_to_json, Backend};

use super::{admin_page, crumb};
use crate::{
    blocks::{
        admin::{AUDIT_LOGS_COLLECTION as AUDIT_LOGS, REQUEST_LOGS_COLLECTION as REQUEST_LOGS},
        auth::USERS_COLLECTION as USERS,
    },
    ui::{
        shell::Topbar,
        templates::{dashboard_page, PageHeader, StatTile},
        SiteConfig, UserInfo,
    },
};

pub async fn dashboard(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // Total users
    let user_count = db::list(
        ctx,
        USERS,
        &ListOptions {
            filters: vec![Filter {
                field: "deleted_at".into(),
                operator: FilterOp::IsNull,
                value: serde_json::Value::Null,
            }],
            limit: 1,
            ..Default::default()
        },
    )
    .await
    .map(|r| r.total_count)
    .unwrap_or(0);

    // New users today
    let today_start = format!("{today}T00:00:00");
    let (sql, vals) = aggregate::build_count(
        USERS,
        &[
            Filter {
                field: "deleted_at".into(),
                operator: FilterOp::IsNull,
                value: serde_json::Value::Null,
            },
            Filter {
                field: "created_at".into(),
                operator: FilterOp::GreaterEqual,
                value: serde_json::json!(&today_start),
            },
        ],
        Backend::Sqlite,
    );
    let new_users_today = db::query_raw(ctx, &sql, &sea_values_to_json(vals))
        .await
        .ok()
        .and_then(|r| {
            r.first()
                .and_then(|r| r.data.get("cnt").and_then(|v| v.as_i64()))
        })
        .unwrap_or(0);

    // Requests today
    let (sql, vals) = aggregate::build_count(
        REQUEST_LOGS,
        &[Filter {
            field: "created_at".into(),
            operator: FilterOp::GreaterEqual,
            value: serde_json::json!(&today_start),
        }],
        Backend::Sqlite,
    );
    let requests_today = db::query_raw(ctx, &sql, &sea_values_to_json(vals))
        .await
        .ok()
        .and_then(|r| {
            r.first()
                .and_then(|r| r.data.get("cnt").and_then(|v| v.as_i64()))
        })
        .unwrap_or(0);

    // Errors today
    let (sql, vals) = aggregate::build_count(
        REQUEST_LOGS,
        &[
            Filter {
                field: "status".into(),
                operator: FilterOp::Equal,
                value: serde_json::json!("ERROR"),
            },
            Filter {
                field: "created_at".into(),
                operator: FilterOp::GreaterEqual,
                value: serde_json::json!(&today_start),
            },
        ],
        Backend::Sqlite,
    );
    let errors_today = db::query_raw(ctx, &sql, &sea_values_to_json(vals))
        .await
        .ok()
        .and_then(|r| {
            r.first()
                .and_then(|r| r.data.get("cnt").and_then(|v| v.as_i64()))
        })
        .unwrap_or(0);

    // Avg response time today
    let (sql, vals) = aggregate::build_avg(
        REQUEST_LOGS,
        "duration_ms",
        &[Filter {
            field: "created_at".into(),
            operator: FilterOp::GreaterEqual,
            value: serde_json::json!(&today_start),
        }],
        Backend::Sqlite,
    );
    let avg_ms = db::query_raw(ctx, &sql, &sea_values_to_json(vals))
        .await
        .ok()
        .and_then(|r| {
            r.first()
                .and_then(|r| r.data.get("avg_val").and_then(|v| v.as_f64()))
        })
        .unwrap_or(0.0);

    // Recent users (last 5 logins)
    let (sql, vals) = query::build_select_columns(
        USERS,
        &["id", "email", "created_at"],
        &ListOptions {
            filters: vec![Filter {
                field: "deleted_at".into(),
                operator: FilterOp::IsNull,
                value: serde_json::Value::Null,
            }],
            sort: vec![SortField {
                field: "created_at".into(),
                desc: true,
            }],
            limit: 5,
            ..Default::default()
        },
        None,
        Backend::Sqlite,
    );
    let recent_users = db::query_raw(ctx, &sql, &sea_values_to_json(vals))
        .await
        .unwrap_or_default();

    // Recent audit logs (last 5)
    let (sql, vals) = query::build_select_columns(
        AUDIT_LOGS,
        &["action", "resource", "user_id", "created_at"],
        &ListOptions {
            sort: vec![SortField {
                field: "created_at".into(),
                desc: true,
            }],
            limit: 5,
            ..Default::default()
        },
        None,
        Backend::Sqlite,
    );
    let recent_audit = db::query_raw(ctx, &sql, &sea_values_to_json(vals))
        .await
        .unwrap_or_default();

    // Recent errors (last 5)
    let or_cond = sea_query::Cond::any()
        .add(sea_query::Expr::col(wafer_sql_utils::ident::DynCol("status".into())).eq("ERROR"))
        .add(sea_query::Expr::col(wafer_sql_utils::ident::DynCol("status_code".into())).gte(400));
    let (sql, vals) = query::build_select_columns(
        REQUEST_LOGS,
        &["status_code", "method", "path", "duration_ms", "created_at"],
        &ListOptions {
            sort: vec![SortField {
                field: "created_at".into(),
                desc: true,
            }],
            limit: 5,
            ..Default::default()
        },
        Some(or_cond),
        Backend::Sqlite,
    );
    let recent_errors = db::query_raw(ctx, &sql, &sea_values_to_json(vals))
        .await
        .unwrap_or_default();

    let user_count_str = user_count.to_string();
    let new_users_str = new_users_today.to_string();
    let requests_str = requests_today.to_string();
    let errors_str = errors_today.to_string();
    let avg_ms_str = format!("{:.0}ms", avg_ms);

    let stats = vec![
        StatTile {
            label: "Total Users",
            value: &user_count_str,
            trend: None,
        },
        StatTile {
            label: "New Today",
            value: &new_users_str,
            trend: None,
        },
        StatTile {
            label: "Requests Today",
            value: &requests_str,
            trend: None,
        },
        StatTile {
            label: "Errors Today",
            value: &errors_str,
            trend: None,
        },
        StatTile {
            label: "Avg Response",
            value: &avg_ms_str,
            trend: None,
        },
    ];

    let recent_users_card = html! {
        section .card {
            header .card__head {
                h3 .card__title { "Recent Users" }
                a .btn .btn-ghost .btn-sm href="/b/admin/users" { "View all" }
            }
            div .card__body {
                @if recent_users.is_empty() {
                    p .text-muted .text-sm { "No users yet" }
                } @else {
                    div .table-container {
                        table .table {
                            tbody {
                                @for record in &recent_users {
                                    @let email = record.data.get("email").and_then(|v| v.as_str()).unwrap_or("");
                                    @let created = record.data.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                                    tr {
                                        td .text-sm { (email) }
                                        td .text-muted .text-sm .text-right { (created.get(..10).unwrap_or(created)) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    let recent_activity_card = html! {
        section .card {
            header .card__head {
                h3 .card__title { "Recent Activity" }
                a .btn .btn-ghost .btn-sm href="/b/admin/logs?tab=audit" { "View all" }
            }
            div .card__body {
                @if recent_audit.is_empty() {
                    p .text-muted .text-sm { "No activity yet" }
                } @else {
                    div .table-container {
                        table .table {
                            tbody {
                                @for record in &recent_audit {
                                    @let action = record.data.get("action").and_then(|v| v.as_str()).unwrap_or("");
                                    @let resource = record.data.get("resource").and_then(|v| v.as_str()).unwrap_or("");
                                    @let created = record.data.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                                    tr {
                                        td { span .badge .badge-info .text-xs { (action) } }
                                        td .text-sm { (resource) }
                                        td .text-muted .text-sm .text-right { (created.get(..19).unwrap_or(created)) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    let recent_errors_card = if recent_errors.is_empty() {
        None
    } else {
        Some(html! {
            section .card {
                header .card__head {
                    h3 .card__title { "Recent Errors" }
                    a .btn .btn-ghost .btn-sm href="/b/admin/logs" { "View all" }
                }
                div .card__body {
                    div .table-container {
                        table .table {
                            thead {
                                tr {
                                    th { "Status" }
                                    th { "Method" }
                                    th { "Path" }
                                    th { "Duration" }
                                    th { "Time" }
                                }
                            }
                            tbody {
                                @for record in &recent_errors {
                                    @let code = record.data.get("status_code").and_then(|v| v.as_i64()).unwrap_or(0);
                                    @let method = record.data.get("method").and_then(|v| v.as_str()).unwrap_or("");
                                    @let path = record.data.get("path").and_then(|v| v.as_str()).unwrap_or("");
                                    @let duration = record.data.get("duration_ms").and_then(|v| v.as_i64()).unwrap_or(0);
                                    @let created = record.data.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                                    tr {
                                        td {
                                            span .badge .(if code >= 500 { "badge-danger" } else { "badge-warning" }) { (code) }
                                        }
                                        td .text-sm .font-medium { (method.to_uppercase()) }
                                        td .text-sm { (path) }
                                        td .text-muted .text-sm { (duration) "ms" }
                                        td .text-muted .text-sm { (created.get(..19).unwrap_or(created)) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        })
    };

    let body = dashboard_page(
        PageHeader {
            title: "",
            subtitle: None,
            primary_action: None,
        },
        stats,
        recent_users_card,
        recent_activity_card,
        recent_errors_card,
        None,
    );

    admin_page(
        "Dashboard",
        &config,
        "/b/admin/",
        user.as_ref(),
        Topbar {
            crumbs: crumb("Dashboard"),
            primary_action: None,
            subtitle: Some("System overview"),
            show_palette: true,
        },
        body,
        msg,
    )
}
