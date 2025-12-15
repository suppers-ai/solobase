package database

import (
	"net/http"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/internal/core/services"
	"github.com/suppers-ai/solobase/utils"
)

type DatabaseTable struct {
	Name      string `json:"name"`
	Schema    string `json:"schema"`
	Type      string `json:"type"`
	RowsCount int    `json:"rowsCount"`
	Size      string `json:"size"`
}

type DatabaseColumn struct {
	Name      string `json:"name"`
	Type      string `json:"type"`
	Nullable  bool   `json:"nullable"`
	Default   string `json:"default,omitempty"`
	IsPrimary bool   `json:"isPrimary"`
	IsUnique  bool   `json:"isUnique"`
}

// HandleGetDatabaseInfo returns database information
func HandleGetDatabaseInfo(dbService *services.DatabaseService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		dbType, version := dbService.GetDatabaseInfo()
		
		info := map[string]interface{}{
			"type":    dbType,
			"version": version,
			"status":  "connected",
		}
		
		utils.JSONResponse(w, http.StatusOK, info)
	}
}

type QueryRequest struct {
	Query string `json:"query"`
}

type QueryResult struct {
	Columns       []string        `json:"columns"`
	Rows          [][]interface{} `json:"rows"`
	AffectedRows  int             `json:"affectedRows,omitempty"`
	ExecutionTime int64           `json:"executionTime"`
}

func HandleGetDatabaseTables(dbService *services.DatabaseService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		tables, err := dbService.GetTables()
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to fetch tables")
			return
		}

		utils.JSONResponse(w, http.StatusOK, tables)
	}
}

func HandleGetTableColumns(dbService *services.DatabaseService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		tableName := vars["table"]

		columns, err := dbService.GetTableColumns(tableName)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, "Failed to fetch columns")
			return
		}

		utils.JSONResponse(w, http.StatusOK, columns)
	}
}

func HandleExecuteQuery(dbService *services.DatabaseService) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req QueryRequest
		if !utils.DecodeJSONBody(w, r, &req) {
			return
		}

		// Check if user has permission to execute queries
		// This should be restricted to admin users only
		if !utils.IsAdmin(r) {
			utils.JSONError(w, http.StatusForbidden, "Insufficient permissions")
			return
		}

		result, err := dbService.ExecuteQuery(req.Query)
		if err != nil {
			utils.JSONError(w, http.StatusBadRequest, err.Error())
			return
		}

		utils.JSONResponse(w, http.StatusOK, result)
	}
}
