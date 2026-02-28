package admin

import (
	"bytes"
	"context"
	"encoding/csv"
	"encoding/json"
	"fmt"

	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/solobase/core/constants"
	waffle "github.com/suppers-ai/waffle-go"
	"github.com/suppers-ai/waffle-go/services/database"
)

const (
	logsCollection        = "sys_logs"
	messageLogsCollection = "sys_message_logs"
)

func (b *AdminBlock) registerLogsRoutes() {
	b.router.Retrieve("/admin/logs", b.handleGetLogs)
	b.router.Retrieve("/admin/logs/messages", b.handleGetMessageLogs)
	b.router.Retrieve("/admin/logs/stats", b.handleGetLogStats)
	b.router.Retrieve("/admin/logs/details", b.handleGetLogDetails)
	b.router.Retrieve("/admin/logs/export", b.handleExportLogs)
	b.router.Create("/admin/logs/clear", b.handleClearLogs)
}

func (b *AdminBlock) handleGetLogs(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	page, size, _ := msg.PaginationParams(constants.LogsPageSize)
	filters := buildLogFilters(msg)

	result, err := database.PaginatedList(context.Background(), db, logsCollection,
		page, size, filters,
		[]database.SortField{{Field: "created_at", Desc: true}},
	)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to fetch logs")
	}

	var responseLogs []map[string]any
	for _, r := range result.Records {
		logMap := map[string]any{
			"id":        r.ID,
			"level":     r.Data["level"],
			"message":   r.Data["message"],
			"createdAt": r.Data["created_at"],
			"userID":    r.Data["user_id"],
			"traceID":   r.Data["trace_id"],
		}
		if fields, ok := r.Data["fields"].(string); ok && fields != "" {
			var parsed map[string]any
			if err := json.Unmarshal([]byte(fields), &parsed); err == nil {
				logMap["fields"] = parsed
			}
		}
		responseLogs = append(responseLogs, logMap)
	}

	return waffle.JSONRespond(msg, 200, map[string]any{
		"logs":  responseLogs,
		"total": result.TotalCount,
		"page":  page,
		"size":  size,
	})
}

func (b *AdminBlock) handleGetMessageLogs(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	page, size, _ := msg.PaginationParams(constants.LogsPageSize)
	filters := buildMessageLogFilters(msg)

	result, err := database.PaginatedList(context.Background(), db, messageLogsCollection,
		page, size, filters,
		[]database.SortField{{Field: "created_at", Desc: true}},
	)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to fetch message logs")
	}

	var responseLogs []map[string]any
	for _, r := range result.Records {
		logMap := map[string]any{
			"id":          r.ID,
			"chainId":     r.Data["chain_id"],
			"blockName":   r.Data["block_name"],
			"messageKind": r.Data["message_kind"],
			"action":      r.Data["action"],
			"durationMs":  r.Data["duration_ms"],
			"createdAt":   r.Data["created_at"],
		}
		if v := r.Data["trace_id"]; v != nil && v != "" {
			logMap["traceId"] = v
		}
		if v := r.Data["error"]; v != nil && v != "" {
			logMap["error"] = v
		}
		if v := r.Data["user_id"]; v != nil && v != "" {
			logMap["userId"] = v
		}
		if v := r.Data["meta_snapshot"]; v != nil && v != "" {
			logMap["metaSnapshot"] = v
		}
		responseLogs = append(responseLogs, logMap)
	}

	return waffle.JSONRespond(msg, 200, map[string]any{
		"logs":  responseLogs,
		"total": result.TotalCount,
		"page":  page,
		"size":  size,
	})
}

