package web

import (
	"encoding/json"
	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/extensions/core"
	"net/http"
)

// ExtensionsHandler handles the main extensions management page
func (h *Handler) ExtensionsHandler(w http.ResponseWriter, r *http.Request) {
	// Get all registered extensions
	extensions := h.extensionRegistry.GetAll()

	// Prepare extension data for display
	var extensionData []map[string]interface{}
	for _, ext := range extensions {
		metadata := ext.Metadata()
		status, _ := h.extensionRegistry.GetStatus(metadata.Name)

		// Default status if not found
		enabled := false
		state := "unknown"
		var health interface{} = nil

		if status != nil {
			enabled = status.Enabled
			state = status.State
			health = status.Health
		}

		data := map[string]interface{}{
			"name":        metadata.Name,
			"version":     metadata.Version,
			"description": metadata.Description,
			"author":      metadata.Author,
			"tags":        metadata.Tags,
			"enabled":     enabled,
			"state":       state,
			"health":      health,
		}

		// Add dashboard URL if extension has a dashboard
		if dashExt, ok := ext.(core.ExtensionWithDashboard); ok {
			if dashboardPath := dashExt.DashboardPath(); dashboardPath != "" {
				data["dashboardUrl"] = "/ext/" + metadata.Name + "/" + dashboardPath
			}
		}

		extensionData = append(extensionData, data)
	}

	// Render the extensions management page
	h.renderExtensionsPage(w, extensionData)
}

