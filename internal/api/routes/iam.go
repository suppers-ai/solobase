package routes

import (
	"encoding/json"
	"net/http"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/internal/iam"
)

// RegisterIAMRoutes registers all IAM-related routes
func RegisterIAMRoutes(router *mux.Router, iamService *iam.Service) {
	// Apply auth middleware to all IAM routes
	iamRouter := router.PathPrefix("/iam").Subrouter()
	// Note: Auth middleware is already applied in the parent router
	// We just need to ensure these routes are protected

	// Role management
	iamRouter.HandleFunc("/roles", handleGetRoles(iamService)).Methods("GET")
	iamRouter.HandleFunc("/roles", handleCreateRole(iamService)).Methods("POST")
	iamRouter.HandleFunc("/roles/{name}", handleDeleteRole(iamService)).Methods("DELETE")
	iamRouter.HandleFunc("/roles/{name}", handleUpdateRole(iamService)).Methods("PUT")

	// Policy management
	iamRouter.HandleFunc("/policies", handleGetPolicies(iamService)).Methods("GET")
	iamRouter.HandleFunc("/policies", handleCreatePolicy(iamService)).Methods("POST")
	iamRouter.HandleFunc("/policies/{id}", handleDeletePolicy(iamService)).Methods("DELETE")

	// User role management
	iamRouter.HandleFunc("/users", handleGetUsersWithRoles(iamService)).Methods("GET")
	iamRouter.HandleFunc("/users/{userId}/roles", handleAssignRole(iamService)).Methods("POST")
	iamRouter.HandleFunc("/users/{userId}/roles/{roleName}", handleRemoveRole(iamService)).Methods("DELETE")

	// Permission testing
	iamRouter.HandleFunc("/test-permission", handleTestPermission(iamService)).Methods("POST")

	// Audit logs
	iamRouter.HandleFunc("/audit-logs", handleGetAuditLogs(iamService)).Methods("GET")
}

func handleGetRoles(iamService *iam.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		roles, err := iamService.GetRoles(r.Context())
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(roles)
	}
}

func handleCreateRole(iamService *iam.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var role iam.Role
		if err := json.NewDecoder(r.Body).Decode(&role); err != nil {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}

		if err := iamService.CreateRole(r.Context(), &role); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(role)
	}
}

func handleDeleteRole(iamService *iam.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		roleName := vars["name"]

		if err := iamService.DeleteRole(r.Context(), roleName); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.WriteHeader(http.StatusNoContent)
	}
}

func handleUpdateRole(iamService *iam.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		roleName := vars["name"]

		var role iam.Role
		if err := json.NewDecoder(r.Body).Decode(&role); err != nil {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}

		updates := map[string]interface{}{
			"display_name": role.DisplayName,
			"description":  role.Description,
			"metadata":     role.Metadata,
		}
		
		if err := iamService.UpdateRole(r.Context(), roleName, updates); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(role)
	}
}

func handleGetPolicies(iamService *iam.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		policies, err := iamService.GetPolicies(r.Context())
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(policies)
	}
}

func handleCreatePolicy(iamService *iam.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var policy struct {
			Subject  string `json:"subject"`
			Resource string `json:"resource"`
			Action   string `json:"action"`
			Effect   string `json:"effect"`
		}

		if err := json.NewDecoder(r.Body).Decode(&policy); err != nil {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}

		if err := iamService.AddPolicy(r.Context(), policy.Subject, policy.Resource, policy.Action, policy.Effect); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.WriteHeader(http.StatusCreated)
		json.NewEncoder(w).Encode(map[string]string{"status": "created"})
	}
}

func handleDeletePolicy(iamService *iam.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		policyID := vars["id"]

		// For now, we'll need to parse the ID to get the policy details
		// In a real implementation, you'd store policies with IDs
		var policy struct {
			Subject  string `json:"subject"`
			Resource string `json:"resource"`
			Action   string `json:"action"`
		}

		// Get the policy details from the request body for deletion
		if err := json.NewDecoder(r.Body).Decode(&policy); err != nil {
			// Try to delete by pattern matching - assume "allow" effect by default
			if err := iamService.RemovePolicy(r.Context(), policyID, "*", "*", "allow"); err != nil {
				http.Error(w, err.Error(), http.StatusInternalServerError)
				return
			}
		} else {
			// Default to "allow" effect if not specified
			if err := iamService.RemovePolicy(r.Context(), policy.Subject, policy.Resource, policy.Action, "allow"); err != nil {
				http.Error(w, err.Error(), http.StatusInternalServerError)
				return
			}
		}

		w.WriteHeader(http.StatusNoContent)
	}
}

func handleGetUsersWithRoles(iamService *iam.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		users, err := iamService.GetUsersWithRoles(r.Context())
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(users)
	}
}

func handleAssignRole(iamService *iam.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		userID := vars["userId"]

		var req struct {
			Role string `json:"role"`
		}

		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}

		if err := iamService.AssignRoleToUser(r.Context(), userID, req.Role); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.WriteHeader(http.StatusCreated)
		json.NewEncoder(w).Encode(map[string]string{"status": "assigned"})
	}
}

func handleRemoveRole(iamService *iam.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		userID := vars["userId"]
		roleName := vars["roleName"]

		if err := iamService.RemoveRoleFromUser(r.Context(), userID, roleName); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.WriteHeader(http.StatusNoContent)
	}
}

func handleTestPermission(iamService *iam.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req struct {
			UserID   string                 `json:"userId"`
			Resource string                 `json:"resource"`
			Action   string                 `json:"action"`
			Context  map[string]interface{} `json:"context,omitempty"`
		}

		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}

		allowed, err := iamService.CheckPermission(r.Context(), req.UserID, req.Resource, req.Action)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		// Get user roles for debugging
		userRoles, _ := iamService.GetUserRoles(r.Context(), req.UserID)

		result := map[string]interface{}{
			"allowed":   allowed,
			"userRoles": userRoles,
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(result)
	}
}

func handleGetAuditLogs(iamService *iam.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		limit := r.URL.Query().Get("limit")
		filter := r.URL.Query().Get("filter")
		logType := r.URL.Query().Get("type")

		logs, err := iamService.GetAuditLogs(r.Context(), limit, filter, logType)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(logs)
	}
}