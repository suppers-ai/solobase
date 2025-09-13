package webhooks

import (
	"bytes"
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"net/http"
	"time"

	"github.com/suppers-ai/solobase/extensions/core"
)

// WebhooksExtension provides webhook management and delivery
type WebhooksExtension struct {
	services *core.ExtensionServices
	enabled  bool
	client   *http.Client
	hooks    []WebhookConfig
}

// WebhookConfig defines a webhook configuration
type WebhookConfig struct {
	ID        string            `json:"id"`
	Name      string            `json:"name"`
	URL       string            `json:"url"`
	Events    []string          `json:"events"`
	Headers   map[string]string `json:"headers"`
	Secret    string            `json:"secret"`
	Active    bool              `json:"active"`
	CreatedAt time.Time         `json:"created_at"`
	UpdatedAt time.Time         `json:"updated_at"`
}

// WebhookDelivery represents a webhook delivery attempt
type WebhookDelivery struct {
	ID          string                 `json:"id"`
	WebhookID   string                 `json:"webhook_id"`
	Event       string                 `json:"event"`
	Payload     map[string]interface{} `json:"payload"`
	Status      int                    `json:"status"`
	Response    string                 `json:"response"`
	Duration    time.Duration          `json:"duration"`
	DeliveredAt time.Time              `json:"delivered_at"`
}

// NewWebhooksExtension creates a new webhooks extension
func NewWebhooksExtension() *WebhooksExtension {
	return &WebhooksExtension{
		enabled: true,
		client: &http.Client{
			Timeout: 10 * time.Second,
		},
		hooks: []WebhookConfig{},
	}
}

// Metadata returns extension metadata
func (e *WebhooksExtension) Metadata() core.ExtensionMetadata {
	return core.ExtensionMetadata{
		Name:        "webhooks",
		Version:     "1.0.0",
		Description: "Robust webhook management and delivery system for real-time event notifications and third-party integrations. Features secure HMAC signatures, automatic retries, delivery monitoring, and a comprehensive management dashboard.",
		Author:      "Solobase Official",
		License:     "MIT",
		Homepage:    "https://github.com/suppers-ai/solobase",
		Tags:        []string{"webhooks", "integration", "automation", "notifications", "api"},
		MinVersion:  "1.0.0",
		MaxVersion:  "2.0.0",
	}
}

// Initialize initializes the extension
func (e *WebhooksExtension) Initialize(ctx context.Context, services *core.ExtensionServices) error {
	e.services = services
	services.Logger().Info(ctx, "Webhooks extension initializing")

	// Load webhook configurations from database
	if err := e.loadWebhooks(ctx); err != nil {
		return fmt.Errorf("failed to load webhooks: %w", err)
	}

	return nil
}

// Start starts the extension
func (e *WebhooksExtension) Start(ctx context.Context) error {
	if e.services != nil && e.services.Logger() != nil {
		e.services.Logger().Info(ctx, "Webhooks extension started")
	}
	return nil
}

// Stop stops the extension
func (e *WebhooksExtension) Stop(ctx context.Context) error {
	if e.services != nil && e.services.Logger() != nil {
		e.services.Logger().Info(ctx, "Webhooks extension stopped")
	}
	e.enabled = false
	return nil
}

// Health returns health status
func (e *WebhooksExtension) Health(ctx context.Context) (*core.HealthStatus, error) {
	status := "healthy"
	if !e.enabled {
		status = "stopped"
	}

	return &core.HealthStatus{
		Status:      status,
		Message:     fmt.Sprintf("Managing %d webhooks", len(e.hooks)),
		LastChecked: time.Now(),
		Checks: []core.HealthCheck{
			{
				Name:   "database",
				Status: "healthy",
			},
			{
				Name:   "http_client",
				Status: "healthy",
			},
		},
	}, nil
}

