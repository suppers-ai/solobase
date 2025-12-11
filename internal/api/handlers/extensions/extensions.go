package extensions

import (
	"encoding/json"
	"fmt"
	"net/http"

	"github.com/gorilla/mux"
	"github.com/suppers-ai/solobase/extensions/core"
	"github.com/suppers-ai/solobase/extensions/official/cloudstorage"
	// "github.com/suppers-ai/solobase/extensions/official/hugo" // Temporarily disabled - needs API updates
	"github.com/suppers-ai/solobase/extensions/official/products"
	"github.com/suppers-ai/solobase/extensions/official/webhooks"
	"github.com/suppers-ai/solobase/utils"
)

// Global extension registry - in production this should be passed properly
var globalExtensionRegistry *core.ExtensionRegistry

// InitializeExtensions initializes the extension system
func InitializeExtensions() error {
	// This should be called from main.go with proper services
	// For now, we'll create a simple registry
	if globalExtensionRegistry == nil {
		globalExtensionRegistry = &core.ExtensionRegistry{}
	}
	return nil
}

// HandleGetExtensions returns all extensions
func HandleGetExtensions() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Create a list of all available extensions with their metadata
		tempExtensions := []core.Extension{
			products.NewProductsExtension(),
			// hugo.NewHugoExtension(), // Temporarily disabled
			// analytics.NewAnalyticsExtension(), // Now registered via extension registry
			cloudstorage.NewCloudStorageExtension(nil),
			webhooks.NewWebhooksExtension(),
		}

		extensions := []map[string]interface{}{}
		for _, ext := range tempExtensions {
			metadata := ext.Metadata()
			extensions = append(extensions, map[string]interface{}{
				"name":        metadata.Name,
				"version":     metadata.Version,
				"description": metadata.Description,
				"author":      metadata.Author,
				"tags":        metadata.Tags,
				"license":     metadata.License,
				"homepage":    metadata.Homepage,
				"enabled":     true, // For now, show all as enabled
				"state":       "healthy",
			})
		}

		utils.JSONResponse(w, http.StatusOK, extensions)
	}
}

// HandleExtensionsManagement returns the extensions management page with actual data
func HandleExtensionsManagement() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Create extension cards HTML
		extensionCards := ""

		// Create a temporary registry to get metadata
		tempExtensions := []core.Extension{
			products.NewProductsExtension(),
			// hugo.NewHugoExtension(), // Temporarily disabled
			// analytics.NewAnalyticsExtension(), // Now registered via extension registry
			cloudstorage.NewCloudStorageExtension(nil),
			webhooks.NewWebhooksExtension(),
		}

		for _, ext := range tempExtensions {
			metadata := ext.Metadata()

			// Create tags HTML
			tagsHTML := ""
			for _, tag := range metadata.Tags {
				tagsHTML += fmt.Sprintf(`<span class="tag">%s</span>`, tag)
			}

			extensionCards += fmt.Sprintf(`
				<div class="extension-card">
					<div class="extension-header">
						<div class="extension-info">
							<div style="display: flex; align-items: center; gap: 0.75rem; margin-bottom: 0.5rem;">
								<span style="font-size: 1.5rem;">%s</span>
								<div>
									<span class="extension-name">%s</span>
									<span class="extension-version">v%s</span>
								</div>
							</div>
							<div class="extension-author">by %s</div>
						</div>
						<div class="toggle-switch enabled" id="toggle-%s">
							<div class="toggle-handle"></div>
						</div>
					</div>
					<p class="extension-description">%s</p>
					<div class="extension-tags">%s</div>
					<div class="extension-controls">
						<span class="status-badge status-enabled">Enabled</span>
					</div>
				</div>
			`,
				getExtensionIcon(metadata.Name),
				metadata.Name,
				metadata.Version,
				metadata.Author,
				metadata.Name,
				metadata.Description,
				tagsHTML,
			)
		}

		// If no extensions, show empty state
		if extensionCards == "" {
			extensionCards = `
				<div style="grid-column: 1 / -1; text-align: center; padding: 4rem 2rem; color: #6b7280;">
					<div style="font-size: 3rem; margin-bottom: 1rem;">🧩</div>
					<p style="font-size: 1.125rem;">No extensions available</p>
					<p style="margin-top: 0.5rem;">Extensions will appear here once they are registered</p>
				</div>
			`
		}

		html := fmt.Sprintf(`<!DOCTYPE html>
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
            background: linear-gradient(135deg, #8b5cf6 0%%, #7c3aed 100%%);
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
        %s
    </div>
    
    <script>
        async function loadExtensions() {
            try {
                const response = await fetch('/api/extensions');
                if (!response.ok) throw new Error('Failed to load extensions');
                
                const extensions = await response.json();
                renderExtensions(extensions);
            } catch (err) {
                console.error('Error loading extensions:', err);
                document.getElementById('extensionsGrid').innerHTML = 
                    '<p style="text-align: center; color: #ef4444;">Failed to load extensions</p>';
            }
        }
        
        function renderExtensions(extensions) {
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
                const response = await fetch('/api/extensions/' + name + '/toggle', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ enabled: enable })
                });
                
                if (response.ok) {
                    loadExtensions(); // Reload to show updated state
                } else {
                    alert('Failed to ' + (enable ? 'enable' : 'disable') + ' extension');
                }
            } catch (err) {
                console.error('Error toggling extension:', err);
                alert('Failed to toggle extension');
            }
        }
        
        // Load extensions on page load
        loadExtensions();
        
        // Refresh status every 10 seconds
        setInterval(loadExtensions, 10000);
    </script>
</body>
</html>`, extensionCards)

		w.Header().Set("Content-Type", "text/html")
		w.Write([]byte(html))
	}
}

