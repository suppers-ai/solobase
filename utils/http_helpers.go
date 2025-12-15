package utils

import (
	"encoding/json"
	"net/http"
	"strconv"

	"github.com/suppers-ai/solobase/constants"
)

// JSONResponse writes a JSON response with the given status code
func JSONResponse(w http.ResponseWriter, status int, data interface{}) error {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	return json.NewEncoder(w).Encode(data)
}

// JSONError writes a JSON error response
func JSONError(w http.ResponseWriter, status int, message string) {
	JSONResponse(w, status, map[string]string{"error": message})
}

// JSONSuccess writes a successful JSON response
func JSONSuccess(w http.ResponseWriter, message string) {
	JSONResponse(w, http.StatusOK, map[string]string{"message": message})
}

// ParseJSONRequest decodes a JSON request body into the given struct
func ParseJSONRequest(r *http.Request, dest interface{}) error {
	decoder := json.NewDecoder(r.Body)
	decoder.DisallowUnknownFields()
	return decoder.Decode(dest)
}

// GetPaginationParams extracts pagination parameters from request
func GetPaginationParams(r *http.Request, defaultPageSize int) (page, pageSize, offset int) {
	page = 1
	if p := r.URL.Query().Get("page"); p != "" {
		if parsed, err := strconv.Atoi(p); err == nil && parsed > 0 {
			page = parsed
		}
	}

	pageSize = defaultPageSize
	if ps := r.URL.Query().Get("page_size"); ps != "" {
		if parsed, err := strconv.Atoi(ps); err == nil && parsed > 0 && parsed <= 100 {
			pageSize = parsed
		}
	}

	offset = (page - 1) * pageSize
	return
}

// IsHTMXRequest checks if the request is an HTMX request
func IsHTMXRequest(r *http.Request) bool {
	return r.Header.Get("HX-Request") == "true"
}

// MethodNotAllowed writes a method not allowed error
func MethodNotAllowed(w http.ResponseWriter, allowed ...string) {
	w.Header().Set("Allow", http.MethodOptions+", "+http.MethodGet)
	for _, method := range allowed {
		w.Header().Add("Allow", ", "+method)
	}
	http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
}

// DecodeJSONBody decodes JSON request body into dest and returns error response if failed
// Returns true if successful, false if failed (error response already sent)
func DecodeJSONBody(w http.ResponseWriter, r *http.Request, dest interface{}) bool {
	if err := json.NewDecoder(r.Body).Decode(dest); err != nil {
		JSONError(w, http.StatusBadRequest, "Invalid request body")
		return false
	}
	return true
}

// GetUserIDFromContext extracts user ID from request context
// Returns empty string if not found
func GetUserIDFromContext(r *http.Request) string {
	// Try typed context key first
	if userID, ok := r.Context().Value(constants.ContextKeyUserID).(string); ok && userID != "" {
		return userID
	}
	// Fallback to string key for backward compatibility
	if userID, ok := r.Context().Value("userID").(string); ok && userID != "" {
		return userID
	}
	if userID, ok := r.Context().Value("user_id").(string); ok && userID != "" {
		return userID
	}
	return ""
}

// GetUserRolesFromContext extracts user roles from request context
// Returns empty slice if not found
func GetUserRolesFromContext(r *http.Request) []string {
	if roles, ok := r.Context().Value(constants.ContextKeyUserRoles).([]string); ok {
		return roles
	}
	if roles, ok := r.Context().Value("user_roles").([]string); ok {
		return roles
	}
	return []string{}
}

// IsAdmin checks if user has admin role
func IsAdmin(r *http.Request) bool {
	roles := GetUserRolesFromContext(r)
	for _, role := range roles {
		if role == "admin" {
			return true
		}
	}
	return false
}

// RequireUserID extracts user ID from context and returns error response if not found
// Returns userID and true if successful, empty string and false if failed (error response already sent)
func RequireUserID(w http.ResponseWriter, r *http.Request) (string, bool) {
	userID := GetUserIDFromContext(r)
	if userID == "" {
		JSONError(w, http.StatusUnauthorized, "Unauthorized")
		return "", false
	}
	return userID, true
}

// PaginatedResult represents a paginated response
type PaginatedResult struct {
	Data       interface{} `json:"data"`
	Total      int         `json:"total"`
	Page       int         `json:"page"`
	PageSize   int         `json:"pageSize"`
	TotalPages int         `json:"totalPages"`
}

// NewPaginatedResult creates a new paginated result
func NewPaginatedResult(data interface{}, total, page, pageSize int) *PaginatedResult {
	totalPages := (total + pageSize - 1) / pageSize
	return &PaginatedResult{
		Data:       data,
		Total:      total,
		Page:       page,
		PageSize:   pageSize,
		TotalPages: totalPages,
	}
}

