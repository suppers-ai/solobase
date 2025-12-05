package hugo

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
)

// listSites handles GET /sites
func (e *HugoExtension) listSites(w http.ResponseWriter, r *http.Request) {
	// Get user ID from context (would come from auth middleware)
	userID := r.Context().Value("user_id")
	if userID == nil {
		userID = "default-user" // For development
	}

	sites, err := e.service.ListSites(fmt.Sprintf("%v", userID))
	if err != nil {
		http.Error(w, fmt.Sprintf("Failed to list sites: %v", err), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(sites)
}

// createSite handles POST /sites
func (e *HugoExtension) createSite(w http.ResponseWriter, r *http.Request) {
	var req struct {
		Name      string `json:"name"`
		Domain    string `json:"domain"`
		Theme     string `json:"theme"`
		IsExample bool   `json:"isExample"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request body", http.StatusBadRequest)
		return
	}

	// Validate required fields
	if req.Name == "" {
		http.Error(w, "Name is required", http.StatusBadRequest)
		return
	}

	// Get user ID from context
	userID := r.Context().Value("user_id")
	if userID == nil {
		userID = "default-user" // For development
	}

	// Set default theme if not provided
	if req.Theme == "" {
		req.Theme = e.config.DefaultTheme
	}

	site, err := e.service.CreateSite(fmt.Sprintf("%v", userID), req.Name, req.Domain, req.Theme, req.IsExample)
	if err != nil {
		http.Error(w, fmt.Sprintf("Failed to create site: %v", err), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(site)
}

// getSite handles GET /sites/{id}
func (e *HugoExtension) getSite(w http.ResponseWriter, r *http.Request) {
	siteID := extractIDFromPath(r.URL.Path)
	if siteID == "" {
		http.Error(w, "Site ID is required", http.StatusBadRequest)
		return
	}

	site, err := e.service.GetSite(siteID)
	if err != nil {
		http.Error(w, fmt.Sprintf("Failed to get site: %v", err), http.StatusNotFound)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(site)
}

// deleteSite handles DELETE /sites/{id}
func (e *HugoExtension) deleteSite(w http.ResponseWriter, r *http.Request) {
	siteID := extractIDFromPath(r.URL.Path)
	if siteID == "" {
		http.Error(w, "Site ID is required", http.StatusBadRequest)
		return
	}

	if err := e.service.DeleteSite(siteID); err != nil {
		http.Error(w, fmt.Sprintf("Failed to delete site: %v", err), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// buildSite handles POST /sites/{id}/build
func (e *HugoExtension) buildSite(w http.ResponseWriter, r *http.Request) {
	siteID := extractIDFromPath(r.URL.Path)
	if siteID == "" {
		http.Error(w, "Site ID is required", http.StatusBadRequest)
		return
	}

	result, err := e.service.BuildSite(siteID)
	if err != nil {
		http.Error(w, fmt.Sprintf("Failed to build site: %v", err), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(result)
}

// listFiles handles GET /sites/{id}/files
func (e *HugoExtension) listFiles(w http.ResponseWriter, r *http.Request) {
	siteID := extractIDFromPath(r.URL.Path)
	if siteID == "" {
		http.Error(w, "Site ID is required", http.StatusBadRequest)
		return
	}

	files, err := e.service.ListFiles(siteID)
	if err != nil {
		http.Error(w, fmt.Sprintf("Failed to list files: %v", err), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(files)
}

// readFile handles POST /sites/{id}/files/read
func (e *HugoExtension) readFile(w http.ResponseWriter, r *http.Request) {
	siteID := extractIDFromPath(r.URL.Path)
	if siteID == "" {
		http.Error(w, "Site ID is required", http.StatusBadRequest)
		return
	}

	var req struct {
		Path string `json:"path"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request body", http.StatusBadRequest)
		return
	}

	if req.Path == "" {
		http.Error(w, "File path is required", http.StatusBadRequest)
		return
	}

	content, err := e.service.ReadFile(siteID, req.Path)
	if err != nil {
		http.Error(w, fmt.Sprintf("Failed to read file: %v", err), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"content": content,
	})
}

// saveFile handles POST /sites/{id}/files/save
func (e *HugoExtension) saveFile(w http.ResponseWriter, r *http.Request) {
	siteID := extractIDFromPath(r.URL.Path)
	if siteID == "" {
		http.Error(w, "Site ID is required", http.StatusBadRequest)
		return
	}

	var req struct {
		Path    string `json:"path"`
		Content string `json:"content"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request body", http.StatusBadRequest)
		return
	}

	if req.Path == "" {
		http.Error(w, "File path is required", http.StatusBadRequest)
		return
	}

	if err := e.service.SaveFile(siteID, req.Path, req.Content); err != nil {
		http.Error(w, fmt.Sprintf("Failed to save file: %v", err), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"status": "success",
	})
}

// getStats handles GET /stats
func (e *HugoExtension) getStats(w http.ResponseWriter, r *http.Request) {
	// Get user ID from context
	userID := r.Context().Value("user_id")
	if userID == nil {
		userID = "default-user" // For development
	}

	stats, err := e.service.GetStats(fmt.Sprintf("%v", userID))
	if err != nil {
		http.Error(w, fmt.Sprintf("Failed to get stats: %v", err), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(stats)
}

// Helper function to extract ID from path like "/sites/123/build"
func extractIDFromPath(path string) string {
	parts := strings.Split(strings.Trim(path, "/"), "/")
	if len(parts) >= 2 {
		return parts[len(parts)-2]
	}
	if len(parts) >= 1 {
		return parts[len(parts)-1]
	}
	return ""
}