// getExtensionIcon returns an icon for the extension
func getExtensionIcon(name string) string {
	icons := map[string]string{
		"Products & Pricing": "📦",
		// "hugo":               "🌐", // Temporarily disabled
		"analytics":    "📊",
		"cloudstorage": "☁️",
		"webhooks":     "🔗",
	}

	if icon, ok := icons[name]; ok {
		return icon
	}
	return "🧩"
}

// HandleToggleExtension toggles an extension on/off
func HandleToggleExtension() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		name := vars["name"]

		var req struct {
			Enabled bool `json:"enabled"`
		}

		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid request")
			return
		}

		// For now, just return success
		// In production, this would actually toggle the extension
		utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
			"success": true,
			"name":    name,
			"enabled": req.Enabled,
		})
	}
}

// HandleExtensionsStatus returns the status of all extensions
func HandleExtensionsStatus() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Mock status data
		statuses := []map[string]interface{}{
			{
				"name":    "analytics",
				"enabled": true,
				"state":   "running",
				"health": map[string]interface{}{
					"status":  "healthy",
					"message": "Analytics extension is running",
				},
			},
			{
				"name":    "webhooks",
				"enabled": true,
				"state":   "running",
				"health": map[string]interface{}{
					"status":  "healthy",
					"message": "Webhooks extension is running",
				},
			},
		}

		utils.JSONResponse(w, http.StatusOK, statuses)
	}
}

// Helper function to get extension instances
func getWebhooksExtension() *webhooks.WebhooksExtension {
	return webhooks.NewWebhooksExtension()
}

// Analytics Dashboard Handlers

// Analytics handlers have been moved to the analytics extension

// Webhooks Dashboard Handlers

// HandleWebhooksDashboard returns the webhooks dashboard
func HandleWebhooksDashboard() http.HandlerFunc {
	ext := getWebhooksExtension()
	return ext.DashboardHandler()
}

// HandleWebhooksList returns the list of webhooks
func HandleWebhooksList() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Mock webhook data
		webhooks := []map[string]interface{}{
			{
				"id":     "wh_1",
				"name":   "Order Notifications",
				"url":    "https://api.example.com/webhooks/orders",
				"events": []string{"order.created", "order.updated"},
				"active": true,
			},
			{
				"id":     "wh_2",
				"name":   "User Updates",
				"url":    "https://api.example.com/webhooks/users",
				"events": []string{"user.created", "user.deleted"},
				"active": false,
			},
		}

		utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
			"webhooks": webhooks,
			"total":    len(webhooks),
		})
	}
}

// HandleWebhooksCreate creates a new webhook
func HandleWebhooksCreate() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var webhook map[string]interface{}
		if err := json.NewDecoder(r.Body).Decode(&webhook); err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid request")
			return
		}

		// Mock creation
		webhook["id"] = "wh_new"
		webhook["createdAt"] = "2024-01-15T10:00:00Z"

		utils.JSONResponse(w, http.StatusCreated, webhook)
	}
}

// HandleWebhooksToggle toggles a webhook on/off
func HandleWebhooksToggle() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		id := vars["id"]

		var req struct {
			Active bool `json:"active"`
		}

		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			utils.JSONError(w, http.StatusBadRequest, "Invalid request")
			return
		}

		utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
			"success": true,
			"id":      id,
			"active":  req.Active,
		})
	}
}

// HandleWebhooksDelete deletes a webhook
func HandleWebhooksDelete() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		id := vars["id"]

		// In production, this would delete the webhook from the database
		// For now, just return success
		utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
			"success": true,
			"message": fmt.Sprintf("Webhook %s deleted successfully", id),
		})
	}
}

// Analytics export and clear handlers have been moved to the analytics extension
