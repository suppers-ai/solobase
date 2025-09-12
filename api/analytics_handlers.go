package api

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"time"

	"github.com/suppers-ai/solobase/database"
	"github.com/suppers-ai/solobase/utils"
)

// AnalyticsHandlers contains handlers for the analytics extension
type AnalyticsHandlers struct {
	db *database.DB
}

// NewAnalyticsHandlers creates new analytics handlers
func NewAnalyticsHandlers(db *database.DB) *AnalyticsHandlers {
	return &AnalyticsHandlers{
		db: db,
	}
}

// HandleStats returns analytics statistics
func (h *AnalyticsHandlers) HandleStats() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		stats := map[string]interface{}{
			"totalViews":  0,
			"uniqueUsers": 0,
			"todayViews":  0,
			"activeNow":   0,
		}

		if h.db != nil && h.db.DB != nil {
			// Get total views
			var totalViews int64
			h.db.DB.Table("page_views").Count(&totalViews)
			stats["totalViews"] = totalViews

			// Get unique users (non-empty user_id)
			var uniqueUsers int64
			h.db.DB.Table("page_views").Where("user_id IS NOT NULL AND user_id != ?", "").
				Select("COUNT(DISTINCT user_id)").Row().Scan(&uniqueUsers)
			stats["uniqueUsers"] = uniqueUsers

			// Get today's views
			var todayViews int64
			h.db.DB.Table("page_views").Where("DATE(created_at) = DATE(?)", time.Now()).Count(&todayViews)
			stats["todayViews"] = todayViews

			// Get active users (last 5 minutes)
			var activeNow int64
			fiveMinutesAgo := time.Now().Add(-5 * time.Minute)
			h.db.DB.Table("page_views").Where("created_at > ?", fiveMinutesAgo).
				Select("COUNT(DISTINCT COALESCE(NULLIF(user_id, ''), session_id))").Row().Scan(&activeNow)
			stats["activeNow"] = activeNow
		}

		utils.JSONResponse(w, http.StatusOK, stats)
	}
}

// HandlePageViews returns page view statistics
func (h *AnalyticsHandlers) HandlePageViews() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		pageViews := []map[string]interface{}{}

		if h.db != nil && h.db.DB != nil {
			sevenDaysAgo := time.Now().AddDate(0, 0, -7)

			type PageViewCount struct {
				PageURL string
				Views   int
			}

			var results []PageViewCount
			h.db.DB.Table("page_views").
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

		utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
			"pageViews": pageViews,
		})
	}
}

// HandleTrack handles event tracking
func (h *AnalyticsHandlers) HandleTrack() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
			return
		}

		var data map[string]interface{}
		if err := json.NewDecoder(r.Body).Decode(&data); err != nil {
			http.Error(w, "Invalid request", http.StatusBadRequest)
			return
		}

		ctx := r.Context()

		// Extract user ID from context if available
		userID := ""
		if uid := ctx.Value("user_id"); uid != nil {
			userID = fmt.Sprintf("%v", uid)
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

			// Insert page view
			if h.db != nil && h.db.DB != nil {
				pageView := map[string]interface{}{
					"user_id":    userID,
					"session_id": sessionID,
					"page_url":   pageURL,
					"referrer":   r.Referer(),
					"user_agent": r.UserAgent(),
					"ip_address": r.RemoteAddr,
					"created_at": time.Now(),
				}

				if err := h.db.DB.Table("page_views").Create(&pageView).Error; err != nil {
					fmt.Printf("Failed to track page view: %v\n", err)
				}
			}
		} else {
			// Store as regular event
			if h.db != nil && h.db.DB != nil {
				eventName := ""
				if name, ok := data["event"].(string); ok {
					eventName = name
				}

				eventData, _ := json.Marshal(data)

				event := map[string]interface{}{
					"user_id":    userID,
					"event_name": eventName,
					"event_data": string(eventData),
					"created_at": time.Now(),
				}

				if err := h.db.DB.Table("analytics_events").Create(&event).Error; err != nil {
					fmt.Printf("Failed to track event: %v\n", err)
				}
			}
		}

		w.WriteHeader(http.StatusNoContent)
	}
}