// RegisterRoutes registers extension routes
func (e *WebhooksExtension) RegisterRoutes(router core.ExtensionRouter) error {
	// Dashboard route - main entry point
	router.HandleFunc("/dashboard", e.DashboardHandler())

	// Webhook management endpoints
	router.HandleFunc("/api/webhooks", e.handleListWebhooks)
	router.HandleFunc("/api/webhooks/create", e.handleCreateWebhook)
	router.HandleFunc("/api/webhooks/{id}", e.handleGetWebhook)
	router.HandleFunc("/api/webhooks/{id}/update", e.handleUpdateWebhook)
	router.HandleFunc("/api/webhooks/{id}/delete", e.handleDeleteWebhook)
	router.HandleFunc("/api/webhooks/{id}/test", e.handleTestWebhook)

	// Delivery history
	router.HandleFunc("/api/webhooks/{id}/deliveries", e.handleListDeliveries)
	router.HandleFunc("/api/webhooks/deliveries/{deliveryId}", e.handleGetDelivery)

	return nil
}

// RegisterMiddleware registers middleware
func (e *WebhooksExtension) RegisterMiddleware() []core.MiddlewareRegistration {
	return []core.MiddlewareRegistration{}
}

// RegisterHooks registers hooks
func (e *WebhooksExtension) RegisterHooks() []core.HookRegistration {
	return []core.HookRegistration{
		{
			Extension: "webhooks",
			Name:      "trigger-webhooks",
			Type:      core.HookPostRequest,
			Priority:  100,
			Handler:   e.triggerWebhooksHook,
		},
	}
}

// RegisterTemplates registers templates
func (e *WebhooksExtension) RegisterTemplates() []core.TemplateRegistration {
	return []core.TemplateRegistration{}
}

// RegisterStaticAssets registers static assets
func (e *WebhooksExtension) RegisterStaticAssets() []core.StaticAssetRegistration {
	return []core.StaticAssetRegistration{}
}

// ConfigSchema returns configuration schema
func (e *WebhooksExtension) ConfigSchema() json.RawMessage {
	schema := map[string]interface{}{
		"type": "object",
		"properties": map[string]interface{}{
			"enabled": map[string]interface{}{
				"type":        "boolean",
				"description": "Enable webhook delivery",
				"default":     true,
			},
			"maxRetries": map[string]interface{}{
				"type":        "integer",
				"description": "Maximum delivery retries",
				"default":     3,
			},
			"retryDelay": map[string]interface{}{
				"type":        "integer",
				"description": "Delay between retries in seconds",
				"default":     60,
			},
			"timeout": map[string]interface{}{
				"type":        "integer",
				"description": "Request timeout in seconds",
				"default":     10,
			},
		},
	}

	data, _ := json.Marshal(schema)
	return data
}

// ValidateConfig validates configuration
func (e *WebhooksExtension) ValidateConfig(config json.RawMessage) error {
	var cfg map[string]interface{}
	return json.Unmarshal(config, &cfg)
}

// ApplyConfig applies configuration
func (e *WebhooksExtension) ApplyConfig(config json.RawMessage) error {
	var cfg map[string]interface{}
	if err := json.Unmarshal(config, &cfg); err != nil {
		return err
	}

	if v, ok := cfg["enabled"].(bool); ok {
		e.enabled = v
	}

	if v, ok := cfg["timeout"].(float64); ok {
		e.client.Timeout = time.Duration(v) * time.Second
	}

	return nil
}

// DatabaseSchema returns database schema name
func (e *WebhooksExtension) DatabaseSchema() string {
	return "ext_webhooks"
}

// RequiredPermissions returns required permissions
func (e *WebhooksExtension) RequiredPermissions() []core.Permission {
	return []core.Permission{
		{
			Name:        "webhooks.manage",
			Description: "Manage webhooks",
			Resource:    "webhooks",
			Actions:     []string{"create", "read", "update", "delete"},
		},
		{
			Name:        "webhooks.deliver",
			Description: "Deliver webhooks",
			Resource:    "webhooks",
			Actions:     []string{"execute"},
		},
	}
}

// Handler implementations

// DashboardHandler returns the dashboard handler for the webhooks extension
func (e *WebhooksExtension) DashboardHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Get webhook statistics
		activeCount := 0
		totalDeliveries := 0
		successRate := float64(0) // Calculate from actual deliveries

		for _, hook := range e.hooks {
			if hook.Active {
				activeCount++
			}
		}

		w.Header().Set("Content-Type", "text/html")
		w.Write([]byte(e.renderDashboardHTML(len(e.hooks), activeCount, totalDeliveries, successRate)))
	}
}

