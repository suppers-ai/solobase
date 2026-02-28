package admin

import (
	"context"
	"encoding/json"
	"fmt"
	"strconv"
	"strings"

	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/solobase/core/models"
	waffle "github.com/suppers-ai/waffle-go"
	"github.com/suppers-ai/waffle-go/services/database"
)

func (b *AdminBlock) registerCustomTablesRoutes() {
	// Table CRUD
	b.router.Retrieve("/admin/custom-tables", b.handleListTables)
	b.router.Create("/admin/custom-tables", b.handleCreateTable)
	b.router.Retrieve("/admin/custom-tables/{name}", b.handleGetTableSchema)
	b.router.Update("/admin/custom-tables/{name}", b.handleAlterTable)
	b.router.Delete("/admin/custom-tables/{name}", b.handleDropTable)
	// Data CRUD
	b.router.Retrieve("/admin/custom-tables/{name}/data", b.handleQueryData)
	b.router.Create("/admin/custom-tables/{name}/data", b.handleInsertData)
	b.router.Retrieve("/admin/custom-tables/{name}/data/{id}", b.handleGetRecord)
	b.router.Update("/admin/custom-tables/{name}/data/{id}", b.handleUpdateRecord)
	b.router.Delete("/admin/custom-tables/{name}/data/{id}", b.handleDeleteRecord)
	// Migrations
	b.router.Retrieve("/admin/custom-tables/{name}/migrations", b.handleGetMigrationHistory)
}

// --- Table CRUD handlers ---

func (b *AdminBlock) handleCreateTable(_ waffle.Context, msg *waffle.Message) waffle.Result {
	if b.db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database not available")
	}

	userID := msg.UserID()

	var body createTableRequest
	if err := msg.Decode(&body); err != nil {
		return waffle.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	if body.Name == "" {
		return waffle.Error(msg, 400, "bad_request", "Table name is required")
	}
	if len(body.Fields) == 0 {
		return waffle.Error(msg, 400, "bad_request", "At least one field is required")
	}

	for _, field := range body.Fields {
		if !validateFieldType(field.Type) {
			return waffle.Error(msg, 400, "bad_request", "Invalid field type: "+field.Type)
		}
	}

	if err := validateTableName(body.Name); err != nil {
		return waffle.Error(msg, 400, "bad_request", err.Error())
	}

	tableName := models.EnsureCustomPrefix(body.Name)

	if b.ctTableExists(tableName) {
		return waffle.Error(msg, 400, "bad_request", fmt.Sprintf("table '%s' already exists", models.StripCustomPrefix(tableName)))
	}

	ctx := context.Background()
	now := apptime.NowString()
	fieldsJSON, _ := json.Marshal(body.Fields)
	indexesJSON, _ := json.Marshal(body.Indexes)
	optionsJSON, _ := json.Marshal(body.Options)

	rec, err := b.db.Create(ctx, "custom_table_definitions", map[string]any{
		"name":         tableName,
		"display_name": body.Name,
		"description":  body.Description,
		"fields":       string(fieldsJSON),
		"indexes":      string(indexesJSON),
		"options":      string(optionsJSON),
		"created_by":   userID,
		"status":       "active",
		"created_at":   now,
		"updated_at":   now,
	})
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to save table definition")
	}

	defID := rec.ID

	definition := &models.CustomTableDefinition{
		Name:    tableName,
		Fields:  body.Fields,
		Indexes: body.Indexes,
		Options: body.Options,
	}

	if err := b.ctCreatePhysicalTable(ctx, definition); err != nil {
		b.db.Delete(ctx, "custom_table_definitions", defID)
		return waffle.Error(msg, 400, "bad_request", fmt.Sprintf("failed to create table: %v", err))
	}

	schemaJSON := serializeSchema(definition)
	b.db.Create(ctx, "custom_table_migrations", map[string]any{
		"table_id":       defID,
		"version":        1,
		"migration_type": "create",
		"new_schema":     string(schemaJSON),
		"executed_by":    userID,
		"executed_at":    now,
		"status":         "completed",
	})

	return waffle.JSONRespond(msg, 201, map[string]any{
		"success":    true,
		"table_name": tableName,
		"message":    "Table created successfully",
	})
}

