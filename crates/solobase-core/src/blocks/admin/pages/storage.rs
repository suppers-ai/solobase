use maud::{html, Markup};
use wafer_core::clients::database::{self as db, ListOptions, SortField};
use wafer_run::{context::Context, types::*, OutputStream};
use wafer_sql_utils::{query, value::sea_values_to_json, Backend};

use super::admin_page;
use crate::{
    blocks::admin::{
        STORAGE_ACCESS_LOGS_COLLECTION as STORAGE_ACCESS_LOGS,
        STORAGE_RULES_COLLECTION as STORAGE_RULES,
    },
    ui::{components, icons, SiteConfig, UserInfo},
};

pub async fn storage_page(ctx: &dyn Context, msg: &Message) -> OutputStream {
    let config = SiteConfig::load(ctx).await;
    let user = UserInfo::from_message(msg);
    let tab = msg.query("tab");
    let active_tab = match tab {
        "rules" => "rules",
        _ => "logs",
    };

    let content = html! {
        (components::page_header(
            "Storage",
            Some("Per-block storage isolation and access rules"),
            Some(html! {
                button .btn .btn-secondary .btn-sm
                    hx-get={"/b/admin/storage?tab=" (active_tab)}
                    hx-target="#content"
                { (icons::refresh_cw()) " Refresh" }
            })
        ))

        div .tabs {
            a .tab .(if active_tab == "logs" { "active" } else { "" })
                href="/b/admin/storage"
                hx-get="/b/admin/storage"
                hx-target="#content"
                hx-push-url="true"
            { (icons::eye()) " Access Logs" }
        }

        @if active_tab == "rules" {
            div .card .mt-4 style="background:#f0f9ff;border-color:#bae6fd" {
                p style="padding:12px;margin:0;font-size:13px" {
                    (icons::info()) " Storage permissions have moved to the "
                    a href="/b/admin/permissions?tab=storage" { "Permissions" }
                    " page."
                }
            }
        }

        div #storage-tab-content {
            @if active_tab == "rules" {
                // rules tab still renders content but with banner above
                (storage_rules_tab(ctx, msg).await)
            } @else {
                (storage_logs_tab(ctx, msg).await)
            }
        }
    };

    admin_page(
        "Storage",
        &config,
        "/b/admin/storage",
        user.as_ref(),
        content,
        msg,
    )
}

async fn storage_logs_tab(ctx: &dyn Context, _msg: &Message) -> Markup {
    let (sql, vals) = query::build_select_columns(
        STORAGE_ACCESS_LOGS,
        &["source_block", "operation", "path", "status", "created_at"],
        &ListOptions {
            sort: vec![SortField {
                field: "created_at".into(),
                desc: true,
            }],
            limit: 100,
            ..Default::default()
        },
        None,
        Backend::Sqlite,
    );
    let logs = db::query_raw(ctx, &sql, &sea_values_to_json(vals))
        .await
        .unwrap_or_default();

    html! {
        p .text-muted style="margin-bottom:16px" {
            "Recent storage access by blocks. Each block is isolated to "
            code { "/storage/{block-name}/" }
            "."
        }

        div .table-container {
            table .table {
                thead {
                    tr {
                        th { "Block" }
                        th { "Operation" }
                        th { "Path" }
                        th { "Status" }
                        th { "Time" }
                    }
                }
                tbody {
                    @if logs.is_empty() {
                        tr {
                            td colspan="5" .text-center .text-muted style="padding: 2rem;" {
                                "No storage access logs yet."
                            }
                        }
                    }
                    @for log in &logs {
                        @let source = log.data.get("source_block").and_then(|v| v.as_str()).unwrap_or("");
                        @let op = log.data.get("operation").and_then(|v| v.as_str()).unwrap_or("");
                        @let path = log.data.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        @let status = log.data.get("status").and_then(|v| v.as_str()).unwrap_or("");
                        @let created = log.data.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                        tr {
                            td {
                                @if !source.is_empty() {
                                    span .badge .badge-info { (source) }
                                }
                            }
                            td .text-sm style="font-family:monospace" { (op) }
                            td .text-sm style="font-family:monospace" { (path) }
                            td .text-sm {
                                @if status.starts_with("BLOCKED") {
                                    span .badge .badge-danger { (status) }
                                } @else if status.starts_with("ERROR") {
                                    span .badge .badge-warning { (status) }
                                } @else {
                                    span .text-muted { (status) }
                                }
                            }
                            td .text-muted .text-sm { (created.get(..19).unwrap_or(created)) }
                        }
                    }
                }
            }
        }
    }
}

