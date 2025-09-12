package handlers

import (
	"encoding/json"
	"net/http"
	"strconv"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/services"
	"github.com/suppers-ai/solobase/utils"
)

// Collections handlers
func ListCollections(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ctx := r.Context()
		collections, err := svc.Collections().ListCollections(ctx)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, err.Error())
			return
		}
		utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
			"collections": collections,
			"total":       len(collections),
		})
	}
}

func CreateCollection(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ctx := r.Context()

		var req struct {
			Name        string                 `json:"name"`
			DisplayName string                 `json:"display_name"`
			Description string                 `json:"description"`
			Schema      map[string]interface{} `json:"schema"`
		}
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid request body")
			return
		}

		collection, err := svc.Collections().CreateCollection(ctx, req.Name, req.DisplayName, req.Description, req.Schema)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, err.Error())
			return
		}

		utils.JSONResponse(w, http.StatusCreated, collection)
	}
}

func GetCollection(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ctx := r.Context()
		vars := mux.Vars(r)
		collectionName := vars["collection"]

		collection, err := svc.Collections().GetCollection(ctx, collectionName)
		if err != nil {
			utils.JSONError(w, http.StatusNotFound, "Collection not found")
			return
		}

		utils.JSONResponse(w, http.StatusOK, collection)
	}
}

func UpdateCollection(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		utils.JSONResponse(w, http.StatusNotImplemented, map[string]string{"message": "Not implemented"})
	}
}

func DeleteCollection(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		utils.JSONResponse(w, http.StatusNotImplemented, map[string]string{"message": "Not implemented"})
	}
}

// Records handlers
func ListRecords(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ctx := r.Context()
		vars := mux.Vars(r)
		collectionName := vars["collection"]

		// Parse query parameters
		limit := 100
		offset := 0

		if l := r.URL.Query().Get("limit"); l != "" {
			if val, err := strconv.Atoi(l); err == nil && val > 0 && val <= 1000 {
				limit = val
			}
		}

		if o := r.URL.Query().Get("offset"); o != "" {
			if val, err := strconv.Atoi(o); err == nil && val >= 0 {
				offset = val
			}
		}

		records, total, err := svc.Collections().ListRecords(ctx, collectionName, limit, offset, nil)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, err.Error())
			return
		}

		utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
			"records": records,
			"total":   total,
			"limit":   limit,
			"offset":  offset,
		})
	}
}

func CreateRecord(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ctx := r.Context()
		vars := mux.Vars(r)
		collectionName := vars["collection"]

		var record map[string]interface{}
		if err := json.NewDecoder(r.Body).Decode(&record); err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid request body")
			return
		}

		id, err := svc.Collections().CreateRecord(ctx, collectionName, record, nil)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, err.Error())
			return
		}

		record["id"] = id
		utils.JSONResponse(w, http.StatusCreated, record)
	}
}

func GetRecord(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		utils.JSONResponse(w, http.StatusNotImplemented, map[string]string{"message": "Not implemented"})
	}
}

func UpdateRecord(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		utils.JSONResponse(w, http.StatusNotImplemented, map[string]string{"message": "Not implemented"})
	}
}

func DeleteRecord(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		utils.JSONResponse(w, http.StatusNotImplemented, map[string]string{"message": "Not implemented"})
	}
}

// Storage handlers
func ListBuckets(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		utils.JSONResponse(w, http.StatusNotImplemented, map[string]string{"message": "Not implemented"})
	}
}

func CreateBucket(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		utils.JSONResponse(w, http.StatusNotImplemented, map[string]string{"message": "Not implemented"})
	}
}

func DeleteBucket(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		utils.JSONResponse(w, http.StatusNotImplemented, map[string]string{"message": "Not implemented"})
	}
}

func UploadFile(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		utils.JSONResponse(w, http.StatusNotImplemented, map[string]string{"message": "Not implemented"})
	}
}

func GetFile(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		utils.JSONResponse(w, http.StatusNotImplemented, map[string]string{"message": "Not implemented"})
	}
}

func DeleteFile(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		utils.JSONResponse(w, http.StatusNotImplemented, map[string]string{"message": "Not implemented"})
	}
}

// User management handlers
func ListUsers(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ctx := r.Context()

		// Get users from users table
		rows, err := svc.Database().Query(ctx, `
			SELECT id, email, role, email_verified, created_at 
			FROM users 
			ORDER BY created_at DESC
		`)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, err.Error())
			return
		}
		defer rows.Close()

		var users []map[string]interface{}
		for rows.Next() {
			var user struct {
				ID            string
				Email         string
				Role          *string
				EmailVerified bool
				CreatedAt     string
			}

			if err := rows.Scan(&user.ID, &user.Email, &user.Role, &user.EmailVerified, &user.CreatedAt); err != nil {
				continue
			}

			role := "user"
			if user.Role != nil {
				role = *user.Role
			}

			users = append(users, map[string]interface{}{
				"id":             user.ID,
				"email":          user.Email,
				"role":           role,
				"email_verified": user.EmailVerified,
				"created_at":     user.CreatedAt,
			})
		}

		utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
			"users": users,
			"total": len(users),
		})
	}
}

func GetUser(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		utils.JSONResponse(w, http.StatusNotImplemented, map[string]string{"message": "Not implemented"})
	}
}

func UpdateUser(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		utils.JSONResponse(w, http.StatusNotImplemented, map[string]string{"message": "Not implemented"})
	}
}

func DeleteUser(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		utils.JSONResponse(w, http.StatusNotImplemented, map[string]string{"message": "Not implemented"})
	}
}

// Logs handlers
func GetLogs(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ctx := r.Context()

		// Get logs from logger schema
		rows, err := svc.Database().Query(ctx, `
			SELECT id, level, message, fields, user_id, trace_id, created_at 
			FROM logger.logs 
			ORDER BY created_at DESC 
			LIMIT 100
		`)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, err.Error())
			return
		}
		defer rows.Close()

		var logs []map[string]interface{}
		for rows.Next() {
			var log struct {
				ID        string
				Level     string
				Message   string
				Fields    json.RawMessage
				UserID    *string
				TraceID   *string
				CreatedAt string
			}

			if err := rows.Scan(&log.ID, &log.Level, &log.Message, &log.Fields, &log.UserID, &log.TraceID, &log.CreatedAt); err != nil {
				continue
			}

			logs = append(logs, map[string]interface{}{
				"id":         log.ID,
				"level":      log.Level,
				"message":    log.Message,
				"fields":     log.Fields,
				"user_id":    log.UserID,
				"trace_id":   log.TraceID,
				"created_at": log.CreatedAt,
			})
		}

		utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
			"logs":  logs,
			"total": len(logs),
		})
	}
}

func GetRequestLogs(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		utils.JSONResponse(w, http.StatusNotImplemented, map[string]string{"message": "Not implemented"})
	}
}

// WebSocket handler for real-time subscriptions
func WebSocketHandler(svc *services.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		utils.JSONResponse(w, http.StatusNotImplemented, map[string]string{"message": "WebSocket not implemented"})
	}
}