func (b *AdminBlock) handleGetLogStats(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	timeRange := msg.Query("range")
	if timeRange == "" {
		timeRange = "24h"
	}
	cutoff := timeRangeCutoff(timeRange)

	stats := map[string]any{}
	for _, level := range []string{"debug", "info", "warn", "error"} {
		records, err := db.List(context.Background(), logsCollection, &database.ListOptions{
			Filters: []database.Filter{
				{Field: "level", Operator: database.OpEqual, Value: level},
				{Field: "created_at", Operator: database.OpGreaterEqual, Value: cutoff},
			},
			Limit: 0,
		})
		if err == nil {
			stats[level] = records.TotalCount
		}
	}

	allRecords, err := db.List(context.Background(), logsCollection, &database.ListOptions{
		Filters: []database.Filter{
			{Field: "created_at", Operator: database.OpGreaterEqual, Value: cutoff},
		},
		Limit: 0,
	})
	if err == nil {
		stats["total"] = allRecords.TotalCount
	}

	return waffle.JSONRespond(msg, 200, stats)
}

func (b *AdminBlock) handleGetLogDetails(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	logID := msg.Query("id")
	if logID == "" {
		return waffle.Error(msg, 400, "bad_request", "Log ID is required")
	}

	record, err := db.Get(context.Background(), logsCollection, logID)
	if err == nil {
		result := map[string]any{
			"id":        record.ID,
			"createdAt": record.Data["created_at"],
			"level":     record.Data["level"],
			"message":   record.Data["message"],
			"userID":    record.Data["user_id"],
			"traceID":   record.Data["trace_id"],
		}
		if fields, ok := record.Data["fields"].(string); ok && fields != "" {
			var parsed map[string]any
			if err := json.Unmarshal([]byte(fields), &parsed); err == nil {
				result["details"] = parsed
			}
		}
		return waffle.JSONRespond(msg, 200, result)
	}

	msgRecord, err := db.Get(context.Background(), messageLogsCollection, logID)
	if err != nil {
		return waffle.Error(msg, 404, "not_found", "Log not found")
	}

	result := map[string]any{
		"id":          msgRecord.ID,
		"chainId":     msgRecord.Data["chain_id"],
		"blockName":   msgRecord.Data["block_name"],
		"messageKind": msgRecord.Data["message_kind"],
		"action":      msgRecord.Data["action"],
		"durationMs":  msgRecord.Data["duration_ms"],
		"createdAt":   msgRecord.Data["created_at"],
	}
	if v := msgRecord.Data["trace_id"]; v != nil && v != "" {
		result["traceId"] = v
	}
	if v := msgRecord.Data["error"]; v != nil && v != "" {
		result["error"] = v
	}
	if v := msgRecord.Data["user_id"]; v != nil && v != "" {
		result["userId"] = v
	}
	if v := msgRecord.Data["meta_snapshot"]; v != nil && v != "" {
		result["metaSnapshot"] = v
	}

	return waffle.JSONRespond(msg, 200, result)
}

func (b *AdminBlock) handleClearLogs(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	var body struct {
		OlderThan string `json:"olderThan"`
	}
	if err := msg.Decode(&body); err != nil {
		return waffle.Error(msg, 400, "bad_request", "Invalid JSON body")
	}
	if body.OlderThan == "" {
		body.OlderThan = "7d"
	}

	cutoff := timeRangeCutoff(body.OlderThan)

	deleted := 0
	records, err := database.ListAll(context.Background(), db, logsCollection,
		database.Filter{Field: "created_at", Operator: database.OpLessThan, Value: cutoff},
	)
	if err == nil {
		for _, r := range records {
			if err := db.Delete(context.Background(), logsCollection, r.ID); err == nil {
				deleted++
			}
		}
	}

	msgRecords, err := database.ListAll(context.Background(), db, messageLogsCollection,
		database.Filter{Field: "created_at", Operator: database.OpLessThan, Value: cutoff},
	)
	if err == nil {
		for _, r := range msgRecords {
			if err := db.Delete(context.Background(), messageLogsCollection, r.ID); err == nil {
				deleted++
			}
		}
	}

	return waffle.JSONRespond(msg, 200, map[string]any{
		"message": fmt.Sprintf("Successfully deleted %d log entries", deleted),
		"deleted": deleted,
	})
}