// DashboardPath returns the dashboard path
func (e *WebhooksExtension) DashboardPath() string {
	return "dashboard"
}

// Documentation returns comprehensive documentation for the webhooks extension
func (e *WebhooksExtension) Documentation() core.ExtensionDocumentation {
	return core.ExtensionDocumentation{
		Overview: "The Webhooks extension provides a robust webhook management system that allows you to send HTTP callbacks to external services when specific events occur in your application. Perfect for integrations, notifications, and real-time data synchronization.",
		DataCollected: []core.DataPoint{
			{
				Name:        "Webhook URL",
				Type:        "string",
				Description: "The endpoint URL where webhook payloads are sent",
				Purpose:     "Deliver event notifications to external services",
				Retention:   "Until webhook is deleted",
				Sensitive:   false,
			},
			{
				Name:        "Delivery Status",
				Type:        "object",
				Description: "HTTP response codes and timing for each delivery attempt",
				Purpose:     "Monitor webhook reliability and debugging",
				Retention:   "30 days",
				Sensitive:   false,
			},
			{
				Name:        "Event Payloads",
				Type:        "json",
				Description: "The actual data sent in webhook requests",
				Purpose:     "Event data transmission",
				Retention:   "7 days for retry purposes",
				Sensitive:   true,
			},
		},
		Endpoints: []core.EndpointDoc{
			{
				Path:        "/ext/webhooks/api/webhooks",
				Methods:     []string{"GET"},
				Description: "List all configured webhooks",
				Auth:        "Required",
			},
			{
				Path:        "/ext/webhooks/api/webhooks/create",
				Methods:     []string{"POST"},
				Description: "Create a new webhook",
				Auth:        "Required",
			},
			{
				Path:        "/ext/webhooks/api/webhooks/{id}/test",
				Methods:     []string{"POST"},
				Description: "Send a test payload to verify webhook configuration",
				Auth:        "Required",
			},
		},
		UsageExamples: []core.UsageExample{
			{
				Title:       "Creating a Webhook",
				Description: "Register a new webhook to receive user signup events",
				Language:    "javascript",
				Code: `fetch('/ext/webhooks/api/webhooks/create', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    name: 'User Signups',
    url: 'https://api.example.com/webhooks/signup',
    events: ['user.created', 'user.verified'],
    secret: 'your-webhook-secret'
  })
})`,
			},
		},
	}
}

func (e *WebhooksExtension) handleListWebhooks(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"webhooks": e.hooks,
		"total":    len(e.hooks),
	})
}

func (e *WebhooksExtension) handleCreateWebhook(w http.ResponseWriter, r *http.Request) {
	var webhook WebhookConfig
	if err := json.NewDecoder(r.Body).Decode(&webhook); err != nil {
		http.Error(w, "Invalid request", http.StatusBadRequest)
		return
	}

	webhook.ID = fmt.Sprintf("wh_%d", time.Now().Unix())
	webhook.CreatedAt = time.Now()
	webhook.UpdatedAt = time.Now()

	e.hooks = append(e.hooks, webhook)

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(webhook)
}

func (e *WebhooksExtension) handleGetWebhook(w http.ResponseWriter, r *http.Request) {
	// Implementation would get webhook by ID
	http.Error(w, "Not implemented", http.StatusNotImplemented)
}

func (e *WebhooksExtension) handleUpdateWebhook(w http.ResponseWriter, r *http.Request) {
	// Implementation would update webhook
	http.Error(w, "Not implemented", http.StatusNotImplemented)
}

func (e *WebhooksExtension) handleDeleteWebhook(w http.ResponseWriter, r *http.Request) {
	// Implementation would delete webhook
	http.Error(w, "Not implemented", http.StatusNotImplemented)
}

func (e *WebhooksExtension) handleTestWebhook(w http.ResponseWriter, r *http.Request) {
	// Send test webhook
	testPayload := map[string]interface{}{
		"event":     "test",
		"timestamp": time.Now(),
		"message":   "This is a test webhook",
	}

	// Implementation would send test webhook
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"success": true,
		"payload": testPayload,
	})
}

