package custom_tables

import (
	"database/sql"
	"fmt"
	"net/http"
	"strconv"

	"github.com/gorilla/mux"
	coremodels "github.com/suppers-ai/solobase/internal/core/models"
	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/internal/data/models"
	"github.com/suppers-ai/solobase/utils"
)

// Handler handles custom table API requests
type Handler struct {
	service *services.CustomTablesService
	db      *sql.DB
}

// NewHandler creates a new custom tables handler
func NewHandler(service *services.CustomTablesService, db *sql.DB) *Handler {
	return &Handler{
		service: service,
		db:      db,
	}
}

// CreateTableRequest represents a request to create a new table
type CreateTableRequest struct {
	Name        string                    `json:"name"`        // User-provided name (without custom_ prefix)
	Description string                    `json:"description"`
	Fields      []models.CustomTableField `json:"fields"`
	Indexes     []models.CustomTableIndex `json:"indexes,omitempty"`
	Options     models.CustomTableOptions `json:"options"`
}

// CreateTable handles POST /api/admin/custom-tables
func (h *Handler) CreateTable(w http.ResponseWriter, r *http.Request) {
	userID := utils.GetUserIDFromContext(r)

	var req CreateTableRequest
	if !utils.DecodeJSONBody(w, r, &req) {
		return
	}

	// Validate required fields
	if req.Name == "" {
		utils.JSONError(w, http.StatusBadRequest, "Table name is required")
		return
	}

	if len(req.Fields) == 0 {
		utils.JSONError(w, http.StatusBadRequest, "At least one field is required")
		return
	}

	// Validate field types
	for _, field := range req.Fields {
		if !h.service.ValidateFieldType(field.Type) {
			utils.JSONError(w, http.StatusBadRequest, "Invalid field type: "+field.Type)
			return
		}
	}

	// Create table definition
	definition := &models.CustomTableDefinition{
		DisplayName: req.Name,
		Description: req.Description,
		Fields:      req.Fields,
		Indexes:     req.Indexes,
		Options:     req.Options,
	}

	// Create the table
	if err := h.service.CreateTable(definition, userID); err != nil {
		utils.JSONError(w, http.StatusBadRequest, err.Error())
		return
	}

	utils.JSONResponse(w, http.StatusCreated, map[string]interface{}{
		"success":    true,
		"table_name": definition.Name,
		"message":    "Table created successfully",
	})
}

// ListTables handles GET /api/admin/custom-tables
func (h *Handler) ListTables(w http.ResponseWriter, r *http.Request) {
	tables, err := h.service.ListTables()
	if err != nil {
		utils.JSONError(w, http.StatusInternalServerError, "Failed to list tables")
		return
	}

	// Transform for response
	response := make([]map[string]interface{}, len(tables))
	for i, table := range tables {
		// Get row count
		var count int64
		h.db.QueryRow(fmt.Sprintf("SELECT COUNT(*) FROM %s", table.Name)).Scan(&count)

		response[i] = map[string]interface{}{
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

	utils.JSONResponse(w, http.StatusOK, response)
}

// GetTableSchema handles GET /api/admin/custom-tables/{name}
func (h *Handler) GetTableSchema(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	tableName := vars["name"]

	schema, err := h.service.GetTableSchema(tableName)
	if err != nil {
		utils.JSONError(w, http.StatusNotFound, err.Error())
		return
	}

	utils.JSONResponse(w, http.StatusOK, schema)
}

// AlterTableRequest represents a request to alter a table
type AlterTableRequest struct {
	Description string                    `json:"description,omitempty"`
	Fields      []models.CustomTableField `json:"fields"`
	Indexes     []models.CustomTableIndex `json:"indexes,omitempty"`
	Options     models.CustomTableOptions `json:"options"`
}

// AlterTable handles PUT /api/admin/custom-tables/{name}
func (h *Handler) AlterTable(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	tableName := vars["name"]

	userID := utils.GetUserIDFromContext(r)

	var req AlterTableRequest
	if !utils.DecodeJSONBody(w, r, &req) {
		return
	}

	// Create update definition
	updates := &models.CustomTableDefinition{
		Description: req.Description,
		Fields:      req.Fields,
		Indexes:     req.Indexes,
		Options:     req.Options,
	}

	// Alter the table
	if err := h.service.AlterTable(tableName, updates, userID); err != nil {
		utils.JSONError(w, http.StatusBadRequest, err.Error())
		return
	}

	utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
		"success": true,
		"message": "Table altered successfully",
	})
}

// DropTable handles DELETE /api/admin/custom-tables/{name}
func (h *Handler) DropTable(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	tableName := vars["name"]

	userID := utils.GetUserIDFromContext(r)

	// Check for permanent deletion flag
	permanent := r.URL.Query().Get("permanent") == "true"

	// Drop the table
	if err := h.service.DropTable(tableName, permanent, userID); err != nil {
		utils.JSONError(w, http.StatusBadRequest, err.Error())
		return
	}

	message := "Table archived successfully"
	if permanent {
		message = "Table permanently deleted"
	}

	utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
		"success": true,
		"message": message,
	})
}

// InsertDataRequest represents a request to insert data
type InsertDataRequest struct {
	Data   map[string]interface{}   `json:"data,omitempty"`   // Single record
	Bulk   []map[string]interface{} `json:"bulk,omitempty"`   // Multiple records
}