// SendPaginatedResponse writes a paginated JSON response
func SendPaginatedResponse(w http.ResponseWriter, data interface{}, total, page, pageSize int) {
	JSONResponse(w, http.StatusOK, NewPaginatedResult(data, total, page, pageSize))
}

// GetIntQueryParam extracts an integer query parameter with a default value
func GetIntQueryParam(r *http.Request, key string, defaultVal int) int {
	if val := r.URL.Query().Get(key); val != "" {
		if parsed, err := strconv.Atoi(val); err == nil {
			return parsed
		}
	}
	return defaultVal
}

// GetStringQueryParam extracts a string query parameter with a default value
func GetStringQueryParam(r *http.Request, key, defaultVal string) string {
	if val := r.URL.Query().Get(key); val != "" {
		return val
	}
	return defaultVal
}

// APIError represents an error with an associated HTTP status code
type APIError struct {
	Status  int    `json:"-"`
	Message string `json:"error"`
	Code    string `json:"code,omitempty"`
}

func (e *APIError) Error() string {
	return e.Message
}

// Common API errors
var (
	ErrBadRequest       = &APIError{Status: http.StatusBadRequest, Message: "Bad request", Code: "BAD_REQUEST"}
	ErrUnauthorized     = &APIError{Status: http.StatusUnauthorized, Message: "Unauthorized", Code: "UNAUTHORIZED"}
	ErrForbidden        = &APIError{Status: http.StatusForbidden, Message: "Forbidden", Code: "FORBIDDEN"}
	ErrNotFound         = &APIError{Status: http.StatusNotFound, Message: "Not found", Code: "NOT_FOUND"}
	ErrConflict         = &APIError{Status: http.StatusConflict, Message: "Resource already exists", Code: "CONFLICT"}
	ErrInternalServer   = &APIError{Status: http.StatusInternalServerError, Message: "Internal server error", Code: "INTERNAL_ERROR"}
	ErrServiceUnavailable = &APIError{Status: http.StatusServiceUnavailable, Message: "Service unavailable", Code: "SERVICE_UNAVAILABLE"}
)

// NewAPIError creates a new API error with a custom message
func NewAPIError(status int, message string) *APIError {
	return &APIError{Status: status, Message: message}
}

// NewBadRequest creates a bad request error with a custom message
func NewBadRequest(message string) *APIError {
	return &APIError{Status: http.StatusBadRequest, Message: message, Code: "BAD_REQUEST"}
}

// NewNotFound creates a not found error with a custom message
func NewNotFound(message string) *APIError {
	return &APIError{Status: http.StatusNotFound, Message: message, Code: "NOT_FOUND"}
}

// NewForbidden creates a forbidden error with a custom message
func NewForbidden(message string) *APIError {
	return &APIError{Status: http.StatusForbidden, Message: message, Code: "FORBIDDEN"}
}

// HandleAPIError sends an appropriate HTTP error response for an APIError
// If the error is not an APIError, it sends a 500 Internal Server Error
func HandleAPIError(w http.ResponseWriter, err error) {
	if apiErr, ok := err.(*APIError); ok {
		JSONError(w, apiErr.Status, apiErr.Message)
		return
	}
	JSONError(w, http.StatusInternalServerError, "Internal server error")
}

// WrapDBError wraps a database error into an appropriate APIError
func WrapDBError(err error, notFoundMsg string) *APIError {
	if err == nil {
		return nil
	}
	// Check for common GORM errors
	errStr := err.Error()
	if errStr == "record not found" {
		return NewNotFound(notFoundMsg)
	}
	return &APIError{Status: http.StatusInternalServerError, Message: "Database error", Code: "DB_ERROR"}
}

// SuccessResponse represents a standard success response
type SuccessResponse struct {
	Success bool        `json:"success"`
	Message string      `json:"message,omitempty"`
	Data    interface{} `json:"data,omitempty"`
}

// SendSuccess sends a standard success response
func SendSuccess(w http.ResponseWriter, message string, data interface{}) {
	JSONResponse(w, http.StatusOK, SuccessResponse{
		Success: true,
		Message: message,
		Data:    data,
	})
}

// SendCreated sends a 201 Created response
func SendCreated(w http.ResponseWriter, data interface{}) {
	JSONResponse(w, http.StatusCreated, data)
}

// SendNoContent sends a 204 No Content response
func SendNoContent(w http.ResponseWriter) {
	w.WriteHeader(http.StatusNoContent)
}
