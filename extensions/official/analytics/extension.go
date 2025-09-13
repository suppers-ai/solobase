package analytics

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"time"

	"github.com/google/uuid"
	"github.com/suppers-ai/solobase/extensions/core"
	"gorm.io/datatypes"
	"gorm.io/gorm"
)

// AnalyticsExtension provides page view tracking and analytics
type AnalyticsExtension struct {
	services *core.ExtensionServices
	db       *gorm.DB
	enabled  bool
}

// NewAnalyticsExtension creates a new analytics extension
func NewAnalyticsExtension() *AnalyticsExtension {
	return &AnalyticsExtension{
		enabled: true,
	}
}

// Metadata returns the extension metadata
func (e *AnalyticsExtension) Metadata() core.ExtensionMetadata {
	return core.ExtensionMetadata{
		Name:        "analytics",
		Version:     "1.0.0",
		Description: "Comprehensive analytics and tracking system for monitoring user interactions, page views, and custom events. Includes a real-time dashboard with visualizations, automatic page tracking middleware, and REST API endpoints for retrieving analytics data. Stores data in a dedicated schema with configurable retention periods.",
		Author:      "Solobase Official",
		License:     "MIT",
		Homepage:    "https://github.com/suppers-ai/solobase",
		Tags:        []string{"analytics", "tracking", "dashboard", "metrics", "statistics"},
		MinVersion:  "1.0.0",
		MaxVersion:  "2.0.0",
	}
}

// Initialize initializes the extension
func (e *AnalyticsExtension) Initialize(ctx context.Context, services *core.ExtensionServices) error {
	e.services = services

	// Log initialization
	fmt.Println("Analytics: Initialize called")
	services.Logger().Info(ctx, "Analytics extension initializing")

	// Initialize database tables if database is available
	if e.db != nil {
		if err := e.db.AutoMigrate(&PageView{}, &Event{}); err != nil {
			services.Logger().Error(ctx, fmt.Sprintf("Failed to migrate analytics tables: %v", err))
			return err
		}
		services.Logger().Info(ctx, "Analytics tables migrated successfully")
	}

	return nil
}

// Start starts the extension
func (e *AnalyticsExtension) Start(ctx context.Context) error {
	if e.services != nil && e.services.Logger() != nil {
		e.services.Logger().Info(ctx, "Analytics extension started")
	}
	return nil
}

// Stop stops the extension
func (e *AnalyticsExtension) Stop(ctx context.Context) error {
	if e.services != nil && e.services.Logger() != nil {
		e.services.Logger().Info(ctx, "Analytics extension stopped")
	}
	e.enabled = false
	return nil
}

// Health returns the health status of the extension
func (e *AnalyticsExtension) Health(ctx context.Context) (*core.HealthStatus, error) {
	status := "healthy"
	if !e.enabled {
		status = "stopped"
	}

	return &core.HealthStatus{
		Status:      status,
		Message:     "Analytics extension is running",
		LastChecked: time.Now(),
		Checks: []core.HealthCheck{
			{
				Name:   "database",
				Status: "healthy",
			},
		},
	}, nil
}

// RegisterRoutes registers the extension routes
func (e *AnalyticsExtension) RegisterRoutes(router core.ExtensionRouter) error {
	// Dashboard route - disabled as we use the Svelte dashboard
	// router.HandleFunc("/dashboard", e.DashboardHandler())

	// API endpoints - will be under /api/ext/analytics/
	fmt.Println("Analytics: Registering /pageviews")
	router.HandleFunc("/pageviews", e.handlePageViews)
	fmt.Println("Analytics: Registering /track")
	router.HandleFunc("/track", e.handleTrack)
	fmt.Println("Analytics: Registering /stats")
	router.HandleFunc("/stats", e.handleStats)
	fmt.Println("Analytics: Registering /daily")
	router.HandleFunc("/daily", e.handleDailyStats)
	fmt.Println("Analytics: Routes registered")

	return nil
}

// RegisterMiddleware registers middleware for automatic tracking
func (e *AnalyticsExtension) RegisterMiddleware() []core.MiddlewareRegistration {
	return []core.MiddlewareRegistration{
		{
			Extension: "analytics",
			Name:      "page-tracker",
			Priority:  100,
			Handler:   e.trackingMiddleware,
		},
	}
}