func (b *AdminBlock) handleListTables(_ waffle.Context, msg *waffle.Message) waffle.Result {
	if b.db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database not available")
	}

	ctx := context.Background()
	defs, err := b.ctListActiveDefinitions(ctx)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to list tables")
	}

	response := make([]map[string]any, len(defs))
	for i, table := range defs {
		count, _ := b.db.Count(ctx, table.Name, nil)

		response[i] = map[string]any{
			"id":          table.ID,
			"name":        table.DisplayName,
			"table_name":  table.Name,
			"description": table.Description,
			"field_count": len(table.Fields),
			"row_count":   count,
			"options":     table.Options,
			"created_at":  table.CreatedAt,
			"created_by":  table.CreatedBy,
		}
	}

	return waffle.JSONRespond(msg, 200, response)
}

func (b *AdminBlock) handleGetTableSchema(_ waffle.Context, msg *waffle.Message) waffle.Result {
	if b.db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database not available")
	}

	tableName := msg.Var("name")
	definition, err := b.ctGetDefinition(tableName)
	if err != nil {
		return waffle.Error(msg, 404, "not_found", err.Error())
	}

	ctx := context.Background()
	columns, err := b.ctGetTableColumns(ctx, definition.Name)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to get table columns")
	}

	return waffle.JSONRespond(msg, 200, map[string]any{
		"name":        definition.DisplayName,
		"table_name":  definition.Name,
		"description": definition.Description,
		"fields":      definition.Fields,
		"indexes":     definition.Indexes,
		"options":     definition.Options,
		"columns":     columns,
		"created_at":  definition.CreatedAt,
		"created_by":  definition.CreatedBy,
	})
}

func (b *AdminBlock) handleAlterTable(_ waffle.Context, msg *waffle.Message) waffle.Result {
	if b.db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database not available")
	}

	tableName := msg.Var("name")
	userID := msg.UserID()

	var body alterTableRequest
	if err := msg.Decode(&body); err != nil {
		return waffle.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	existing, err := b.ctGetDefinition(tableName)
	if err != nil {
		return waffle.Error(msg, 404, "not_found", err.Error())
	}

	oldSchema := serializeSchema(&existing.CustomTableDefinition)

	updates := &models.CustomTableDefinition{
		Fields:  body.Fields,
		Indexes: body.Indexes,
		Options: body.Options,
	}

	ctx := context.Background()
	if err := b.ctAlterPhysicalTable(ctx, existing, updates); err != nil {
		return waffle.Error(msg, 400, "bad_request", fmt.Sprintf("failed to alter table: %v", err))
	}

	existing.Fields = updates.Fields
	existing.Indexes = updates.Indexes
	existing.Options = updates.Options
	existing.Description = body.Description

	now := apptime.NowString()
	fieldsJSON, _ := json.Marshal(existing.Fields)
	indexesJSON, _ := json.Marshal(existing.Indexes)
	optionsJSON, _ := json.Marshal(existing.Options)

	b.db.Update(ctx, "custom_table_definitions", existing.DefID, map[string]any{
		"display_name": existing.DisplayName,
		"description":  existing.Description,
		"fields":       string(fieldsJSON),
		"indexes":      string(indexesJSON),
		"options":      string(optionsJSON),
		"status":       existing.Status,
		"updated_at":   now,
	})

	nextVersion := b.ctGetNextVersion(ctx, existing.DefID)
	newSchema := serializeSchema(&existing.CustomTableDefinition)

	b.db.Create(ctx, "custom_table_migrations", map[string]any{
		"table_id":       existing.DefID,
		"version":        nextVersion,
		"migration_type": "alter",
		"old_schema":     string(oldSchema),
		"new_schema":     string(newSchema),
		"executed_by":    userID,
		"executed_at":    now,
		"status":         "completed",
	})

	return waffle.JSONRespond(msg, 200, map[string]any{
		"success": true,
		"message": "Table altered successfully",
	})
}