func (b *AdminBlock) handleExportLogs(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	filters := buildLogFilters(msg)

	result, err := db.List(context.Background(), logsCollection, &database.ListOptions{
		Filters: filters,
		Sort:    []database.SortField{{Field: "created_at", Desc: true}},
		Limit:   10000,
	})
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to fetch logs")
	}

	var buf bytes.Buffer
	csvWriter := csv.NewWriter(&buf)
	csvWriter.Write([]string{"ID", "Time", "Level", "Message", "User ID", "Trace ID"})

	for _, r := range result.Records {
		record := []string{
			r.ID,
			fmt.Sprintf("%v", r.Data["created_at"]),
			fmt.Sprintf("%v", r.Data["level"]),
			fmt.Sprintf("%v", r.Data["message"]),
			fmt.Sprintf("%v", r.Data["user_id"]),
			fmt.Sprintf("%v", r.Data["trace_id"]),
		}
		csvWriter.Write(record)
	}
	csvWriter.Flush()

	filename := fmt.Sprintf("attachment; filename=logs_%s.csv", apptime.NowTime().Format("20060102_150405"))
	return msg.Respond(waffle.Response{
		Data: buf.Bytes(),
		Meta: map[string]string{
			waffle.MetaRespStatus:                              "200",
			waffle.MetaRespContentType:                         "text/csv",
			waffle.MetaRespHeaderPrefix + "Content-Disposition": filename,
		},
	})
}

// --- Log filter helpers ---

func buildLogFilters(msg *waffle.Message) []database.Filter {
	var filters []database.Filter

	if level := msg.Query("level"); level != "" {
		filters = append(filters, database.Filter{Field: "level", Operator: database.OpEqual, Value: level})
	}
	if search := msg.Query("search"); search != "" {
		filters = append(filters, database.Filter{Field: "message", Operator: database.OpLike, Value: "%" + search + "%"})
	}

	timeRange := msg.Query("range")
	if timeRange == "" {
		timeRange = "24h"
	}
	cutoff := timeRangeCutoff(timeRange)
	filters = append(filters, database.Filter{Field: "created_at", Operator: database.OpGreaterEqual, Value: cutoff})

	return filters
}

func buildMessageLogFilters(msg *waffle.Message) []database.Filter {
	var filters []database.Filter

	if v := msg.Query("chainId"); v != "" {
		filters = append(filters, database.Filter{Field: "chain_id", Operator: database.OpEqual, Value: v})
	}
	if v := msg.Query("blockName"); v != "" {
		filters = append(filters, database.Filter{Field: "block_name", Operator: database.OpEqual, Value: v})
	}
	if v := msg.Query("kind"); v != "" {
		filters = append(filters, database.Filter{Field: "message_kind", Operator: database.OpEqual, Value: v})
	}
	if v := msg.Query("action"); v != "" {
		filters = append(filters, database.Filter{Field: "action", Operator: database.OpEqual, Value: v})
	}

	timeRange := msg.Query("range")
	if timeRange == "" {
		timeRange = "24h"
	}
	cutoff := timeRangeCutoff(timeRange)
	filters = append(filters, database.Filter{Field: "created_at", Operator: database.OpGreaterEqual, Value: cutoff})

	return filters
}

func timeRangeCutoff(timeRange string) string {
	now := apptime.NowTime()
	var cutoff apptime.Time
	switch timeRange {
	case "1h":
		cutoff = now.Add(-1 * apptime.Hour)
	case "6h":
		cutoff = now.Add(-6 * apptime.Hour)
	case "24h":
		cutoff = now.Add(-24 * apptime.Hour)
	case "7d":
		cutoff = now.Add(-7 * 24 * apptime.Hour)
	case "30d":
		cutoff = now.Add(-30 * 24 * apptime.Hour)
	default:
		cutoff = now.Add(-24 * apptime.Hour)
	}
	return cutoff.Format(apptime.TimeFormat)
}