// RegisterHooks registers hooks
func (e *AnalyticsExtension) RegisterHooks() []core.HookRegistration {
	return []core.HookRegistration{
		{
			Extension: "analytics",
			Name:      "post-auth-track",
			Type:      core.HookPostAuth,
			Priority:  50,
			Handler:   e.postAuthHook,
		},
	}
}

// RegisterTemplates registers templates
func (e *AnalyticsExtension) RegisterTemplates() []core.TemplateRegistration {
	return []core.TemplateRegistration{}
}

// RegisterStaticAssets registers static assets
func (e *AnalyticsExtension) RegisterStaticAssets() []core.StaticAssetRegistration {
	return []core.StaticAssetRegistration{}
}

// ConfigSchema returns the configuration schema
func (e *AnalyticsExtension) ConfigSchema() json.RawMessage {
	schema := map[string]interface{}{
		"type": "object",
		"properties": map[string]interface{}{
			"enabled": map[string]interface{}{
				"type":        "boolean",
				"description": "Enable analytics tracking",
				"default":     true,
			},
			"excludePaths": map[string]interface{}{
				"type":        "array",
				"description": "Paths to exclude from tracking",
				"items": map[string]interface{}{
					"type": "string",
				},
				"default": []string{"/api/", "/ext/"},
			},
			"retentionDays": map[string]interface{}{
				"type":        "integer",
				"description": "Days to retain analytics data",
				"default":     90,
			},
		},
	}

	data, _ := json.Marshal(schema)
	return data
}

// ValidateConfig validates the configuration
func (e *AnalyticsExtension) ValidateConfig(config json.RawMessage) error {
	var cfg map[string]interface{}
	if err := json.Unmarshal(config, &cfg); err != nil {
		return fmt.Errorf("invalid config format: %w", err)
	}

	// Validate enabled field
	if v, ok := cfg["enabled"]; ok {
		if _, ok := v.(bool); !ok {
			return fmt.Errorf("enabled must be a boolean")
		}
	}

	// Validate retentionDays
	if v, ok := cfg["retentionDays"]; ok {
		if days, ok := v.(float64); !ok || days < 1 || days > 365 {
			return fmt.Errorf("retentionDays must be between 1 and 365")
		}
	}

	return nil
}

// ApplyConfig applies the configuration
func (e *AnalyticsExtension) ApplyConfig(config json.RawMessage) error {
	var cfg map[string]interface{}
	if err := json.Unmarshal(config, &cfg); err != nil {
		return err
	}

	if v, ok := cfg["enabled"].(bool); ok {
		e.enabled = v
	}

	return nil
}

// DatabaseSchema returns the database schema name
func (e *AnalyticsExtension) DatabaseSchema() string {
	return "ext_analytics"
}

// SetDatabase sets the database instance for the extension
func (e *AnalyticsExtension) SetDatabase(db *gorm.DB) {
	e.db = db
}

// RequiredPermissions returns required permissions
func (e *AnalyticsExtension) RequiredPermissions() []core.Permission {
	return []core.Permission{
		{
			Name:        "analytics.view",
			Description: "View analytics data",
			Resource:    "analytics",
			Actions:     []string{"read"},
		},
		{
			Name:        "analytics.admin",
			Description: "Administer analytics settings",
			Resource:    "analytics",
			Actions:     []string{"read", "write", "delete"},
		},
	}
}

// Handler methods

// DashboardHandler returns the dashboard handler for analytics
func (e *AnalyticsExtension) DashboardHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/html")
		fmt.Fprint(w, e.renderDashboardHTML())
	}
}

// DashboardPath returns the dashboard path
func (e *AnalyticsExtension) DashboardPath() string {
	return "dashboard"
}