func (b *AdminBlock) handleDropTable(_ waffle.Context, msg *waffle.Message) waffle.Result {
	if b.db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database not available")
	}

	tableName := msg.Var("name")
	userID := msg.UserID()
	permanent := msg.Query("permanent") == "true"

	existing, err := b.ctGetDefinition(tableName)
	if err != nil {
		return waffle.Error(msg, 404, "not_found", err.Error())
	}

	ctx := context.Background()
	now := apptime.NowString()

	nextVersion := b.ctGetNextVersion(ctx, existing.DefID)
	oldSchema := serializeSchema(&existing.CustomTableDefinition)

	b.db.Create(ctx, "custom_table_migrations", map[string]any{
		"table_id":       existing.DefID,
		"version":        nextVersion,
		"migration_type": "drop",
		"old_schema":     string(oldSchema),
		"executed_by":    userID,
		"executed_at":    now,
		"status":         "completed",
	})

	if permanent {
		if _, err := b.db.ExecRaw(ctx, fmt.Sprintf("DROP TABLE IF EXISTS %s", existing.Name)); err != nil {
			return waffle.Error(msg, 500, "internal_error", fmt.Sprintf("failed to drop table: %v", err))
		}
		b.db.Delete(ctx, "custom_table_definitions", existing.DefID)
	} else {
		b.db.Update(ctx, "custom_table_definitions", existing.DefID, map[string]any{
			"status":     "archived",
			"updated_at": now,
		})
	}

	message := "Table archived successfully"
	if permanent {
		message = "Table permanently deleted"
	}

	return waffle.JSONRespond(msg, 200, map[string]any{
		"success": true,
		"message": message,
	})
}

// --- Data CRUD handlers ---

func (b *AdminBlock) handleInsertData(_ waffle.Context, msg *waffle.Message) waffle.Result {
	if b.db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database not available")
	}

	tableName := msg.Var("name")
	definition, err := b.ctGetDefinition(tableName)
	if err != nil {
		return waffle.Error(msg, 404, "not_found", err.Error())
	}

	var body insertDataRequest
	if err := msg.Decode(&body); err != nil {
		return waffle.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	repo := newDynamicRepo(b.db, definition.Name, &definition.CustomTableDefinition)

	if len(body.Bulk) > 0 {
		if err := repo.BulkInsert(body.Bulk); err != nil {
			return waffle.Error(msg, 400, "bad_request", err.Error())
		}
		return waffle.JSONRespond(msg, 201, map[string]any{
			"success": true,
			"message": "Records inserted successfully",
			"count":   len(body.Bulk),
		})
	}

	if body.Data != nil {
		result, err := repo.Create(body.Data)
		if err != nil {
			return waffle.Error(msg, 400, "bad_request", err.Error())
		}
		return waffle.JSONRespond(msg, 201, result)
	}

	return waffle.Error(msg, 400, "bad_request", "No data provided")
}

func (b *AdminBlock) handleQueryData(_ waffle.Context, msg *waffle.Message) waffle.Result {
	if b.db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database not available")
	}

	tableName := msg.Var("name")
	definition, err := b.ctGetDefinition(tableName)
	if err != nil {
		return waffle.Error(msg, 404, "not_found", err.Error())
	}

	limit := 100
	if l := msg.Query("limit"); l != "" {
		if parsed, err := strconv.Atoi(l); err == nil && parsed > 0 {
			limit = parsed
		}
	}

	offset := 0
	if o := msg.Query("offset"); o != "" {
		if parsed, err := strconv.Atoi(o); err == nil && parsed >= 0 {
			offset = parsed
		}
	}

	queryParams := msg.QueryParams()
	conditions := make(map[string]any)
	for key, value := range queryParams {
		if key == "limit" || key == "offset" || key == "sort" || key == "order" {
			continue
		}
		for _, field := range definition.Fields {
			if field.Name == key {
				conditions[key] = value
				break
			}
		}
	}

	repo := newDynamicRepo(b.db, definition.Name, &definition.CustomTableDefinition)

	results, err := repo.Find(conditions, limit, offset)
	if err != nil {
		return waffle.Error(msg, 400, "bad_request", err.Error())
	}

	totalCount, _ := repo.Count(conditions)

	return waffle.JSONRespond(msg, 200, map[string]any{
		"data":       results,
		"total":      totalCount,
		"limit":      limit,
		"offset":     offset,
		"conditions": conditions,
	})
}

func (b *AdminBlock) handleGetRecord(_ waffle.Context, msg *waffle.Message) waffle.Result {
	if b.db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database not available")
	}

	tableName := msg.Var("name")
	recordID := msg.Var("id")

	definition, err := b.ctGetDefinition(tableName)
	if err != nil {
		return waffle.Error(msg, 404, "not_found", err.Error())
	}

	repo := newDynamicRepo(b.db, definition.Name, &definition.CustomTableDefinition)
	result, err := repo.FindByID(recordID)
	if err != nil {
		return waffle.Error(msg, 404, "not_found", err.Error())
	}

	return waffle.JSONRespond(msg, 200, result)
}