// InitializeSchema creates the analytics tables if they don't exist
func (h *AnalyticsHandlers) InitializeSchema() error {
	if h.db == nil || h.db.DB == nil {
		return fmt.Errorf("database not initialized")
	}

	// Detect database type for proper SQL syntax
	dbType := "sqlite"
	if h.db.Config.Type == "postgres" || h.db.Config.Type == "postgresql" {
		dbType = "postgres"
	}

	// Create page_views table with appropriate syntax
	var createPageViewsSQL string
	if dbType == "postgres" {
		createPageViewsSQL = `
			CREATE TABLE IF NOT EXISTS page_views (
				id SERIAL PRIMARY KEY,
				user_id TEXT,
				session_id TEXT,
				page_url TEXT NOT NULL,
				referrer TEXT,
				user_agent TEXT,
				ip_address TEXT,
				created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
			)`
	} else {
		createPageViewsSQL = `
			CREATE TABLE IF NOT EXISTS page_views (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				user_id TEXT,
				session_id TEXT,
				page_url TEXT NOT NULL,
				referrer TEXT,
				user_agent TEXT,
				ip_address TEXT,
				created_at DATETIME DEFAULT CURRENT_TIMESTAMP
			)`
	}

	err := h.db.DB.Exec(createPageViewsSQL).Error
	if err != nil {
		return fmt.Errorf("failed to create page_views table: %w", err)
	}

	// Create analytics_events table
	var createEventsSQL string
	if dbType == "postgres" {
		createEventsSQL = `
			CREATE TABLE IF NOT EXISTS analytics_events (
				id SERIAL PRIMARY KEY,
				user_id TEXT,
				event_name TEXT NOT NULL,
				event_data TEXT,
				created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
			)`
	} else {
		createEventsSQL = `
			CREATE TABLE IF NOT EXISTS analytics_events (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				user_id TEXT,
				event_name TEXT NOT NULL,
				event_data TEXT,
				created_at DATETIME DEFAULT CURRENT_TIMESTAMP
			)`
	}

	err = h.db.DB.Exec(createEventsSQL).Error
	if err != nil {
		return fmt.Errorf("failed to create analytics_events table: %w", err)
	}

	// Create indexes for better performance
	h.db.DB.Exec(`CREATE INDEX IF NOT EXISTS idx_page_views_created_at ON page_views(created_at)`)
	h.db.DB.Exec(`CREATE INDEX IF NOT EXISTS idx_page_views_user_id ON page_views(user_id)`)
	h.db.DB.Exec(`CREATE INDEX IF NOT EXISTS idx_page_views_page_url ON page_views(page_url)`)
	h.db.DB.Exec(`CREATE INDEX IF NOT EXISTS idx_events_created_at ON analytics_events(created_at)`)
	h.db.DB.Exec(`CREATE INDEX IF NOT EXISTS idx_events_event_name ON analytics_events(event_name)`)

	return nil
}

// TrackPageViewMiddleware is middleware that tracks page views
func (h *AnalyticsHandlers) TrackPageViewMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Track page view asynchronously
		go h.trackPageView(r)

		// Continue with request
		next.ServeHTTP(w, r)
	})
}

func (h *AnalyticsHandlers) trackPageView(r *http.Request) {
	if h.db == nil || h.db.DB == nil {
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

	// Extract user ID
	userID := ""
	if uid := r.Context().Value("user_id"); uid != nil {
		userID = fmt.Sprintf("%v", uid)
	}

	// Get session ID from cookie
	sessionID := ""
	if cookie, err := r.Cookie("session_id"); err == nil {
		sessionID = cookie.Value
	}

	// Get client IP
	clientIP := r.RemoteAddr
	if forwarded := r.Header.Get("X-Forwarded-For"); forwarded != "" {
		clientIP = forwarded
	}

	// Insert page view
	pageView := map[string]interface{}{
		"user_id":    userID,
		"session_id": sessionID,
		"page_url":   r.URL.Path,
		"referrer":   r.Referer(),
		"user_agent": r.UserAgent(),
		"ip_address": clientIP,
		"created_at": time.Now(),
	}

	if err := h.db.DB.Table("page_views").Create(&pageView).Error; err != nil {
		// Log error but don't fail the request
		fmt.Printf("Failed to track page view: %v\n", err)
	}
}

// GetTopPages returns the top pages by views
func (h *AnalyticsHandlers) GetTopPages(ctx context.Context, days int, limit int) ([]map[string]interface{}, error) {
	if h.db == nil || h.db.DB == nil {
		return nil, fmt.Errorf("database not initialized")
	}

	daysAgo := time.Now().AddDate(0, 0, -days)

	type PageViewCount struct {
		PageURL string
		Views   int
	}

	var results []PageViewCount
	h.db.DB.Table("page_views").
		Select("page_url, COUNT(*) as views").
		Where("created_at > ?", daysAgo).
		Group("page_url").
		Order("views DESC").
		Limit(limit).
		Scan(&results)

	var pages []map[string]interface{}
	for _, result := range results {
		pages = append(pages, map[string]interface{}{
			"url":   result.PageURL,
			"views": result.Views,
		})
	}

	return pages, nil
}

// GetDailyStats returns daily statistics for the chart
func (h *AnalyticsHandlers) GetDailyStats(ctx context.Context, days int) ([]map[string]interface{}, error) {
	if h.db == nil || h.db.DB == nil {
		return nil, fmt.Errorf("database not initialized")
	}

	daysAgo := time.Now().AddDate(0, 0, -days)

	type DailyStat struct {
		Date           string
		Views          int
		UniqueVisitors int
	}

	var results []DailyStat
	h.db.DB.Table("page_views").
		Select("DATE(created_at) as date, COUNT(*) as views, COUNT(DISTINCT COALESCE(NULLIF(user_id, ''), session_id)) as unique_visitors").
		Where("created_at > ?", daysAgo).
		Group("DATE(created_at)").
		Order("date ASC").
		Scan(&results)

	var stats []map[string]interface{}
	for _, result := range results {
		stats = append(stats, map[string]interface{}{
			"date":           result.Date,
			"views":          result.Views,
			"uniqueVisitors": result.UniqueVisitors,
		})
	}

	return stats, nil
}

// HandleDailyStats returns daily statistics for charts
func (h *AnalyticsHandlers) HandleDailyStats() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
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

		stats, err := h.GetDailyStats(r.Context(), days)
		if err != nil {
			utils.JSONError(w, http.StatusInternalServerError, err.Error())
			return
		}

		utils.JSONResponse(w, http.StatusOK, map[string]interface{}{
			"dailyStats": stats,
		})
	}
}