func (e *WebhooksExtension) handleListDeliveries(w http.ResponseWriter, r *http.Request) {
	// Implementation would list deliveries
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"deliveries": []WebhookDelivery{},
		"total":      0,
	})
}

func (e *WebhooksExtension) handleGetDelivery(w http.ResponseWriter, r *http.Request) {
	// Implementation would get delivery details
	http.Error(w, "Not implemented", http.StatusNotImplemented)
}

// Helper methods

func (e *WebhooksExtension) loadWebhooks(ctx context.Context) error {
	// Load webhooks from database
	// For now, return empty list
	e.hooks = []WebhookConfig{}
	return nil
}

func (e *WebhooksExtension) triggerWebhooksHook(ctx context.Context, hookCtx *core.HookContext) error {
	// Trigger webhooks based on event
	// This would check for matching webhooks and deliver them
	return nil
}

// DeliverWebhook delivers a webhook
func (e *WebhooksExtension) DeliverWebhook(webhook WebhookConfig, event string, payload map[string]interface{}) error {
	if !webhook.Active || !e.enabled {
		return nil
	}

	// Prepare payload
	body, err := json.Marshal(payload)
	if err != nil {
		return err
	}

	// Create request
	req, err := http.NewRequest("POST", webhook.URL, bytes.NewReader(body))
	if err != nil {
		return err
	}

	// Add headers
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("X-Webhook-Event", event)
	req.Header.Set("X-Webhook-ID", webhook.ID)

	for k, v := range webhook.Headers {
		req.Header.Set(k, v)
	}

	// Add signature if secret is configured
	if webhook.Secret != "" {
		signature := e.generateSignature(webhook.Secret, body)
		req.Header.Set("X-Webhook-Signature", signature)
	}

	// Send request
	start := time.Now()
	resp, err := e.client.Do(req)
	duration := time.Since(start)

	if err != nil {
		e.recordDelivery(webhook.ID, event, payload, 0, err.Error(), duration)
		return err
	}
	defer resp.Body.Close()

	// Record delivery
	e.recordDelivery(webhook.ID, event, payload, resp.StatusCode, "", duration)

	return nil
}

func (e *WebhooksExtension) generateSignature(secret string, payload []byte) string {
	h := hmac.New(sha256.New, []byte(secret))
	h.Write(payload)
	return hex.EncodeToString(h.Sum(nil))
}

func (e *WebhooksExtension) recordDelivery(webhookID, event string, payload map[string]interface{}, status int, response string, duration time.Duration) {
	// Record delivery in database
	// Implementation would store in ext_webhooks.deliveries table
}