func (b *AdminBlock) handleUpdateRecord(_ waffle.Context, msg *waffle.Message) waffle.Result {
	if b.db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database not available")
	}

	tableName := msg.Var("name")
	recordID := msg.Var("id")

	definition, err := b.ctGetDefinition(tableName)
	if err != nil {
		return waffle.Error(msg, 404, "not_found", err.Error())
	}

	var updates map[string]any
	if err := msg.Decode(&updates); err != nil {
		return waffle.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	repo := newDynamicRepo(b.db, definition.Name, &definition.CustomTableDefinition)
	if err := repo.Update(recordID, updates); err != nil {
		return waffle.Error(msg, 400, "bad_request", err.Error())
	}

	return waffle.JSONRespond(msg, 200, map[string]any{
		"success": true,
		"message": "Record updated successfully",
	})
}

func (b *AdminBlock) handleDeleteRecord(_ waffle.Context, msg *waffle.Message) waffle.Result {
	if b.db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database not available")
	}

	tableName := msg.Var("name")
	recordID := msg.Var("id")

	definition, err := b.ctGetDefinition(tableName)
	if err != nil {
		return waffle.Error(msg, 404, "not_found", err.Error())
	}

	repo := newDynamicRepo(b.db, definition.Name, &definition.CustomTableDefinition)
	if err := repo.Delete(recordID); err != nil {
		return waffle.Error(msg, 400, "bad_request", err.Error())
	}

	message := "Record deleted successfully"
	if definition.Options.SoftDelete {
		message = "Record soft-deleted successfully"
	}

	return waffle.JSONRespond(msg, 200, map[string]any{
		"success": true,
		"message": message,
	})
}

// --- Migration handler ---

func (b *AdminBlock) handleGetMigrationHistory(_ waffle.Context, msg *waffle.Message) waffle.Result {
	if b.db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database not available")
	}

	tableName := msg.Var("name")
	definition, err := b.ctGetDefinition(tableName)
	if err != nil {
		return waffle.Error(msg, 404, "not_found", err.Error())
	}

	ctx := context.Background()
	result, err := b.db.List(ctx, "custom_table_migrations", &database.ListOptions{
		Filters: []database.Filter{
			{Field: "table_id", Operator: database.OpEqual, Value: definition.DefID},
		},
		Sort:  []database.SortField{{Field: "version", Desc: true}},
		Limit: 100,
	})
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to get migration history")
	}

	var migrations []map[string]any
	for _, rec := range result.Records {
		d := rec.Data
		mig := map[string]any{
			"id":            d["id"],
			"tableId":       d["table_id"],
			"version":       d["version"],
			"migrationType": strVal(d["migration_type"]),
			"executedBy":    strVal(d["executed_by"]),
			"executedAt":    strVal(d["executed_at"]),
			"status":        strVal(d["status"]),
			"errorMessage":  strVal(d["error_message"]),
		}
		if s := strVal(d["old_schema"]); s != "" {
			mig["oldSchema"] = json.RawMessage(s)
		}
		if s := strVal(d["new_schema"]); s != "" {
			mig["newSchema"] = json.RawMessage(s)
		}
		if s := strVal(d["rollback_at"]); s != "" {
			mig["rollbackAt"] = s
		}
		migrations = append(migrations, mig)
	}

	return waffle.JSONRespond(msg, 200, migrations)
}

// --- Custom tables internal helpers ---

type customTableDef struct {
	models.CustomTableDefinition
	DefID string
}

func (b *AdminBlock) ctGetDefinition(name string) (*customTableDef, error) {
	name = models.EnsureCustomPrefix(name)
	ctx := context.Background()

	rec, err := database.GetByField(ctx, b.db, "custom_table_definitions", "name", name)
	if err != nil {
		return nil, fmt.Errorf("table '%s' not found", models.StripCustomPrefix(name))
	}

	def := &customTableDef{}
	def.DefID = rec.ID
	d := rec.Data
	def.Name = strVal(d["name"])
	def.DisplayName = strVal(d["display_name"])
	def.Description = strVal(d["description"])
	def.CreatedBy = strVal(d["created_by"])
	def.Status = strVal(d["status"])

	if s := strVal(d["created_at"]); s != "" {
		def.CreatedAt = apptime.MustParse(s)
	}
	if s := strVal(d["updated_at"]); s != "" {
		def.UpdatedAt = apptime.MustParse(s)
	}

	if s := strVal(d["fields"]); s != "" {
		json.Unmarshal([]byte(s), &def.Fields)
	}
	if s := strVal(d["indexes"]); s != "" {
		json.Unmarshal([]byte(s), &def.Indexes)
	}
	if s := strVal(d["options"]); s != "" {
		json.Unmarshal([]byte(s), &def.Options)
	}

	if def.Status != "active" {
		return nil, fmt.Errorf("table '%s' not found", models.StripCustomPrefix(name))
	}

	return def, nil
}

