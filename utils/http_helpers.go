package utils

import (
	"encoding/json"
	"net/http"
	"strconv"
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
