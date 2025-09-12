package iam

import (
	"encoding/json"
	"net/http"
	"strings"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/constants"
)

// Handlers provides HTTP handlers for IAM endpoints
type Handlers struct {
	service *Service
}

// NewHandlers creates new IAM handlers
func NewHandlers(service *Service) *Handlers {
	return &Handlers{
		service: service,
	}
}

// RegisterRoutes registers IAM routes
func (h *Handlers) RegisterRoutes(router *mux.Router) {
	// Role management
	router.HandleFunc("/api/iam/roles", h.GetRoles).Methods("GET")
	router.HandleFunc("/api/iam/roles", h.CreateRole).Methods("POST")
	router.HandleFunc("/api/iam/roles/{id}", h.GetRole).Methods("GET")
	router.HandleFunc("/api/iam/roles/{id}", h.UpdateRole).Methods("PUT")
	router.HandleFunc("/api/iam/roles/{id}", h.DeleteRole).Methods("DELETE")
	
	// Permission management
	router.HandleFunc("/api/iam/permissions", h.GetPermissions).Methods("GET")
	router.HandleFunc("/api/iam/permissions", h.CreatePermission).Methods("POST")
	
	// Policy management
	router.HandleFunc("/api/iam/policies", h.GetPolicies).Methods("GET")
	router.HandleFunc("/api/iam/policies", h.CreatePolicy).Methods("POST")
	router.HandleFunc("/api/iam/policies", h.DeletePolicy).Methods("DELETE")
	router.HandleFunc("/api/iam/policies/role/{role}", h.GetRolePolicies).Methods("GET")
	
	// User role assignment
	router.HandleFunc("/api/iam/users/{userId}/roles", h.GetUserRoles).Methods("GET")
	router.HandleFunc("/api/iam/users/{userId}/roles", h.AssignRole).Methods("POST")
	router.HandleFunc("/api/iam/users/{userId}/roles/{role}", h.RemoveRole).Methods("DELETE")
	
	// Policy templates
	router.HandleFunc("/api/iam/templates", h.GetPolicyTemplates).Methods("GET")
	router.HandleFunc("/api/iam/templates", h.CreatePolicyTemplate).Methods("POST")
	router.HandleFunc("/api/iam/templates/{id}/apply", h.ApplyPolicyTemplate).Methods("POST")
	
	// Evaluation and testing
	router.HandleFunc("/api/iam/evaluate", h.EvaluatePermission).Methods("POST")
	router.HandleFunc("/api/iam/audit", h.GetAuditLogs).Methods("GET")
}