func (b *AdminBlock) ctListActiveDefinitions(ctx context.Context) ([]customTableDef, error) {
	result, err := b.db.List(ctx, "custom_table_definitions", &database.ListOptions{
		Filters: []database.Filter{
			{Field: "status", Operator: database.OpEqual, Value: "active"},
		},
		Sort:  []database.SortField{{Field: "name"}},
		Limit: 10000,
	})
	if err != nil {
		return nil, err
	}

	defs := make([]customTableDef, len(result.Records))
	for i, rec := range result.Records {
		d := rec.Data
		def := &defs[i]
		def.DefID = rec.ID
		def.Name = strVal(d["name"])
		def.DisplayName = strVal(d["display_name"])
		def.Description = strVal(d["description"])
		def.CreatedBy = strVal(d["created_by"])
		def.Status = strVal(d["status"])

		if s := strVal(d["created_at"]); s != "" {
			def.CreatedAt = apptime.MustParse(s)
		}
		if s := strVal(d["updated_at"]); s != "" {
			def.UpdatedAt = apptime.MustParse(s)
		}
		if s := strVal(d["fields"]); s != "" {
			json.Unmarshal([]byte(s), &def.Fields)
		}
		if s := strVal(d["indexes"]); s != "" {
			json.Unmarshal([]byte(s), &def.Indexes)
		}
		if s := strVal(d["options"]); s != "" {
			json.Unmarshal([]byte(s), &def.Options)
		}
	}
	return defs, nil
}

func (b *AdminBlock) ctTableExists(tableName string) bool {
	ctx := context.Background()
	records, err := b.db.QueryRaw(ctx,
		"SELECT 1 FROM sqlite_master WHERE type='table' AND name=?", tableName,
	)
	return err == nil && len(records) > 0
}

func (b *AdminBlock) ctGetTableColumns(ctx context.Context, tableName string) ([]map[string]any, error) {
	records, err := b.db.QueryRaw(ctx, fmt.Sprintf("PRAGMA table_info(%s)", tableName))
	if err != nil {
		return nil, err
	}

	var columns []map[string]any
	for _, rec := range records {
		d := rec.Data
		col := map[string]any{
			"name":        strVal(d["name"]),
			"type":        strVal(d["type"]),
			"nullable":    toInt64(d["notnull"]) == 0,
			"has_default": d["dflt_value"] != nil,
			"is_primary":  toInt64(d["pk"]) == 1,
		}
		if d["dflt_value"] != nil {
			col["default_value"] = strVal(d["dflt_value"])
		}
		columns = append(columns, col)
	}
	return columns, nil
}

func (b *AdminBlock) ctGetNextVersion(ctx context.Context, defID string) int {
	records, err := b.db.QueryRaw(ctx,
		"SELECT MAX(version) as max_version FROM custom_table_migrations WHERE table_id = ?", defID,
	)
	if err != nil || len(records) == 0 {
		return 1
	}
	maxV := toInt64(records[0].Data["max_version"])
	if maxV == 0 {
		return 1
	}
	return int(maxV) + 1
}