// Documentation returns comprehensive documentation
func (e *AnalyticsExtension) Documentation() core.ExtensionDocumentation {
	return core.ExtensionDocumentation{
		Overview: "The Analytics extension provides comprehensive tracking and analytics for your application. It automatically tracks page views, user interactions, and custom events, presenting them in an intuitive dashboard with real-time updates and historical trends.",
		DataCollected: []core.DataPoint{
			{
				Name:        "Page Views",
				Type:        "pageview",
				Description: "URL, referrer, and timestamp of each page visit",
				Purpose:     "Understand user navigation patterns and popular content",
				Retention:   "90 days",
				Sensitive:   false,
			},
			{
				Name:        "User Sessions",
				Type:        "session",
				Description: "Anonymous session identifiers and duration",
				Purpose:     "Track user engagement and session metrics",
				Retention:   "30 days",
				Sensitive:   false,
			},
			{
				Name:        "Custom Events",
				Type:        "event",
				Description: "User-defined events with custom properties",
				Purpose:     "Track specific user actions and behaviors",
				Retention:   "60 days",
				Sensitive:   false,
			},
		},
		Endpoints: []core.EndpointDoc{
			{
				Path:        "/ext/analytics/api/track",
				Methods:     []string{"POST"},
				Description: "Track custom events",
				Auth:        "Optional",
			},
			{
				Path:        "/ext/analytics/api/pageviews",
				Methods:     []string{"GET"},
				Description: "Retrieve page view statistics",
				Auth:        "Required",
			},
			{
				Path:        "/ext/analytics/api/stats",
				Methods:     []string{"GET"},
				Description: "Get aggregated analytics statistics",
				Auth:        "Required",
			},
		},
	}
}

func (e *AnalyticsExtension) handlePageViews(w http.ResponseWriter, r *http.Request) {
	// Start with empty data
	pageViews := []map[string]interface{}{}

	// If we have database access, use real data
	if e.db != nil {
		sevenDaysAgo := time.Now().AddDate(0, 0, -7)

		var results []PageViewStats
		e.db.Model(&PageView{}).
			Select("page_url, COUNT(*) as views").
			Where("created_at > ?", sevenDaysAgo).
			Group("page_url").
			Order("views DESC").
			Limit(10).
			Scan(&results)

		for _, result := range results {
			pageViews = append(pageViews, map[string]interface{}{
				"url":   result.PageURL,
				"views": result.Views,
			})
		}
	}

	// If no data, return some default entries
	if len(pageViews) == 0 {
		pageViews = []map[string]interface{}{
			{"url": "/", "views": 0},
			{"url": "/dashboard", "views": 0},
		}
	}

	// Return JSON response
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"pageViews": pageViews,
	})
}

func (e *AnalyticsExtension) handleTrack(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Parse tracking data
	var data map[string]interface{}
	if err := json.NewDecoder(r.Body).Decode(&data); err != nil {
		http.Error(w, "Invalid request", http.StatusBadRequest)
		return
	}

	ctx := r.Context()

	// Extract user ID from context if available
	var userID *string
	if uid := ctx.Value("user_id"); uid != nil {
		uidStr := fmt.Sprintf("%v", uid)
		userID = &uidStr
	}

	// Check if this is a page view event
	if eventName, ok := data["event"].(string); ok && eventName == "page_view" {
		// Track as page view
		pageURL := "/"
		if url, ok := data["url"].(string); ok {
			pageURL = url
		}

		// Get session ID from cookie or generate one
		sessionID := ""
		if cookie, err := r.Cookie("session_id"); err == nil {
			sessionID = cookie.Value
		} else {
			sessionID = fmt.Sprintf("session_%d", time.Now().Unix())
			http.SetCookie(w, &http.Cookie{
				Name:     "session_id",
				Value:    sessionID,
				Path:     "/",
				HttpOnly: true,
				MaxAge:   86400, // 1 day
			})
		}

		referrer := r.Referer()
		userAgent := r.UserAgent()
		ipAddress := r.RemoteAddr

		// Insert page view
		if e.db != nil {
			pageView := &PageView{
				ID:        uuid.New().String(),
				UserID:    userID,
				SessionID: sessionID,
				PageURL:   pageURL,
				Referrer:  &referrer,
				UserAgent: &userAgent,
				IPAddress: &ipAddress,
				CreatedAt: time.Now(),
			}

			if err := e.db.Create(pageView).Error; err != nil {
				fmt.Printf("Failed to track page view: %v\n", err)
			}
		}
	} else {
		// Store as regular event
		if e.db != nil {
			eventName := ""
			if name, ok := data["event"].(string); ok {
				eventName = name
			}

			eventData, _ := json.Marshal(data)

			event := &Event{
				ID:        uuid.New().String(),
				UserID:    userID,
				EventName: eventName,
				EventData: datatypes.JSON(eventData),
				CreatedAt: time.Now(),
			}

			if err := e.db.Create(event).Error; err != nil {
				fmt.Printf("Failed to track event: %v\n", err)
			}
		}
	}

	w.WriteHeader(http.StatusNoContent)
}