// GetRoles gets all roles
func (h *Handlers) GetRoles(w http.ResponseWriter, r *http.Request) {
	roles, err := h.service.GetRoles(r.Context())
	if err != nil {
		WriteError(w, http.StatusInternalServerError, "Failed to get roles")
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(roles)
}

// CreateRole creates a new role
func (h *Handlers) CreateRole(w http.ResponseWriter, r *http.Request) {
	var role Role
	if err := json.NewDecoder(r.Body).Decode(&role); err != nil {
		WriteError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	if err := h.service.CreateRole(r.Context(), &role); err != nil {
		WriteError(w, http.StatusInternalServerError, "Failed to create role")
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(role)
}

// GetRole gets a specific role
func (h *Handlers) GetRole(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	roleID := vars["id"]

	var role Role
	if err := h.service.db.Where("id = ?", roleID).First(&role).Error; err != nil {
		WriteError(w, http.StatusNotFound, "Role not found")
		return
	}

	// Get policies for this role
	policies, _ := h.service.GetPoliciesForRole(r.Context(), role.Name)

	response := struct {
		Role
		Policies [][]string `json:"policies"`
	}{
		Role:     role,
		Policies: policies,
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(response)
}

// UpdateRole updates a role
func (h *Handlers) UpdateRole(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	roleID := vars["id"]

	var updates map[string]interface{}
	if err := json.NewDecoder(r.Body).Decode(&updates); err != nil {
		WriteError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	if err := h.service.UpdateRole(r.Context(), roleID, updates); err != nil {
		if strings.Contains(err.Error(), "system role") {
			WriteError(w, http.StatusForbidden, err.Error())
		} else {
			WriteError(w, http.StatusInternalServerError, "Failed to update role")
		}
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// DeleteRole deletes a role
func (h *Handlers) DeleteRole(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	roleID := vars["id"]

	if err := h.service.DeleteRole(r.Context(), roleID); err != nil {
		if strings.Contains(err.Error(), "system role") {
			WriteError(w, http.StatusForbidden, err.Error())
		} else {
			WriteError(w, http.StatusInternalServerError, "Failed to delete role")
		}
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// GetPermissions gets all permissions
func (h *Handlers) GetPermissions(w http.ResponseWriter, r *http.Request) {
	var permissions []Permission
	if err := h.service.db.Find(&permissions).Error; err != nil {
		WriteError(w, http.StatusInternalServerError, "Failed to get permissions")
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(permissions)
}

// CreatePermission creates a new permission
func (h *Handlers) CreatePermission(w http.ResponseWriter, r *http.Request) {
	var permission Permission
	if err := json.NewDecoder(r.Body).Decode(&permission); err != nil {
		WriteError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	if err := h.service.db.Create(&permission).Error; err != nil {
		WriteError(w, http.StatusInternalServerError, "Failed to create permission")
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(permission)
}

// GetPolicies gets all policies
func (h *Handlers) GetPolicies(w http.ResponseWriter, r *http.Request) {
	policies, _ := h.service.GetPolicies(r.Context())

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

// CreatePolicy creates a new policy
func (h *Handlers) CreatePolicy(w http.ResponseWriter, r *http.Request) {
	var req struct {
		Subject  string `json:"subject"`
		Resource string `json:"resource"`
		Action   string `json:"action"`
		Effect   string `json:"effect"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		WriteError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	if req.Effect == "" {
		req.Effect = "allow"
	}

	if err := h.service.AddPolicy(r.Context(), req.Subject, req.Resource, req.Action, req.Effect); err != nil {
		WriteError(w, http.StatusInternalServerError, "Failed to create policy")
		return
	}

	w.WriteHeader(http.StatusCreated)
}

// DeletePolicy deletes a policy
func (h *Handlers) DeletePolicy(w http.ResponseWriter, r *http.Request) {
	var req struct {
		Subject  string `json:"subject"`
		Resource string `json:"resource"`
		Action   string `json:"action"`
		Effect   string `json:"effect"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		WriteError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	if err := h.service.RemovePolicy(r.Context(), req.Subject, req.Resource, req.Action, req.Effect); err != nil {
		WriteError(w, http.StatusInternalServerError, "Failed to delete policy")
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// GetRolePolicies gets policies for a specific role
func (h *Handlers) GetRolePolicies(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	roleName := vars["role"]

	policies, _ := h.service.GetPoliciesForRole(r.Context(), roleName)

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

// GetUserRoles gets roles for a user
func (h *Handlers) GetUserRoles(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	userID := vars["userId"]

	roles, err := h.service.GetUserRoles(r.Context(), userID)
	if err != nil {
		WriteError(w, http.StatusInternalServerError, "Failed to get user roles")
		return
	}

	// Get role details
	roleDetails := make([]Role, 0, len(roles))
	for _, roleName := range roles {
		var role Role
		if err := h.service.db.Where("name = ?", roleName).First(&role).Error; err == nil {
			roleDetails = append(roleDetails, role)
		}
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(roleDetails)
}

// AssignRole assigns a role to a user
func (h *Handlers) AssignRole(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	userID := vars["userId"]

	var req struct {
		Role string `json:"role"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		WriteError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	// Get the current user ID from context for audit
	grantedBy := ""
	if currentUserID, ok := r.Context().Value(constants.ContextKeyUserID).(string); ok {
		grantedBy = currentUserID
	}

	if err := h.service.AssignRole(r.Context(), userID, req.Role, grantedBy); err != nil {
		WriteError(w, http.StatusInternalServerError, "Failed to assign role")
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// RemoveRole removes a role from a user
func (h *Handlers) RemoveRole(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	userID := vars["userId"]
	roleName := vars["role"]

	if err := h.service.RemoveRole(r.Context(), userID, roleName); err != nil {
		WriteError(w, http.StatusInternalServerError, "Failed to remove role")
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// GetPolicyTemplates gets all policy templates
func (h *Handlers) GetPolicyTemplates(w http.ResponseWriter, r *http.Request) {
	var templates []PolicyTemplate
	if err := h.service.db.Find(&templates).Error; err != nil {
		WriteError(w, http.StatusInternalServerError, "Failed to get templates")
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(templates)
}

// CreatePolicyTemplate creates a new policy template
func (h *Handlers) CreatePolicyTemplate(w http.ResponseWriter, r *http.Request) {
	var template PolicyTemplate
	if err := json.NewDecoder(r.Body).Decode(&template); err != nil {
		WriteError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	if err := h.service.db.Create(&template).Error; err != nil {
		WriteError(w, http.StatusInternalServerError, "Failed to create template")
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(template)
}

// ApplyPolicyTemplate applies a policy template to a role
func (h *Handlers) ApplyPolicyTemplate(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	templateID := vars["id"]

	var req struct {
		Role string `json:"role"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		WriteError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	if err := h.service.ApplyPolicyTemplate(r.Context(), req.Role, templateID); err != nil {
		WriteError(w, http.StatusInternalServerError, "Failed to apply template")
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// EvaluatePermission evaluates if a permission would be allowed
func (h *Handlers) EvaluatePermission(w http.ResponseWriter, r *http.Request) {
	var req struct {
		UserID   string                 `json:"user_id"`
		Resource string                 `json:"resource"`
		Action   string                 `json:"action"`
		Context  map[string]interface{} `json:"context,omitempty"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		WriteError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	var allowed bool
	var err error

	if req.Context != nil {
		allowed, err = h.service.CheckPermissionWithContext(r.Context(), req.UserID, req.Resource, req.Action, req.Context)
	} else {
		allowed, err = h.service.CheckPermission(r.Context(), req.UserID, req.Resource, req.Action)
	}

	if err != nil {
		WriteError(w, http.StatusInternalServerError, "Failed to evaluate permission")
		return
	}

	// Get matching policies for debugging
	allPolicies, _ := h.service.GetPolicies(r.Context())
	userRoles, _ := h.service.GetUserRoles(r.Context(), req.UserID)
	
	matchingPolicies := []map[string]string{}
	for _, policy := range allPolicies {
		if len(policy) >= 4 {
			// Check if policy applies to user or their roles
			policyApplies := false
			if policy[0] == req.UserID {
				policyApplies = true
			} else {
				for _, role := range userRoles {
					if policy[0] == role {
						policyApplies = true
						break
					}
				}
			}
			
			if policyApplies {
				// Check if resource and action match
				if matchResource(req.Resource, policy[1]) && matchAction(req.Action, policy[2]) {
					matchingPolicies = append(matchingPolicies, map[string]string{
						"subject":  policy[0],
						"resource": policy[1],
						"action":   policy[2],
						"effect":   policy[3],
					})
				}
			}
		}
	}

	response := struct {
		Allowed          bool                   `json:"allowed"`
		UserRoles        []string               `json:"user_roles"`
		MatchingPolicies []map[string]string    `json:"matching_policies"`
		Context          map[string]interface{} `json:"context,omitempty"`
	}{
		Allowed:          allowed,
		UserRoles:        userRoles,
		MatchingPolicies: matchingPolicies,
		Context:          req.Context,
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(response)
}

// GetAuditLogs gets IAM audit logs
func (h *Handlers) GetAuditLogs(w http.ResponseWriter, r *http.Request) {
	query := r.URL.Query()
	userID := query.Get("user_id")
	limit := 100

	var logs []IAMAuditLog
	q := h.service.db.Order("created_at DESC").Limit(limit)
	
	if userID != "" {
		q = q.Where("user_id = ?", userID)
	}
	
	if err := q.Find(&logs).Error; err != nil {
		WriteError(w, http.StatusInternalServerError, "Failed to get audit logs")
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(logs)
}

// Helper functions for pattern matching
func matchResource(requestPath string, policyPattern string) bool {
	// Simple wildcard matching
	if policyPattern == "*" {
		return true
	}
	if strings.HasSuffix(policyPattern, "*") {
		prefix := strings.TrimSuffix(policyPattern, "*")
		return strings.HasPrefix(requestPath, prefix)
	}
	return requestPath == policyPattern
}

func matchAction(requestAction string, policyAction string) bool {
	// Simple wildcard and regex matching
	if policyAction == "*" {
		return true
	}
	if strings.Contains(policyAction, "|") {
		actions := strings.Split(policyAction, "|")
		for _, action := range actions {
			if requestAction == action {
				return true
			}
		}
	}
	return requestAction == policyAction
}