func (b *AdminBlock) ctCreatePhysicalTable(ctx context.Context, definition *models.CustomTableDefinition) error {
	var columns []string

	hasPrimaryKey := false
	for _, field := range definition.Fields {
		if field.IsPrimaryKey {
			hasPrimaryKey = true
			break
		}
	}
	if !hasPrimaryKey {
		columns = append(columns, "id INTEGER PRIMARY KEY AUTOINCREMENT")
	}

	for _, field := range definition.Fields {
		columns = append(columns, buildColumnDef(field))
	}

	if definition.Options.Timestamps {
		columns = append(columns, "created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP")
		columns = append(columns, "updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP")
	}

	if definition.Options.SoftDelete {
		columns = append(columns, "deleted_at TIMESTAMP")
	}

	createSQL := fmt.Sprintf("CREATE TABLE %s (\n  %s\n)",
		definition.Name, strings.Join(columns, ",\n  "))

	if _, err := b.db.ExecRaw(ctx, createSQL); err != nil {
		return err
	}

	for _, field := range definition.Fields {
		if field.IsIndexed && !field.IsPrimaryKey && !field.IsUnique {
			indexSQL := fmt.Sprintf("CREATE INDEX idx_%s_%s ON %s(%s)",
				definition.Name, field.Name, definition.Name, field.Name)
			if _, err := b.db.ExecRaw(ctx, indexSQL); err != nil {
				return fmt.Errorf("failed to create index for %s: %w", field.Name, err)
			}
		}
	}

	if definition.Options.SoftDelete {
		indexSQL := fmt.Sprintf("CREATE INDEX idx_%s_deleted_at ON %s(deleted_at)", definition.Name, definition.Name)
		if _, err := b.db.ExecRaw(ctx, indexSQL); err != nil {
			return fmt.Errorf("failed to create deleted_at index: %w", err)
		}
	}

	for _, idx := range definition.Indexes {
		indexType := "INDEX"
		if idx.Unique {
			indexType = "UNIQUE INDEX"
		}
		indexSQL := fmt.Sprintf("CREATE %s %s ON %s(%s)",
			indexType, idx.Name, definition.Name, strings.Join(idx.Columns, ", "))
		if _, err := b.db.ExecRaw(ctx, indexSQL); err != nil {
			return fmt.Errorf("failed to create index %s: %w", idx.Name, err)
		}
	}

	return nil
}

func (b *AdminBlock) ctAlterPhysicalTable(ctx context.Context, existing *customTableDef, updates *models.CustomTableDefinition) error {
	existingFields := make(map[string]models.CustomTableField)
	for _, field := range existing.Fields {
		existingFields[field.Name] = field
	}

	newFields := make(map[string]models.CustomTableField)
	for _, field := range updates.Fields {
		newFields[field.Name] = field
	}

	for name, field := range newFields {
		if _, exists := existingFields[name]; !exists {
			columnDef := buildColumnDef(field)
			alterSQL := fmt.Sprintf("ALTER TABLE %s ADD COLUMN %s", existing.Name, columnDef)
			if _, err := b.db.ExecRaw(ctx, alterSQL); err != nil {
				return fmt.Errorf("failed to add column %s: %w", name, err)
			}
		}
	}

	for name := range existingFields {
		if _, exists := newFields[name]; !exists {
			if name == "id" || name == "created_at" || name == "updated_at" || name == "deleted_at" {
				continue
			}
			alterSQL := fmt.Sprintf("ALTER TABLE %s DROP COLUMN %s", existing.Name, name)
			if _, err := b.db.ExecRaw(ctx, alterSQL); err != nil {
				return fmt.Errorf("failed to drop column %s: %w", name, err)
			}
		}
	}

	return nil
}

// --- dynamicRepo ---

type dynamicRepo struct {
	db        database.Service
	tableName string
	def       *models.CustomTableDefinition
}

func newDynamicRepo(db database.Service, tableName string, def *models.CustomTableDefinition) *dynamicRepo {
	return &dynamicRepo{db: db, tableName: models.EnsureCustomPrefix(tableName), def: def}
}

func (r *dynamicRepo) Create(data map[string]any) (map[string]any, error) {
	model := models.NewDynamicModel(r.tableName, r.def)
	for key, value := range data {
		if err := model.Set(key, value); err != nil {
			return nil, err
		}
	}

	if r.def.Options.Timestamps {
		now := apptime.NowTime()
		data["created_at"] = now
		data["updated_at"] = now
	}

	var cols []string
	var placeholders []string
	var values []any

	for col, val := range data {
		cols = append(cols, col)
		placeholders = append(placeholders, "?")
		values = append(values, jsonMarshalIfComplex(val))
	}

	query := fmt.Sprintf("INSERT INTO %s (%s) VALUES (%s)",
		r.tableName, strings.Join(cols, ", "), strings.Join(placeholders, ", "))

	_, err := r.db.ExecRaw(context.Background(), query, values...)
	if err != nil {
		return nil, fmt.Errorf("failed to create record: %w", err)
	}

	records, err := r.db.QueryRaw(context.Background(),
		fmt.Sprintf("SELECT * FROM %s ORDER BY ROWID DESC LIMIT 1", r.tableName))
	if err == nil && len(records) > 0 {
		return records[0].Data, nil
	}

	return data, nil
}