func (e *AnalyticsExtension) handleStats(w http.ResponseWriter, r *http.Request) {
	// Start with zero stats
	stats := &AnalyticsStats{
		TotalViews:  0,
		UniqueUsers: 0,
		TodayViews:  0,
		ActiveNow:   0,
	}

	if e.db != nil {
		// Get total views
		e.db.Model(&PageView{}).Count(&stats.TotalViews)

		// Get unique users (non-null user_id)
		e.db.Model(&PageView{}).
			Where("user_id IS NOT NULL").
			Distinct("user_id").
			Count(&stats.UniqueUsers)

		// Get today's views
		today := time.Now().Truncate(24 * time.Hour)
		e.db.Model(&PageView{}).
			Where("created_at >= ?", today).
			Count(&stats.TodayViews)

		// Get active users (last 5 minutes)
		fiveMinutesAgo := time.Now().Add(-5 * time.Minute)
		e.db.Model(&PageView{}).
			Where("created_at > ?", fiveMinutesAgo).
			Distinct("COALESCE(user_id, session_id)").
			Count(&stats.ActiveNow)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(stats)
}

func (e *AnalyticsExtension) handleDailyStats(w http.ResponseWriter, r *http.Request) {
	// Get days parameter from query
	days := 7
	if d := r.URL.Query().Get("days"); d != "" {
		if parsed, err := fmt.Sscanf(d, "%d", &days); err == nil && parsed == 1 {
			// Limit to reasonable range
			if days > 90 {
				days = 90
			} else if days < 1 {
				days = 7
			}
		}
	}

	dailyStats := []DailyStats{}

	if e.db != nil {
		startDate := time.Now().AddDate(0, 0, -days)

		// Get daily stats using GORM
		e.db.Model(&PageView{}).
			Select("DATE(created_at) as date, COUNT(*) as page_views, COUNT(DISTINCT user_id) as unique_users").
			Where("created_at >= ?", startDate).
			Group("DATE(created_at)").
			Order("date ASC").
			Scan(&dailyStats)

		// Also count events for each day
		for i := range dailyStats {
			var eventCount int64
			e.db.Model(&Event{}).
				Where("DATE(created_at) = ?", dailyStats[i].Date).
				Count(&eventCount)
			dailyStats[i].Events = eventCount
		}
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"dailyStats": dailyStats,
	})
}

// Middleware

func (e *AnalyticsExtension) trackingMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !e.enabled {
			next.ServeHTTP(w, r)
			return
		}

		// Track page view asynchronously
		go e.trackPageView(r)

		// Continue with request
		next.ServeHTTP(w, r)
	})
}

func (e *AnalyticsExtension) trackPageView(r *http.Request) {
	if e.db == nil {
		return
	}

	// Don't track API calls or static assets
	path := r.URL.Path
	if len(path) >= 4 && path[:4] == "/api" {
		return
	}
	if len(path) >= 7 && path[:7] == "/static" {
		return
	}
	if len(path) >= 4 && path[:4] == "/ext" {
		return
	}

	// Extract user ID
	var userID *string
	if uid := r.Context().Value("user_id"); uid != nil {
		uidStr := fmt.Sprintf("%v", uid)
		userID = &uidStr
	}

	// Get session ID from cookie
	sessionID := ""
	if cookie, err := r.Cookie("session_id"); err == nil {
		sessionID = cookie.Value
	} else {
		sessionID = fmt.Sprintf("session_%d", time.Now().Unix())
	}

	// Get client IP
	clientIP := r.RemoteAddr
	if forwarded := r.Header.Get("X-Forwarded-For"); forwarded != "" {
		clientIP = forwarded
	}

	referrer := r.Referer()
	userAgent := r.UserAgent()

	// Insert page view
	pageView := &PageView{
		ID:        uuid.New().String(),
		UserID:    userID,
		SessionID: sessionID,
		PageURL:   r.URL.Path,
		Referrer:  &referrer,
		UserAgent: &userAgent,
		IPAddress: &clientIP,
		CreatedAt: time.Now(),
	}

	if err := e.db.Create(pageView).Error; err != nil {
		// Log error but don't fail the request
		fmt.Printf("Failed to track page view: %v\n", err)
	}
}