// InsertData handles POST /api/admin/custom-tables/{name}/data
func (h *Handler) InsertData(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	tableName := vars["name"]

	// Get table definition
	definition, err := h.service.GetTable(tableName)
	if err != nil {
		utils.JSONError(w, http.StatusNotFound, err.Error())
		return
	}

	var req InsertDataRequest
	if !utils.DecodeJSONBody(w, r, &req) {
		return
	}

	// Create repository
	repo := coremodels.NewDynamicRepository(h.db, tableName, definition)

	// Handle bulk insert
	if len(req.Bulk) > 0 {
		if err := repo.BulkInsert(req.Bulk); err != nil {
			utils.JSONError(w, http.StatusBadRequest, err.Error())
			return
		}

		utils.JSONResponse(w, http.StatusCreated, map[string]interface{}{
			"success": true,
			"message": "Records inserted successfully",
			"count":   len(req.Bulk),
		})
		return
	}

	// Handle single insert
	if req.Data != nil {
		result, err := repo.Create(req.Data)
		if err != nil {
			utils.JSONError(w, http.StatusBadRequest, err.Error())
			return
		}

		utils.JSONResponse(w, http.StatusCreated, result)
		return
	}

	utils.JSONError(w, http.StatusBadRequest, "No data provided")
}

// QueryData handles GET /api/admin/custom-tables/{name}/data
func (h *Handler) QueryData(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	tableName := vars["name"]

	// Get table definition
	definition, err := h.service.GetTable(tableName)
	if err != nil {
		utils.JSONError(w, http.StatusNotFound, err.Error())
		return
	}

	// Parse query parameters
	query := r.URL.Query()

	// Pagination
	limit := 100 // Default limit
	if l := query.Get("limit"); l != "" {
		if parsed, err := strconv.Atoi(l); err == nil && parsed > 0 {
			limit = parsed
		}
	}

	offset := 0
	if o := query.Get("offset"); o != "" {
		if parsed, err := strconv.Atoi(o); err == nil && parsed >= 0 {
			offset = parsed
		}
	}

	// Build conditions from query parameters
	conditions := make(map[string]interface{})
	for key, values := range query {
		// Skip pagination parameters
		if key == "limit" || key == "offset" || key == "sort" || key == "order" {
			continue
		}

		// Check if field exists
		fieldExists := false
		for _, field := range definition.Fields {
			if field.Name == key {
				fieldExists = true
				break
			}
		}

		if fieldExists && len(values) > 0 {
			conditions[key] = values[0]
		}
	}

	// Create repository and query data
	repo := coremodels.NewDynamicRepository(h.db, tableName, definition)

	results, err := repo.Find(conditions, limit, offset)
	if err != nil {
		utils.JSONError(w, http.StatusBadRequest, err.Error())
		return
	}

	// Get total count
	totalCount, _ := repo.Count(conditions)

	utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
		"data":        results,
		"total":       totalCount,
		"limit":       limit,
		"offset":      offset,
		"conditions":  conditions,
	})
}

// GetRecord handles GET /api/admin/custom-tables/{name}/data/{id}
func (h *Handler) GetRecord(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	tableName := vars["name"]
	recordID := vars["id"]

	// Get table definition
	definition, err := h.service.GetTable(tableName)
	if err != nil {
		utils.JSONError(w, http.StatusNotFound, err.Error())
		return
	}

	// Create repository
	repo := coremodels.NewDynamicRepository(h.db, tableName, definition)

	// Get record
	result, err := repo.FindByID(recordID)
	if err != nil {
		utils.JSONError(w, http.StatusNotFound, err.Error())
		return
	}

	utils.JSONResponse(w, http.StatusOK, result)
}

// UpdateRecord handles PUT /api/admin/custom-tables/{name}/data/{id}
func (h *Handler) UpdateRecord(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	tableName := vars["name"]
	recordID := vars["id"]

	// Get table definition
	definition, err := h.service.GetTable(tableName)
	if err != nil {
		utils.JSONError(w, http.StatusNotFound, err.Error())
		return
	}

	var updates map[string]interface{}
	if !utils.DecodeJSONBody(w, r, &updates) {
		return
	}

	// Create repository
	repo := coremodels.NewDynamicRepository(h.db, tableName, definition)

	// Update record
	if err := repo.Update(recordID, updates); err != nil {
		utils.JSONError(w, http.StatusBadRequest, err.Error())
		return
	}

	utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
		"success": true,
		"message": "Record updated successfully",
	})
}

// DeleteRecord handles DELETE /api/admin/custom-tables/{name}/data/{id}
func (h *Handler) DeleteRecord(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	tableName := vars["name"]
	recordID := vars["id"]

	// Get table definition
	definition, err := h.service.GetTable(tableName)
	if err != nil {
		utils.JSONError(w, http.StatusNotFound, err.Error())
		return
	}

	// Create repository
	repo := coremodels.NewDynamicRepository(h.db, tableName, definition)

	// Delete record
	if err := repo.Delete(recordID); err != nil {
		utils.JSONError(w, http.StatusBadRequest, err.Error())
		return
	}

	message := "Record deleted successfully"
	if definition.Options.SoftDelete {
		message = "Record soft-deleted successfully"
	}

	utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
		"success": true,
		"message": message,
	})
}

// GetMigrationHistory handles GET /api/admin/custom-tables/{name}/migrations
func (h *Handler) GetMigrationHistory(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	tableName := vars["name"]

	migrations, err := h.service.GetMigrationHistory(tableName)
	if err != nil {
		utils.JSONError(w, http.StatusNotFound, err.Error())
		return
	}

	utils.JSONResponse(w, http.StatusOK, migrations)
}