func (r *dynamicRepo) FindByID(id string) (map[string]any, error) {
	query := fmt.Sprintf("SELECT * FROM %s WHERE id = ?", r.tableName)
	if r.def.Options.SoftDelete {
		query += " AND deleted_at IS NULL"
	}

	records, err := r.db.QueryRaw(context.Background(), query, id)
	if err != nil {
		return nil, err
	}
	if len(records) == 0 {
		return nil, fmt.Errorf("record with id %s not found", id)
	}
	return records[0].Data, nil
}

func (r *dynamicRepo) Find(conditions map[string]any, limit, offset int) ([]map[string]any, error) {
	query := fmt.Sprintf("SELECT * FROM %s WHERE 1=1", r.tableName)
	var values []any

	for field, value := range conditions {
		fieldExists := false
		for _, f := range r.def.Fields {
			if f.Name == field {
				fieldExists = true
				break
			}
		}
		if !fieldExists && field != "id" {
			return nil, fmt.Errorf("field '%s' does not exist", field)
		}
		query += fmt.Sprintf(" AND %s = ?", field)
		values = append(values, value)
	}

	if r.def.Options.SoftDelete {
		query += " AND deleted_at IS NULL"
	}
	if limit > 0 {
		query += fmt.Sprintf(" LIMIT %d", limit)
	}
	if offset > 0 {
		query += fmt.Sprintf(" OFFSET %d", offset)
	}

	records, err := r.db.QueryRaw(context.Background(), query, values...)
	if err != nil {
		return nil, err
	}

	results := make([]map[string]any, len(records))
	for i, rec := range records {
		results[i] = rec.Data
	}
	return results, nil
}

func (r *dynamicRepo) Update(id string, updates map[string]any) error {
	model := models.NewDynamicModel(r.tableName, r.def)
	for key, value := range updates {
		if key == "id" || key == "created_at" {
			continue
		}
		if err := model.Set(key, value); err != nil {
			return err
		}
	}

	if r.def.Options.Timestamps {
		updates["updated_at"] = apptime.NowTime()
	}

	var setClauses []string
	var values []any
	for col, val := range updates {
		if col == "id" || col == "created_at" {
			continue
		}
		setClauses = append(setClauses, fmt.Sprintf("%s = ?", col))
		values = append(values, jsonMarshalIfComplex(val))
	}
	values = append(values, id)

	query := fmt.Sprintf("UPDATE %s SET %s WHERE id = ?", r.tableName, strings.Join(setClauses, ", "))
	if r.def.Options.SoftDelete {
		query += " AND deleted_at IS NULL"
	}

	affected, err := r.db.ExecRaw(context.Background(), query, values...)
	if err != nil {
		return fmt.Errorf("failed to update record: %w", err)
	}
	if affected == 0 {
		return fmt.Errorf("record with id %s not found", id)
	}
	return nil
}

func (r *dynamicRepo) Delete(id string) error {
	ctx := context.Background()
	if r.def.Options.SoftDelete {
		_, err := r.db.ExecRaw(ctx,
			fmt.Sprintf("UPDATE %s SET deleted_at = ? WHERE id = ?", r.tableName),
			apptime.NowTime(), id,
		)
		return err
	}

	_, err := r.db.ExecRaw(ctx,
		fmt.Sprintf("DELETE FROM %s WHERE id = ?", r.tableName), id,
	)
	return err
}

func (r *dynamicRepo) Count(conditions map[string]any) (int64, error) {
	query := fmt.Sprintf("SELECT COUNT(*) as cnt FROM %s WHERE 1=1", r.tableName)
	var values []any

	for field, value := range conditions {
		query += fmt.Sprintf(" AND %s = ?", field)
		values = append(values, value)
	}
	if r.def.Options.SoftDelete {
		query += " AND deleted_at IS NULL"
	}

	records, err := r.db.QueryRaw(context.Background(), query, values...)
	if err != nil {
		return 0, err
	}
	if len(records) == 0 {
		return 0, nil
	}
	return toInt64(records[0].Data["cnt"]), nil
}

func (r *dynamicRepo) BulkInsert(bulkRecords []map[string]any) error {
	for _, record := range bulkRecords {
		if _, err := r.Create(record); err != nil {
			return err
		}
	}
	return nil
}

// --- Utility functions ---