// ExtensionToggleHandler handles enabling/disabling extensions
func (h *Handler) ExtensionToggleHandler(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	name := vars["name"]

	var req struct {
		Enabled bool `json:"enabled"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request", http.StatusBadRequest)
		return
	}

	var err error
	if req.Enabled {
		err = h.extensionRegistry.Enable(name)
	} else {
		err = h.extensionRegistry.Disable(name)
	}

	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"success": true,
		"enabled": req.Enabled,
	})
}

// ExtensionStatusHandler returns the status of all extensions
func (h *Handler) ExtensionStatusHandler(w http.ResponseWriter, r *http.Request) {
	extensions := h.extensionRegistry.GetAll()

	var statuses []map[string]interface{}
	for _, ext := range extensions {
		metadata := ext.Metadata()
		status, _ := h.extensionRegistry.GetStatus(metadata.Name)
		health, _ := ext.Health(r.Context())

		// Default values if status not found
		enabled := false
		state := "unknown"

		if status != nil {
			enabled = status.Enabled
			state = status.State
		}

		statuses = append(statuses, map[string]interface{}{
			"name":    metadata.Name,
			"enabled": enabled,
			"state":   state,
			"health":  health,
		})
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(statuses)
}

// renderExtensionsPage renders the HTML for the extensions management page
func (h *Handler) renderExtensionsPage(w http.ResponseWriter, extensions []map[string]interface{}) {
	html := `<!DOCTYPE html>
<html>
<head>
    <title>Extensions Management</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { 
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            background: #f3f4f6;
            padding: 2rem;
        }
        .header {
            background: white;
            border-radius: 12px;
            padding: 2rem;
            margin-bottom: 2rem;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
        }
        .header-content {
            display: flex;
            align-items: center;
            gap: 1.5rem;
        }
        .header-icon {
            width: 60px;
            height: 60px;
            background: linear-gradient(135deg, #8b5cf6 0%, #7c3aed 100%);
            border-radius: 12px;
            display: flex;
            align-items: center;
            justify-content: center;
            color: white;
            font-size: 24px;
        }
        h1 { color: #1f2937; font-size: 1.875rem; margin-bottom: 0.5rem; }
        .description { color: #6b7280; }
        
        .extensions-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(400px, 1fr));
            gap: 1.5rem;
        }
        
        .extension-card {
            background: white;
            border-radius: 12px;
            padding: 1.5rem;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
            transition: transform 0.2s, box-shadow 0.2s;
        }
        
        .extension-card:hover {
            transform: translateY(-2px);
            box-shadow: 0 4px 6px rgba(0,0,0,0.1);
        }
        
        .extension-header {
            display: flex;
            justify-content: space-between;
            align-items: start;
            margin-bottom: 1rem;
        }
        
        .extension-info {
            flex: 1;
        }
        
        .extension-name {
            font-size: 1.25rem;
            font-weight: 600;
            color: #1f2937;
            margin-bottom: 0.25rem;
        }
        
        .extension-version {
            display: inline-block;
            padding: 0.125rem 0.5rem;
            background: #e5e7eb;
            color: #4b5563;
            border-radius: 999px;
            font-size: 0.75rem;
            margin-left: 0.5rem;
        }
        
        .extension-author {
            color: #6b7280;
            font-size: 0.875rem;
            margin-bottom: 0.5rem;
        }
        
        .extension-description {
            color: #4b5563;
            font-size: 0.875rem;
            line-height: 1.5;
            margin-bottom: 1rem;
        }
        
        .extension-tags {
            display: flex;
            flex-wrap: wrap;
            gap: 0.5rem;
            margin-bottom: 1rem;
        }
        
        .tag {
            padding: 0.25rem 0.75rem;
            background: #f3f4f6;
            color: #6b7280;
            border-radius: 999px;
            font-size: 0.75rem;
        }
        
        .extension-controls {
            display: flex;
            gap: 1rem;
            align-items: center;
        }
        
        .toggle-switch {
            position: relative;
            width: 48px;
            height: 24px;
            background: #e5e7eb;
            border-radius: 999px;
            cursor: pointer;
            transition: background 0.3s;
        }
        
        .toggle-switch.enabled {
            background: #10b981;
        }
        
        .toggle-switch .toggle-handle {
            position: absolute;
            top: 2px;
            left: 2px;
            width: 20px;
            height: 20px;
            background: white;
            border-radius: 999px;
            transition: transform 0.3s;
            box-shadow: 0 1px 3px rgba(0,0,0,0.2);
        }
        
        .toggle-switch.enabled .toggle-handle {
            transform: translateX(24px);
        }
        
        .btn {
            padding: 0.5rem 1rem;
            border-radius: 0.5rem;
            border: none;
            cursor: pointer;
            font-weight: 500;
            transition: all 0.2s;
            font-size: 0.875rem;
            text-decoration: none;
            display: inline-block;
        }
        
        .btn-primary {
            background: #3b82f6;
            color: white;
        }
        
        .btn-primary:hover {
            background: #2563eb;
        }
        
        .btn-secondary {
            background: white;
            color: #4b5563;
            border: 1px solid #e5e7eb;
        }
        
        .btn-secondary:hover {
            background: #f9fafb;
        }
        
        .status-badge {
            padding: 0.25rem 0.75rem;
            border-radius: 999px;
            font-size: 0.75rem;
            font-weight: 500;
        }
        
        .status-enabled {
            background: #d1fae5;
            color: #065f46;
        }
        
        .status-disabled {
            background: #fee2e2;
            color: #991b1b;
        }
        
        .official-badge {
            display: inline-block;
            padding: 0.125rem 0.5rem;
            background: #dbeafe;
            color: #1e40af;
            border-radius: 999px;
            font-size: 0.75rem;
            font-weight: 500;
            margin-left: 0.5rem;
        }
    </style>
</head>
<body>
    <div class="header">
        <div class="header-content">
            <div class="header-icon">🧩</div>
            <div>
                <h1>Extensions Management</h1>
                <p class="description">Enable, disable, and configure extensions to enhance your application</p>
            </div>
        </div>
    </div>
    
    <div class="extensions-grid" id="extensionsGrid">
        <!-- Extensions will be loaded here -->
    </div>
    
    <script>
        const extensions = ` + jsonMarshal(extensions) + `;
        
        function renderExtensions() {
            const grid = document.getElementById('extensionsGrid');
            
            if (!extensions || extensions.length === 0) {
                grid.innerHTML = '<p style="text-align: center; color: #6b7280; padding: 3rem;">No extensions available</p>';
                return;
            }
            
            grid.innerHTML = extensions.map(ext => {
                const isOfficial = ext.author === 'Solobase Official';
                const isEnabled = ext.enabled || false;
                
                return '<div class="extension-card">' +
                    '<div class="extension-header">' +
                        '<div class="extension-info">' +
                            '<div>' +
                                '<span class="extension-name">' + ext.name + '</span>' +
                                '<span class="extension-version">v' + ext.version + '</span>' +
                                (isOfficial ? '<span class="official-badge">Official</span>' : '') +
                            '</div>' +
                            '<div class="extension-author">by ' + ext.author + '</div>' +
                        '</div>' +
                        '<span class="status-badge ' + (isEnabled ? 'status-enabled' : 'status-disabled') + '">' +
                            (isEnabled ? 'Enabled' : 'Disabled') +
                        '</span>' +
                    '</div>' +
                    '<p class="extension-description">' + ext.description + '</p>' +
                    '<div class="extension-tags">' +
                        (ext.tags || []).map(tag => '<span class="tag">' + tag + '</span>').join('') +
                    '</div>' +
                    '<div class="extension-controls">' +
                        '<div class="toggle-switch ' + (isEnabled ? 'enabled' : '') + '" ' +
                            'onclick="toggleExtension(\'' + ext.name + '\', ' + !isEnabled + ')">' +
                            '<div class="toggle-handle"></div>' +
                        '</div>' +
                        (ext.dashboardUrl && isEnabled ? 
                            '<a href="' + ext.dashboardUrl + '" class="btn btn-primary">Open Dashboard</a>' : 
                            '<button class="btn btn-secondary" disabled>Dashboard ' + (isEnabled ? 'Not Available' : 'Disabled') + '</button>'
                        ) +
                    '</div>' +
                '</div>';
            }).join('');
        }
        
        async function toggleExtension(name, enable) {
            try {
                const response = await fetch('/admin/extensions/' + name + '/toggle', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ enabled: enable })
                });
                
                if (response.ok) {
                    // Update the extension in our local array
                    const ext = extensions.find(e => e.name === name);
                    if (ext) {
                        ext.enabled = enable;
                        renderExtensions();
                    }
                } else {
                    alert('Failed to ' + (enable ? 'enable' : 'disable') + ' extension');
                }
            } catch (err) {
                console.error('Error toggling extension:', err);
                alert('Failed to toggle extension');
            }
        }
        
        // Initial render
        renderExtensions();
        
        // Refresh status every 10 seconds
        setInterval(async () => {
            try {
                const response = await fetch('/admin/extensions/status');
                if (response.ok) {
                    const statuses = await response.json();
                    statuses.forEach(status => {
                        const ext = extensions.find(e => e.name === status.name);
                        if (ext) {
                            ext.enabled = status.enabled;
                            ext.health = status.health;
                        }
                    });
                    renderExtensions();
                }
            } catch (err) {
                console.error('Error fetching status:', err);
            }
        }, 10000);
    </script>
</body>
</html>`

	w.Header().Set("Content-Type", "text/html")
	w.Write([]byte(html))
}

func jsonMarshal(v interface{}) string {
	b, _ := json.Marshal(v)
	return string(b)
}
