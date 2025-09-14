package web

import (
	"context"
	"encoding/json"
	"net/http"
	"time"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/internal/pkg/logger"
	"github.com/suppers-ai/solobase/extensions"
	"github.com/suppers-ai/solobase/extensions/core"
)

// ExtensionsHandler handles the extensions management interface
type ExtensionsHandler struct {
	manager *extensions.ExtensionManager
	logger  logger.Logger
}

// NewExtensionsHandler creates a new extensions handler
func NewExtensionsHandler(manager *extensions.ExtensionManager, logger logger.Logger) *ExtensionsHandler {
	return &ExtensionsHandler{
		manager: manager,
		logger:  logger,
	}
}

// RegisterRoutes registers the extension management routes
func (h *ExtensionsHandler) RegisterRoutes(router *mux.Router) {
	// Main extensions page
	router.HandleFunc("/admin/extensions", h.ExtensionsPageHandler).Methods("GET")

	// API endpoints
	router.HandleFunc("/admin/extensions/api/list", h.ListExtensionsHandler).Methods("GET")
	router.HandleFunc("/admin/extensions/api/{name}/toggle", h.ToggleExtensionHandler).Methods("POST")
	router.HandleFunc("/admin/extensions/api/{name}/status", h.ExtensionStatusHandler).Methods("GET")
	router.HandleFunc("/admin/extensions/api/{name}/config", h.ExtensionConfigHandler).Methods("GET", "POST")
}

// ExtensionsPageHandler serves the main extensions management page
func (h *ExtensionsHandler) ExtensionsPageHandler(w http.ResponseWriter, r *http.Request) {
	// Get all registered extensions
	registry := h.manager.GetRegistry()
	extensions := registry.GetAll()

	// Debug: log the number of extensions
	h.logger.Info(r.Context(), "Extensions page requested",
		logger.Int("extension_count", len(extensions)))

	// Prepare extension data for display
	var extensionData []map[string]interface{}

	for _, ext := range extensions {
		metadata := ext.Metadata()

		// Get status
		status, _ := registry.GetStatus(metadata.Name)
		enabled := false
		state := "unknown"

		if status != nil {
			enabled = status.Enabled
			state = status.State
		}

		// Get health
		health, _ := ext.Health(r.Context())

		data := map[string]interface{}{
			"name":        metadata.Name,
			"version":     metadata.Version,
			"description": metadata.Description,
			"author":      metadata.Author,
			"tags":        metadata.Tags,
			"license":     metadata.License,
			"homepage":    metadata.Homepage,
			"enabled":     enabled,
			"state":       state,
			"health":      health,
		}

		// Add dashboard URL if extension has one
		if dashExt, ok := ext.(core.ExtensionWithDashboard); ok {
			if dashboardPath := dashExt.DashboardPath(); dashboardPath != "" {
				data["dashboardUrl"] = "/ext/" + metadata.Name + "/" + dashboardPath
			}
		}

		extensionData = append(extensionData, data)
	}

	// Render the page
	h.renderExtensionsPage(w, extensionData)
}