pub(crate) async fn storage_rules_tab(ctx: &dyn Context, _msg: &Message) -> Markup {
    let blocks = ctx.registered_blocks();
    let block_names: Vec<&str> = blocks.iter().map(|b| b.name.as_str()).collect();

    let rules = db::list_all(ctx, STORAGE_RULES, vec![])
        .await
        .unwrap_or_default();

    // Collect block names that use storage (service blocks don't need their own namespace)
    let registered = ctx.registered_blocks();
    let storage_blocks: Vec<&str> = registered
        .iter()
        .filter(|b| b.category != wafer_run::BlockCategory::Service && !b.name.is_empty())
        .map(|b| b.name.as_str())
        .collect();

    html! {
        div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:16px" {
            p .text-muted style="margin:0" {
                "Each block is isolated to its own storage namespace. "
                "Add rules below to grant cross-block access."
            }
            button .btn .btn-primary .btn-sm
                onclick="openModal('add-storage-rule-modal')"
            { (icons::plus()) " Add Rule" }
        }

        // Default isolation rules (built-in)
        h3 style="font-size:14px;margin-bottom:8px;color:#6b7280" { "Default (built-in)" }
        div .table-container style="margin-bottom:24px" {
            table .table {
                thead {
                    tr {
                        th { "Type" }
                        th { "Block" }
                        th { "Storage Path" }
                        th { "Access" }
                    }
                }
                tbody {
                    @for block_name in &storage_blocks {
                        tr {
                            td { span .badge .badge-success { "Allow" } }
                            td .text-sm { span .badge .badge-info { (block_name) } }
                            td .text-sm style="font-family:monospace" { (block_name) "/*" }
                            td .text-sm { span .badge .badge-secondary { "Read/Write" } }
                        }
                    }
                }
            }
        }

        // Custom rules
        h3 style="font-size:14px;margin-bottom:8px;color:#6b7280" { "Custom cross-block rules" }
        div .table-container {
            table .table {
                thead {
                    tr {
                        th { "Block" }
                        th { "Type" }
                        th { "Storage Path" }
                        th { "Access" }
                        th { "Priority" }
                        th style="width:80px" { "" }
                    }
                }
                tbody {
                    @if rules.is_empty() {
                        tr {
                            td colspan="6" .text-center .text-muted style="padding: 2rem;" {
                                "No cross-block rules configured. Blocks can only access their own files by default."
                            }
                        }
                    }
                    @for rule in &rules {
                        @let id = &rule.id;
                        @let rule_type = rule.data.get("rule_type").and_then(|v| v.as_str()).unwrap_or("");
                        @let source = rule.data.get("source_block").and_then(|v| v.as_str()).unwrap_or("*");
                        @let target = rule.data.get("target_path").and_then(|v| v.as_str()).unwrap_or("");
                        @let access = rule.data.get("access").and_then(|v| v.as_str()).unwrap_or("readwrite");
                        @let priority = rule.data.get("priority").and_then(|v| v.as_i64()).unwrap_or(0);
                        tr {
                            td {
                                @if source == "*" || source.is_empty() {
                                    span .badge .badge-warning { "All blocks" }
                                } @else {
                                    code { (source) }
                                }
                            }
                            td {
                                @if rule_type == "block" {
                                    span .badge .badge-danger { "Deny" }
                                } @else {
                                    span .badge .badge-success { "Allow" }
                                }
                            }
                            td .text-sm style="font-family:monospace" { (target) }
                            td .text-sm {
                                @if access == "read" {
                                    span .badge .badge-info { "Read" }
                                } @else if access == "write" {
                                    span .badge .badge-warning { "Write" }
                                } @else {
                                    span .badge .badge-secondary { "Read/Write" }
                                }
                            }
                            td .text-muted .text-sm { (priority) }
                            td {
                                button .btn .btn-danger .btn-sm
                                    hx-delete={"/b/admin/storage/rules/" (id)}
                                    hx-target="#content"
                                    hx-confirm="Delete this rule?"
                                { (icons::trash()) }
                            }
                        }
                    }
                }
            }
        }

        // Add rule modal
        (components::modal("add-storage-rule-modal", "Add Storage Rule", html! {
            form hx-post="/b/admin/storage/rules" hx-target="#content" {
                div .form-group {
                    label .form-label for="source_block" { "Which block?" }
                    select .form-input name="source_block" {
                        option value="*" { "All blocks" }
                        @for name in &block_names {
                            option value=(name) { (name) }
                        }
                    }
                    p .text-muted style="font-size:12px;margin-top:4px" {
                        "The block this rule applies to."
                    }
                }
                div .form-group {
                    label .form-label for="rule_type" { "Allow or Deny?" }
                    select .form-input name="rule_type" {
                        option value="allow" { "Allow \u{2014} let this block access the storage path" }
                        option value="block" { "Deny \u{2014} block this block from the storage path" }
                    }
                }
                div .form-group {
                    label .form-label for="target_path" { "Which storage path?" }
                    input .form-input type="text" name="target_path"
                        placeholder="e.g. wafer-run/web/public/*" required;
                    p .text-muted style="font-size:12px;margin-top:4px" {
                        "Each block's files are stored under its name. Use " code { "*" } " as wildcard. "
                        "Examples: " code { "wafer-run/web/*" } ", " code { "suppers-ai/files/uploads/*" }
                    }
                }
                div .form-group {
                    label .form-label for="access" { "Read, write, or both?" }
                    select .form-input #access name="access" {
                        option value="readwrite" { "Read & Write" }
                        option value="read" { "Read only" }
                        option value="write" { "Write only" }
                    }
                }
                div .form-group {
                    label .form-label for="priority" { "Priority" }
                    input .form-input type="number" #priority name="priority" value="0";
                    p .text-muted style="font-size:12px;margin-top:4px" { "Higher priority rules are evaluated first" }
                }
                div .form-actions {
                    button .btn .btn-secondary type="button" onclick="closeModal('add-storage-rule-modal')" { "Cancel" }
                    button .btn .btn-primary type="submit" { "Add Rule" }
                }
            }
        }))
    }
}