func jsonMarshalIfComplex(val any) any {
	switch v := val.(type) {
	case map[string]any:
		b, _ := json.Marshal(v)
		return string(b)
	case []any:
		b, _ := json.Marshal(v)
		return string(b)
	default:
		return val
	}
}

func buildColumnDef(field models.CustomTableField) string {
	var parts []string
	parts = append(parts, field.Name)
	parts = append(parts, field.GetSQLType())

	if field.IsPrimaryKey {
		if field.AutoIncrement {
			parts[1] = "INTEGER"
			parts = append(parts, "AUTOINCREMENT")
		}
		parts = append(parts, "PRIMARY KEY")
	}

	if !field.Nullable && !field.IsPrimaryKey {
		parts = append(parts, "NOT NULL")
	}

	if field.IsUnique {
		parts = append(parts, "UNIQUE")
	}

	if field.DefaultValue != nil {
		parts = append(parts, fmt.Sprintf("DEFAULT %s", formatDefault(field.DefaultValue, field.Type)))
	}

	if field.ForeignKey != nil {
		fk := fmt.Sprintf("REFERENCES %s(%s)", field.ForeignKey.ReferenceTable, field.ForeignKey.ReferenceColumn)
		if field.ForeignKey.OnDelete != "" {
			fk += fmt.Sprintf(" ON DELETE %s", field.ForeignKey.OnDelete)
		}
		if field.ForeignKey.OnUpdate != "" {
			fk += fmt.Sprintf(" ON UPDATE %s", field.ForeignKey.OnUpdate)
		}
		parts = append(parts, fk)
	}

	return strings.Join(parts, " ")
}

func formatDefault(value any, fieldType string) string {
	switch fieldType {
	case "string", "text", "varchar":
		return fmt.Sprintf("'%v'", value)
	case "bool", "boolean":
		if v, ok := value.(bool); ok && v {
			return "TRUE"
		}
		return "FALSE"
	case "time", "timestamp":
		if value == "now" || value == "CURRENT_TIMESTAMP" {
			return "CURRENT_TIMESTAMP"
		}
		return fmt.Sprintf("'%v'", value)
	default:
		return fmt.Sprintf("%v", value)
	}
}

func validateTableName(name string) error {
	if name == "" {
		return fmt.Errorf("table name cannot be empty")
	}
	if len(name) < 3 || len(name) > 50 {
		return fmt.Errorf("table name must be between 3 and 50 characters")
	}
	if !isValidTableName(name) {
		return fmt.Errorf("table name must start with a letter and contain only lowercase letters, numbers, and underscores")
	}

	cleanName := models.StripCustomPrefix(name)
	for _, reserved := range reservedTableNames {
		if cleanName == reserved {
			return fmt.Errorf("table name '%s' is reserved", cleanName)
		}
	}
	return nil
}

func isValidTableName(name string) bool {
	if len(name) == 0 {
		return false
	}
	if name[0] < 'a' || name[0] > 'z' {
		return false
	}
	for i := 1; i < len(name); i++ {
		c := name[i]
		if !((c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == '_') {
			return false
		}
	}
	return true
}

func validateFieldType(fieldType string) bool {
	validTypes := []string{"string", "int", "float", "bool", "time", "date", "json", "jsonb", "text", "uuid"}
	for _, valid := range validTypes {
		if fieldType == valid {
			return true
		}
	}
	return false
}

func serializeSchema(definition *models.CustomTableDefinition) json.RawMessage {
	data, _ := json.Marshal(definition)
	return json.RawMessage(data)
}

var reservedTableNames = []string{
	"users", "roles", "permissions", "sessions", "settings",
	"custom_table_definitions", "custom_table_migrations",
	"logs", "audit_logs", "migrations",
}

type createTableRequest struct {
	Name        string                    `json:"name"`
	Description string                    `json:"description"`
	Fields      []models.CustomTableField `json:"fields"`
	Indexes     []models.CustomTableIndex `json:"indexes,omitempty"`
	Options     models.CustomTableOptions `json:"options"`
}

type alterTableRequest struct {
	Description string                    `json:"description,omitempty"`
	Fields      []models.CustomTableField `json:"fields"`
	Indexes     []models.CustomTableIndex `json:"indexes,omitempty"`
	Options     models.CustomTableOptions `json:"options"`
}

type insertDataRequest struct {
	Data map[string]any   `json:"data,omitempty"`
	Bulk []map[string]any `json:"bulk,omitempty"`
}