// ListExtensionsHandler returns a JSON list of all extensions
func (h *ExtensionsHandler) ListExtensionsHandler(w http.ResponseWriter, r *http.Request) {
	registry := h.manager.GetRegistry()
	extensions := registry.GetAll()

	var result []map[string]interface{}

	for _, ext := range extensions {
		metadata := ext.Metadata()
		status, _ := registry.GetStatus(metadata.Name)
		health, _ := ext.Health(r.Context())

		enabled := false
		state := "unknown"

		if status != nil {
			enabled = status.Enabled
			state = status.State
		}

		result = append(result, map[string]interface{}{
			"name":        metadata.Name,
			"version":     metadata.Version,
			"description": metadata.Description,
			"author":      metadata.Author,
			"tags":        metadata.Tags,
			"enabled":     enabled,
			"state":       state,
			"health":      health,
		})
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(result)
}

// ToggleExtensionHandler enables or disables an extension
func (h *ExtensionsHandler) ToggleExtensionHandler(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	name := vars["name"]

	var req struct {
		Enabled bool `json:"enabled"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request", http.StatusBadRequest)
		return
	}

	registry := h.manager.GetRegistry()

	var err error
	if req.Enabled {
		err = registry.Enable(name)
	} else {
		err = registry.Disable(name)
	}

	if err != nil {
		h.logger.Error(r.Context(), "Failed to toggle extension",
			logger.String("extension", name),
			logger.Bool("enable", req.Enabled),
			logger.Err(err))

		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	// Save the state to config
	h.manager.SaveExtensionState(name, req.Enabled)

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"success": true,
		"enabled": req.Enabled,
		"message": "Extension " + name + " has been " + (map[bool]string{true: "enabled", false: "disabled"}[req.Enabled]),
	})
}

// ExtensionStatusHandler returns the status of a specific extension
func (h *ExtensionsHandler) ExtensionStatusHandler(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	name := vars["name"]

	registry := h.manager.GetRegistry()

	ext, exists := registry.Get(name)
	if !exists {
		http.Error(w, "Extension not found", http.StatusNotFound)
		return
	}

	metadata := ext.Metadata()
	status, _ := registry.GetStatus(name)
	health, _ := ext.Health(r.Context())

	enabled := false
	state := "unknown"

	if status != nil {
		enabled = status.Enabled
		state = status.State
	}

	result := map[string]interface{}{
		"name":        metadata.Name,
		"version":     metadata.Version,
		"description": metadata.Description,
		"enabled":     enabled,
		"state":       state,
		"health":      health,
		"lastChecked": time.Now(),
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(result)
}

// ExtensionConfigHandler handles extension configuration
func (h *ExtensionsHandler) ExtensionConfigHandler(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	name := vars["name"]

	registry := h.manager.GetRegistry()

	ext, exists := registry.Get(name)
	if !exists {
		http.Error(w, "Extension not found", http.StatusNotFound)
		return
	}

	if r.Method == "GET" {
		// Get current config schema
		config := ext.ConfigSchema()
		w.Header().Set("Content-Type", "application/json")
		w.Write(config)
	} else {
		// Update config
		var config json.RawMessage
		if err := json.NewDecoder(r.Body).Decode(&config); err != nil {
			http.Error(w, "Invalid config", http.StatusBadRequest)
			return
		}

		// Validate config
		if err := ext.ValidateConfig(config); err != nil {
			http.Error(w, "Invalid config: "+err.Error(), http.StatusBadRequest)
			return
		}

		// Apply config
		if err := ext.ApplyConfig(config); err != nil {
			http.Error(w, "Failed to apply config: "+err.Error(), http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]interface{}{
			"success": true,
			"message": "Configuration updated successfully",
		})
	}
}

// renderExtensionsPage renders the HTML for the extensions management page
func (h *ExtensionsHandler) renderExtensionsPage(w http.ResponseWriter, extensions []map[string]interface{}) {
	ctx := context.Background()

	// Log the page render
	h.logger.Info(ctx, "Rendering extensions page",
		logger.Int("extension_count", len(extensions)))

	html := generateExtensionsHTML(extensions)

	w.Header().Set("Content-Type", "text/html; charset=utf-8")
	w.Write([]byte(html))
}

// generateExtensionsHTML generates the HTML for the extensions page
func generateExtensionsHTML(extensions []map[string]interface{}) string {
	// Convert extensions to JSON for JavaScript
	extensionsJSON, _ := json.Marshal(extensions)

	return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Extensions Management - Solobase</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        
        body { 
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            padding: 2rem;
        }
        
        .container {
            max-width: 1400px;
            margin: 0 auto;
        }
        
        .header {
            background: rgba(255, 255, 255, 0.95);
            backdrop-filter: blur(10px);
            border-radius: 16px;
            padding: 2rem;
            margin-bottom: 2rem;
            box-shadow: 0 20px 40px rgba(0,0,0,0.1);
        }
        
        .header-content {
            display: flex;
            align-items: center;
            gap: 1.5rem;
        }
        
        .header-icon {
            width: 64px;
            height: 64px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            border-radius: 16px;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 32px;
            box-shadow: 0 10px 20px rgba(102, 126, 234, 0.3);
        }
        
        h1 { 
            color: #1a202c; 
            font-size: 2rem; 
            font-weight: 700;
            margin-bottom: 0.5rem; 
        }
        
        .subtitle { 
            color: #718096; 
            font-size: 1.125rem;
        }
        
        .extensions-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(420px, 1fr));
            gap: 1.5rem;
        }
        
        .extension-card {
            background: rgba(255, 255, 255, 0.95);
            backdrop-filter: blur(10px);
            border-radius: 16px;
            padding: 1.5rem;
            box-shadow: 0 10px 30px rgba(0,0,0,0.1);
            transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
            border: 1px solid rgba(255, 255, 255, 0.8);
        }
        
        .extension-card:hover {
            transform: translateY(-4px);
            box-shadow: 0 20px 40px rgba(0,0,0,0.15);
        }
        
        .extension-header {
            display: flex;
            justify-content: space-between;
            align-items: start;
            margin-bottom: 1rem;
        }
        
        .extension-info { flex: 1; }
        
        .extension-name {
            font-size: 1.375rem;
            font-weight: 600;
            color: #2d3748;
            margin-bottom: 0.25rem;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }
        
        .version-badge {
            display: inline-block;
            padding: 0.25rem 0.75rem;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            border-radius: 999px;
            font-size: 0.75rem;
            font-weight: 600;
        }
        
        .extension-author {
            color: #718096;
            font-size: 0.875rem;
            margin-bottom: 0.75rem;
        }
        
        .extension-description {
            color: #4a5568;
            font-size: 0.9375rem;
            line-height: 1.6;
            margin-bottom: 1rem;
            min-height: 3rem;
        }
        
        .extension-tags {
            display: flex;
            flex-wrap: wrap;
            gap: 0.5rem;
            margin-bottom: 1rem;
        }
        
        .tag {
            padding: 0.375rem 0.875rem;
            background: #f7fafc;
            color: #4a5568;
            border-radius: 999px;
            font-size: 0.8125rem;
            font-weight: 500;
            border: 1px solid #e2e8f0;
        }
        
        .extension-footer {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding-top: 1rem;
            border-top: 1px solid #e2e8f0;
        }
        
        .toggle-container {
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }
        
        .toggle-label {
            font-size: 0.875rem;
            color: #4a5568;
            font-weight: 500;
        }
        
        .toggle-switch {
            position: relative;
            width: 52px;
            height: 28px;
            background: #cbd5e0;
            border-radius: 999px;
            cursor: pointer;
            transition: all 0.3s;
            box-shadow: inset 0 2px 4px rgba(0,0,0,0.1);
        }
        
        .toggle-switch.enabled {
            background: linear-gradient(135deg, #48bb78 0%, #38a169 100%);
        }
        
        .toggle-switch .toggle-handle {
            position: absolute;
            top: 3px;
            left: 3px;
            width: 22px;
            height: 22px;
            background: white;
            border-radius: 999px;
            transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
            box-shadow: 0 2px 8px rgba(0,0,0,0.2);
        }
        
        .toggle-switch.enabled .toggle-handle {
            transform: translateX(24px);
        }
        
        .extension-actions {
            display: flex;
            gap: 0.75rem;
        }
        
        .btn {
            padding: 0.625rem 1.25rem;
            border-radius: 8px;
            border: none;
            cursor: pointer;
            font-weight: 600;
            font-size: 0.875rem;
            transition: all 0.2s;
            text-decoration: none;
            display: inline-flex;
            align-items: center;
            gap: 0.5rem;
        }
        
        .btn-primary {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            box-shadow: 0 4px 12px rgba(102, 126, 234, 0.3);
        }
        
        .btn-primary:hover {
            transform: translateY(-1px);
            box-shadow: 0 6px 16px rgba(102, 126, 234, 0.4);
        }
        
        .btn-secondary {
            background: white;
            color: #4a5568;
            border: 2px solid #e2e8f0;
        }
        
        .btn-secondary:hover {
            background: #f7fafc;
            border-color: #cbd5e0;
        }
        
        .btn:disabled {
            opacity: 0.5;
            cursor: not-allowed;
            transform: none !important;
        }
        
        .status-indicator {
            display: inline-flex;
            align-items: center;
            gap: 0.375rem;
            padding: 0.375rem 0.875rem;
            border-radius: 999px;
            font-size: 0.8125rem;
            font-weight: 600;
        }
        
        .status-enabled {
            background: #c6f6d5;
            color: #22543d;
        }
        
        .status-disabled {
            background: #fed7d7;
            color: #742a2a;
        }
        
        .status-dot {
            width: 8px;
            height: 8px;
            border-radius: 999px;
            background: currentColor;
            animation: pulse 2s infinite;
        }
        
        @keyframes pulse {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.5; }
        }
        
        .official-badge {
            display: inline-flex;
            align-items: center;
            gap: 0.25rem;
            padding: 0.25rem 0.625rem;
            background: linear-gradient(135deg, #3182ce 0%, #2c5282 100%);
            color: white;
            border-radius: 999px;
            font-size: 0.75rem;
            font-weight: 600;
        }
        
        .empty-state {
            background: rgba(255, 255, 255, 0.95);
            backdrop-filter: blur(10px);
            border-radius: 16px;
            padding: 4rem 2rem;
            text-align: center;
            box-shadow: 0 10px 30px rgba(0,0,0,0.1);
        }
        
        .empty-state-icon {
            font-size: 4rem;
            margin-bottom: 1rem;
        }
        
        .empty-state-title {
            font-size: 1.5rem;
            color: #2d3748;
            margin-bottom: 0.5rem;
        }
        
        .empty-state-text {
            color: #718096;
            font-size: 1rem;
        }
        
        .toast {
            position: fixed;
            bottom: 2rem;
            right: 2rem;
            padding: 1rem 1.5rem;
            background: white;
            border-radius: 8px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.2);
            display: none;
            animation: slideUp 0.3s ease-out;
            z-index: 1000;
        }
        
        .toast.show { display: block; }
        
        .toast.success { border-left: 4px solid #48bb78; }
        .toast.error { border-left: 4px solid #f56565; }
        
        @keyframes slideUp {
            from {
                transform: translateY(100%);
                opacity: 0;
            }
            to {
                transform: translateY(0);
                opacity: 1;
            }
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <div class="header-content">
                <div class="header-icon">🧩</div>
                <div>
                    <h1>Extensions Management</h1>
                    <p class="subtitle">Enable, disable, and configure extensions to enhance your application</p>
                </div>
            </div>
        </div>
        
        <div class="extensions-grid" id="extensionsGrid">
            <!-- Extensions will be loaded here -->
        </div>
    </div>
    
    <div class="toast" id="toast"></div>
    
    <script>
        const extensions = ` + string(extensionsJSON) + `;
        
        function renderExtensions() {
            const grid = document.getElementById('extensionsGrid');
            
            if (!extensions || extensions.length === 0) {
                grid.innerHTML = ` + "`" + `
                    <div class="empty-state">
                        <div class="empty-state-icon">📦</div>
                        <h2 class="empty-state-title">No Extensions Available</h2>
                        <p class="empty-state-text">Extensions will appear here once they are registered with the system.</p>
                    </div>
                ` + "`" + `;
                return;
            }
            
            grid.innerHTML = extensions.map(ext => {
                const isOfficial = ext.author === 'Solobase Official' || ext.author === 'Solobase Team';
                const isEnabled = ext.enabled || false;
                
                return ` + "`" + `
                    <div class="extension-card">
                        <div class="extension-header">
                            <div class="extension-info">
                                <div class="extension-name">
                                    ${ext.name}
                                    <span class="version-badge">v${ext.version}</span>
                                    ${isOfficial ? '<span class="official-badge">✓ Official</span>' : ''}
                                </div>
                                <div class="extension-author">by ${ext.author}</div>
                                <div class="status-indicator ${isEnabled ? 'status-enabled' : 'status-disabled'}">
                                    <span class="status-dot"></span>
                                    ${isEnabled ? 'Enabled' : 'Disabled'}
                                </div>
                            </div>
                        </div>
                        
                        <p class="extension-description">${ext.description}</p>
                        
                        ${ext.tags && ext.tags.length > 0 ? ` + "`" + `
                            <div class="extension-tags">
                                ${ext.tags.map(tag => ` + "`" + `<span class="tag">${tag}</span>` + "`" + `).join('')}
                            </div>
                        ` + "`" + ` : ''}
                        
                        <div class="extension-footer">
                            <div class="toggle-container">
                                <span class="toggle-label">Status:</span>
                                <div class="toggle-switch ${isEnabled ? 'enabled' : ''}" 
                                    onclick="toggleExtension('${ext.name}', ${!isEnabled})">
                                    <div class="toggle-handle"></div>
                                </div>
                            </div>
                            
                            <div class="extension-actions">
                                ${ext.dashboardUrl && isEnabled ? ` + "`" + `
                                    <a href="${ext.dashboardUrl}" class="btn btn-primary">
                                        <span>📊</span> Dashboard
                                    </a>
                                ` + "`" + ` : ` + "`" + `
                                    <button class="btn btn-secondary" disabled>
                                        <span>📊</span> Dashboard
                                    </button>
                                ` + "`" + `}
                            </div>
                        </div>
                    </div>
                ` + "`" + `;
            }).join('');
        }
        
        async function toggleExtension(name, enable) {
            try {
                const response = await fetch('/admin/extensions/api/' + name + '/toggle', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ enabled: enable })
                });
                
                const data = await response.json();
                
                if (response.ok && data.success) {
                    // Update local state
                    const ext = extensions.find(e => e.name === name);
                    if (ext) {
                        ext.enabled = enable;
                        renderExtensions();
                    }
                    
                    showToast(data.message || ('Extension ' + (enable ? 'enabled' : 'disabled')), 'success');
                } else {
                    showToast('Failed to ' + (enable ? 'enable' : 'disable') + ' extension', 'error');
                }
            } catch (err) {
                console.error('Error toggling extension:', err);
                showToast('Failed to toggle extension', 'error');
            }
        }
        
        function showToast(message, type = 'success') {
            const toast = document.getElementById('toast');
            toast.className = 'toast show ' + type;
            toast.textContent = message;
            
            setTimeout(() => {
                toast.classList.remove('show');
            }, 3000);
        }
        
        // Initial render
        renderExtensions();
        
        // Refresh status periodically
        setInterval(async () => {
            try {
                const response = await fetch('/admin/extensions/api/list');
                if (response.ok) {
                    const updatedExtensions = await response.json();
                    // Update local state
                    updatedExtensions.forEach(updated => {
                        const ext = extensions.find(e => e.name === updated.name);
                        if (ext) {
                            ext.enabled = updated.enabled;
                            ext.health = updated.health;
                            ext.state = updated.state;
                        }
                    });
                    renderExtensions();
                }
            } catch (err) {
                console.error('Error fetching extension status:', err);
            }
        }, 15000);
    </script>
</body>
</html>`
}
