package admin

import (
	"context"
	"strings"

	waffle "github.com/suppers-ai/waffle-go"
)

func (b *AdminBlock) registerDatabaseRoutes() {
	b.router.Retrieve("/admin/database/info", b.handleGetDBInfo)
	b.router.Retrieve("/admin/database/tables", b.handleGetTables)
	b.router.Retrieve("/admin/database/tables/{table}/columns", b.handleGetColumns)
	b.router.Create("/admin/database/query", b.handleExecuteQuery)
}

func (b *AdminBlock) handleGetDBInfo(_ waffle.Context, msg *waffle.Message) waffle.Result {
	if b.db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database not available")
	}
	return waffle.JSONRespond(msg, 200, map[string]any{
		"type":    "SQLite",
		"version": "3.x",
		"status":  "connected",
	})
}

func (b *AdminBlock) handleGetTables(_ waffle.Context, msg *waffle.Message) waffle.Result {
	if b.db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database not available")
	}

	ctx := context.Background()
	records, err := b.db.QueryRaw(ctx, `
		SELECT name, type FROM sqlite_master
		WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%'
		ORDER BY name
	`)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to fetch tables")
	}

	var tables []map[string]any
	for _, rec := range records {
		name, _ := rec.Data["name"].(string)
		tableType, _ := rec.Data["type"].(string)

		var count int
		if isTableAllowed(name) {
			count, _ = b.db.Count(ctx, name, nil)
		}

		tables = append(tables, map[string]any{
			"name":       name,
			"schema":     "main",
			"type":       tableType,
			"rows_count": count,
		})
	}
	return waffle.JSONRespond(msg, 200, tables)
}

func (b *AdminBlock) handleGetColumns(_ waffle.Context, msg *waffle.Message) waffle.Result {
	if b.db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database not available")
	}

	tableName := msg.Var("table")
	if !isTableAllowed(tableName) {
		return waffle.Error(msg, 400, "bad_request", "Table not allowed")
	}

	ctx := context.Background()
	records, err := b.db.QueryRaw(ctx, `PRAGMA table_info("`+tableName+`")`)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to fetch columns")
	}

	var columns []map[string]any
	for _, rec := range records {
		name, _ := rec.Data["name"].(string)
		dataType, _ := rec.Data["type"].(string)
		notNull := toInt64(rec.Data["notnull"]) != 0
		defaultValue := rec.Data["dflt_value"]
		pk := toInt64(rec.Data["pk"])

		columns = append(columns, map[string]any{
			"name":        name,
			"type":        dataType,
			"nullable":    !notNull,
			"has_default": defaultValue != nil,
			"is_primary":  pk > 0,
		})
	}
	return waffle.JSONRespond(msg, 200, columns)
}

func (b *AdminBlock) handleExecuteQuery(_ waffle.Context, msg *waffle.Message) waffle.Result {
	if b.db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database not available")
	}

	var body struct {
		Query string `json:"query"`
	}
	if err := msg.Decode(&body); err != nil {
		return waffle.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	queryUpper := strings.ToUpper(strings.TrimSpace(body.Query))
	if !strings.HasPrefix(queryUpper, "SELECT") {
		return waffle.JSONRespond(msg, 400, map[string]any{
			"error": "Only SELECT queries are allowed",
		})
	}

	for _, kw := range []string{"INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "CREATE", "TRUNCATE", "ATTACH", "DETACH", "PRAGMA", "VACUUM", "REINDEX"} {
		if strings.Contains(queryUpper, kw) {
			return waffle.JSONRespond(msg, 400, map[string]any{
				"error": "Query contains disallowed keywords",
			})
		}
	}

	ctx := context.Background()
	records, err := b.db.QueryRaw(ctx, body.Query)
	if err != nil {
		return waffle.JSONRespond(msg, 200, map[string]any{
			"error": err.Error(),
		})
	}

	var columns []string
	var result [][]any

	if len(records) > 0 {
		for col := range records[0].Data {
			columns = append(columns, col)
		}
		for _, rec := range records {
			row := make([]any, len(columns))
			for i, col := range columns {
				row[i] = rec.Data[col]
			}
			result = append(result, row)
		}
	}

	return waffle.JSONRespond(msg, 200, map[string]any{
		"columns": columns,
		"rows":    result,
	})
}

func isTableAllowed(name string) bool {
	if strings.HasPrefix(name, "ct_") || strings.HasPrefix(name, "ext_") {
		return true
	}
	allowed := map[string]bool{
		"users": true, "auth_users": true, "auth_tokens": true, "api_keys": true,
		"settings": true, "sys_logs": true, "sys_request_logs": true, "sys_message_logs": true,
		"iam_roles": true, "iam_user_roles": true, "iam_policies": true, "iam_audit_logs": true,
		"storage_buckets": true, "storage_objects": true,
		"storage_download_tokens": true, "storage_upload_tokens": true,
		"custom_table_definitions": true, "custom_table_migrations": true,
	}
	return allowed[name]
}