// Hooks

func (e *AnalyticsExtension) postAuthHook(ctx context.Context, hookCtx *core.HookContext) error {
	// Track login event
	if e.db != nil {
		if userID := hookCtx.Request.Context().Value("user_id"); userID != nil {
			uidStr := fmt.Sprintf("%v", userID)
			event := &Event{
				ID:        uuid.New().String(),
				UserID:    &uidStr,
				EventName: "login",
				EventData: datatypes.JSON(`{}`),
				CreatedAt: time.Now(),
			}
			e.db.Create(event)
		}
	}

	return nil
}

// renderDashboardHTML generates the analytics dashboard HTML
func (e *AnalyticsExtension) renderDashboardHTML() string {
	return `<!DOCTYPE html>
<html>
<head>
    <title>Analytics Dashboard</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { 
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
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
            background: linear-gradient(135deg, #3b82f6 0%, #1e40af 100%);
            border-radius: 12px;
            display: flex;
            align-items: center;
            justify-content: center;
            color: white;
            font-size: 24px;
        }
        h1 { color: #1f2937; font-size: 1.875rem; margin-bottom: 0.5rem; }
        .description { color: #6b7280; margin-bottom: 1rem; }
        .info-badge {
            display: inline-block;
            padding: 0.25rem 0.75rem;
            background: #dbeafe;
            color: #1e40af;
            border-radius: 999px;
            font-size: 0.75rem;
            font-weight: 500;
            margin-left: 1rem;
        }
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
            transition: transform 0.2s;
        }
        .stat-card:hover {
            transform: translateY(-2px);
            box-shadow: 0 4px 6px rgba(0,0,0,0.1);
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
        .chart-container {
            background: white;
            border-radius: 12px;
            padding: 1.5rem;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
            margin-bottom: 2rem;
        }
        .chart-title {
            font-size: 1.25rem;
            font-weight: 600;
            color: #1f2937;
            margin-bottom: 1rem;
        }
        #pageViewsChart {
            height: 300px;
            background: #f9fafb;
            border-radius: 8px;
            display: flex;
            align-items: center;
            justify-content: center;
            color: #6b7280;
            position: relative;
        }
        .top-pages {
            background: white;
            border-radius: 12px;
            padding: 1.5rem;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
        }
        .page-item {
            display: flex;
            justify-content: space-between;
            padding: 0.75rem;
            border-radius: 0.5rem;
            transition: background 0.2s;
        }
        .page-item:hover {
            background: #f9fafb;
        }
        .page-url { 
            color: #1f2937; 
            font-weight: 500;
            text-decoration: none;
        }
        .page-views { 
            color: #6b7280; 
            font-size: 0.875rem;
            background: #f3f4f6;
            padding: 0.25rem 0.5rem;
            border-radius: 0.25rem;
        }
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
            font-size: 0.875rem;
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
        .loading {
            text-align: center;
            padding: 2rem;
            color: #6b7280;
        }
        .error {
            text-align: center;
            padding: 2rem;
            color: #ef4444;
            background: #fee2e2;
            border-radius: 0.5rem;
        }
        .empty-state {
            text-align: center;
            padding: 3rem;
            color: #6b7280;
        }
        .empty-state svg {
            width: 64px;
            height: 64px;
            margin: 0 auto 1rem;
            opacity: 0.3;
        }
    </style>
</head>
<body>
    <div class="dashboard-header">
        <div class="header-content">
            <div class="header-icon">📊</div>
            <div>
                <h1>Analytics Dashboard <span class="info-badge">Official Extension</span></h1>
                <p class="description">Real-time insights into page views, user behavior, and application metrics</p>
            </div>
        </div>
        <div class="actions">
            <button class="btn btn-primary" onclick="trackCustomEvent()">📍 Track Event</button>
            <button class="btn btn-secondary" onclick="exportData()">📥 Export</button>
            <button class="btn btn-secondary" onclick="location.reload()">↻ Refresh</button>
        </div>
    </div>
    
    <div class="stats-grid">
        <div class="stat-card">
            <div class="stat-value" id="totalViews">-</div>
            <div class="stat-label">Total Page Views</div>
        </div>
        <div class="stat-card">
            <div class="stat-value" id="uniqueUsers">-</div>
            <div class="stat-label">Unique Users</div>
        </div>
        <div class="stat-card">
            <div class="stat-value" id="todayViews">-</div>
            <div class="stat-label">Views Today</div>
        </div>
        <div class="stat-card">
            <div class="stat-value" id="activeNow">-</div>
            <div class="stat-label">Active Now</div>
        </div>
    </div>
    
    <div class="chart-container">
        <h2 class="chart-title">Page Views Trend (Last 7 Days)</h2>
        <div id="pageViewsChart">
            <div class="loading">Loading chart data...</div>
        </div>
    </div>
    
    <div class="top-pages">
        <h2 class="chart-title">Top Pages This Week</h2>
        <div id="topPagesList">
            <div class="loading">Loading page data...</div>
        </div>
    </div>
    
    <script>
        // Load analytics stats
        async function loadStats() {
            try {
                const response = await fetch("/ext/analytics/api/stats");
                if (!response.ok) throw new Error("Failed to load stats");
                
                const data = await response.json();
                document.getElementById("totalViews").textContent = formatNumber(data.totalViews || 0);
                document.getElementById("uniqueUsers").textContent = formatNumber(data.uniqueUsers || 0);
                document.getElementById("todayViews").textContent = formatNumber(data.todayViews || 0);
                document.getElementById("activeNow").textContent = data.activeNow || 0;
                
                // Update chart placeholder
                document.getElementById("pageViewsChart").innerHTML = 
                    '<div class="empty-state"><p>Chart visualization coming soon</p></div>';
            } catch (err) {
                console.error("Error loading stats:", err);
                document.querySelectorAll(".stat-value").forEach(el => {
                    el.textContent = "0";
                });
            }
        }
        
        // Load top pages
        async function loadTopPages() {
            try {
                const response = await fetch("/ext/analytics/api/pageviews");
                if (!response.ok) throw new Error("Failed to load page views");
                
                const data = await response.json();
                const container = document.getElementById("topPagesList");
                
                if (data.pageViews && data.pageViews.length > 0) {
                    container.innerHTML = data.pageViews.map((page, index) => 
                        '<div class="page-item">' +
                            '<div style="display: flex; align-items: center; gap: 0.5rem;">' +
                                '<span style="color: #9ca3af; font-size: 0.875rem;">' + (index + 1) + '.</span>' +
                                '<a href="' + page.url + '" class="page-url">' + page.url + '</a>' +
                            '</div>' +
                            '<div class="page-views">' + formatNumber(page.views) + ' views</div>' +
                        '</div>'
                    ).join("");
                } else {
                    container.innerHTML = 
                        '<div class="empty-state">' +
                            '<svg fill="none" stroke="currentColor" viewBox="0 0 24 24">' +
                                '<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" ' +
                                    'd="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />' +
                            '</svg>' +
                            '<p>No page view data available yet</p>' +
                            '<p style="font-size: 0.875rem; margin-top: 0.5rem;">Data will appear once users start visiting your site</p>' +
                        '</div>';
                }
            } catch (err) {
                console.error("Error loading page views:", err);
                document.getElementById("topPagesList").innerHTML = 
                    '<div class="error">Failed to load page views. Please try again.</div>';
            }
        }
        
        // Format large numbers
        function formatNumber(num) {
            if (num >= 1000000) return (num / 1000000).toFixed(1) + 'M';
            if (num >= 1000) return (num / 1000).toFixed(1) + 'K';
            return num.toString();
        }
        
        // Track custom event
        function trackCustomEvent() {
            const eventName = prompt("Enter event name:");
            if (!eventName) return;
            
            fetch("/ext/analytics/api/track", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({
                    event: eventName,
                    timestamp: new Date().toISOString()
                })
            }).then(() => {
                alert("Event tracked successfully!");
                setTimeout(loadStats, 1000);
            }).catch(err => {
                alert("Failed to track event");
            });
        }
        
        // Export data
        function exportData() {
            if (confirm("Export analytics data as CSV?")) {
                alert("Export feature will be available soon!");
            }
        }
        
        // Load data on page load
        loadStats();
        loadTopPages();
        
        // Auto-refresh every 30 seconds
        setInterval(() => {
            loadStats();
            loadTopPages();
        }, 30000);
    </script>
</body>
</html>`
}