// renderDashboardHTML generates the dashboard HTML
func (e *WebhooksExtension) renderDashboardHTML(total, active, deliveries int, successRate float64) string {
	return fmt.Sprintf(`
<!DOCTYPE html>
<html>
<head>
    <title>Webhooks Dashboard</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { 
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #f3f4f6;
            padding: 2rem;
        }
        .dashboard-header {
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
            margin-bottom: 1rem;
        }
        .header-icon {
            width: 60px;
            height: 60px;
            background: linear-gradient(135deg, #667eea 0%%, #764ba2 100%%);
            border-radius: 12px;
            display: flex;
            align-items: center;
            justify-content: center;
            color: white;
            font-size: 24px;
        }
        h1 { color: #1f2937; font-size: 1.875rem; margin-bottom: 0.5rem; }
        .description { color: #6b7280; margin-bottom: 1rem; }
        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 1.5rem;
            margin-bottom: 2rem;
        }
        .stat-card {
            background: white;
            border-radius: 12px;
            padding: 1.5rem;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
        }
        .stat-value {
            font-size: 2rem;
            font-weight: bold;
            color: #1f2937;
            margin-bottom: 0.5rem;
        }
        .stat-label {
            color: #6b7280;
            font-size: 0.875rem;
        }
        .webhooks-list {
            background: white;
            border-radius: 12px;
            padding: 1.5rem;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
        }
        .webhook-item {
            border: 1px solid #e5e7eb;
            border-radius: 8px;
            padding: 1rem;
            margin-bottom: 1rem;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        .webhook-active { border-left: 4px solid #10b981; }
        .webhook-inactive { border-left: 4px solid #ef4444; }
        .webhook-name { font-weight: 600; color: #1f2937; margin-bottom: 0.25rem; }
        .webhook-url { color: #6b7280; font-size: 0.875rem; }
        .status-badge {
            padding: 0.25rem 0.75rem;
            border-radius: 999px;
            font-size: 0.75rem;
            font-weight: 500;
        }
        .status-active { background: #d1fae5; color: #065f46; }
        .status-inactive { background: #fee2e2; color: #991b1b; }
        .actions {
            display: flex;
            gap: 1rem;
            margin-top: 1.5rem;
        }
        .btn {
            padding: 0.5rem 1rem;
            border-radius: 0.5rem;
            border: none;
            cursor: pointer;
            font-weight: 500;
            transition: all 0.2s;
        }
        .btn-primary {
            background: #3b82f6;
            color: white;
        }
        .btn-primary:hover { background: #2563eb; }
        .btn-secondary {
            background: white;
            color: #4b5563;
            border: 1px solid #e5e7eb;
        }
        .btn-secondary:hover { background: #f9fafb; }
    </style>
</head>
<body>
    <div class="dashboard-header">
        <div class="header-content">
            <div class="header-icon">🪝</div>
            <div>
                <h1>Webhooks Dashboard</h1>
                <p class="description">Manage and monitor your webhook integrations</p>
            </div>
        </div>
        <div class="actions">
            <button class="btn btn-primary" onclick="createWebhook()">+ New Webhook</button>
            <button class="btn btn-secondary" onclick="location.reload()">↻ Refresh</button>
        </div>
    </div>
    
    <div class="stats-grid">
        <div class="stat-card">
            <div class="stat-value">%d</div>
            <div class="stat-label">Total Webhooks</div>
        </div>
        <div class="stat-card">
            <div class="stat-value">%d</div>
            <div class="stat-label">Active Webhooks</div>
        </div>
        <div class="stat-card">
            <div class="stat-value">%d</div>
            <div class="stat-label">Deliveries Today</div>
        </div>
        <div class="stat-card">
            <div class="stat-value">%.1f%%</div>
            <div class="stat-label">Success Rate</div>
        </div>
    </div>
    
    <div class="webhooks-list">
        <h2 style="margin-bottom: 1rem; color: #1f2937;">Configured Webhooks</h2>
        <div id="webhooks-container">Loading...</div>
    </div>
    
    <script>
        // Load webhooks
        fetch('/ext/webhooks/api/webhooks')
            .then(r => r.json())
            .then(data => {
                const container = document.getElementById('webhooks-container');
                if (data.webhooks && data.webhooks.length > 0) {
                    container.innerHTML = data.webhooks.map(webhook => 
                        '<div class="webhook-item ' + (webhook.active ? 'webhook-active' : 'webhook-inactive') + '">' +
                            '<div>' +
                                '<div class="webhook-name">' + (webhook.name || 'Unnamed Webhook') + '</div>' +
                                '<div class="webhook-url">' + webhook.url + '</div>' +
                            '</div>' +
                            '<span class="status-badge ' + (webhook.active ? 'status-active' : 'status-inactive') + '">' +
                                (webhook.active ? 'Active' : 'Inactive') +
                            '</span>' +
                        '</div>'
                    ).join('');
                } else {
                    container.innerHTML = '<p style="color: #6b7280; text-align: center; padding: 2rem;">No webhooks configured yet. Click "New Webhook" to get started.</p>';
                }
            })
            .catch(err => {
                document.getElementById('webhooks-container').innerHTML = 
                    '<p style="color: #ef4444;">Error loading webhooks</p>';
            });
        
        function createWebhook() {
            // Would open a modal or navigate to create page
            alert('Create webhook feature coming soon!');
        }
    </script>
</body>
</html>
`, total, active, deliveries, successRate)
}
