package routes

import (
	"encoding/json"
	"log"
	"net/http"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/internal/iam"
)

// RegisterIAMRoutes registers all IAM-related routes
func RegisterIAMRoutes(router *mux.Router, iamService *iam.Service) {
	log.Printf("RegisterIAMRoutes: Registering IAM routes")
	// Apply auth middleware to all IAM routes
	iamRouter := router.PathPrefix("/iam").Subrouter()
	// Note: Auth middleware is already applied in the parent router
	// We just need to ensure these routes are protected

	// Role management
	iamRouter.HandleFunc("/roles", handleGetRoles(iamService)).Methods("GET", "OPTIONS")
	iamRouter.HandleFunc("/roles", handleCreateRole(iamService)).Methods("POST", "OPTIONS")
	iamRouter.HandleFunc("/roles/{name}", handleDeleteRole(iamService)).Methods("DELETE", "OPTIONS")
	iamRouter.HandleFunc("/roles/{name}", handleUpdateRole(iamService)).Methods("PUT", "OPTIONS")

	// Policy management
	iamRouter.HandleFunc("/policies", handleGetPolicies(iamService)).Methods("GET", "OPTIONS")
	iamRouter.HandleFunc("/policies", handleCreatePolicy(iamService)).Methods("POST", "OPTIONS")
	iamRouter.HandleFunc("/policies/{id}", handleDeletePolicy(iamService)).Methods("DELETE", "OPTIONS")

	// User role management
	iamRouter.HandleFunc("/users", handleGetUsersWithRoles(iamService)).Methods("GET", "OPTIONS")
	iamRouter.HandleFunc("/users/{userId}/roles", handleAssignRole(iamService)).Methods("POST", "OPTIONS")
	iamRouter.HandleFunc("/users/{userId}/roles/{roleName}", handleRemoveRole(iamService)).Methods("DELETE", "OPTIONS")

	// Permission testing
	iamRouter.HandleFunc("/test-permission", handleTestPermission(iamService)).Methods("POST", "OPTIONS")

	// Audit logs
	log.Printf("RegisterIAMRoutes: Registering audit-logs endpoint")
	iamRouter.HandleFunc("/audit-logs", handleGetAuditLogs(iamService)).Methods("GET", "OPTIONS")
	log.Printf("RegisterIAMRoutes: audit-logs endpoint registered")
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

		// Format policies for API response
		formattedPolicies := make([]map[string]string, 0, len(policies))
		for _, policy := range policies {
			if len(policy) >= 4 {
				formattedPolicies = append(formattedPolicies, map[string]string{
					"subject":  policy[0],
					"resource": policy[1],
					"action":   policy[2],
					"effect":   policy[3],
				})
			}
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(formattedPolicies)
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
		log.Printf("handleGetAuditLogs: Called for path %s, method %s", r.URL.Path, r.Method)
		
		// Handle OPTIONS for CORS
		if r.Method == "OPTIONS" {
			w.WriteHeader(http.StatusOK)
			return
		}
		
		limit := r.URL.Query().Get("limit")
		filter := r.URL.Query().Get("filter")
		logType := r.URL.Query().Get("type")

		log.Printf("handleGetAuditLogs: Getting audit logs with limit=%s, filter=%s, type=%s", limit, filter, logType)
		
		logs, err := iamService.GetAuditLogs(r.Context(), limit, filter, logType)
		if err != nil {
			log.Printf("handleGetAuditLogs: Error getting audit logs: %v", err)
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		log.Printf("handleGetAuditLogs: Successfully retrieved %d audit logs", len(logs))
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(logs)
	}
}