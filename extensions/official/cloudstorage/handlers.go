package cloudstorage

import (
	auth "github.com/suppers-ai/auth"
)

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strconv"
	"strings"
	"time"

	"github.com/google/uuid"
	pkgstorage "github.com/suppers-ai/storage"
)

// Response structs
type ShareResponse struct {
	ID              string     `json:"id"`
	ShareURL        string     `json:"share_url,omitempty"`
	ShareToken      string     `json:"share_token,omitempty"`
	ExpiresAt       *time.Time `json:"expires_at,omitempty"`
	PermissionLevel string     `json:"permission_level"`
}

type QuotaResponse struct {
	StorageUsed         int64      `json:"storage_used"`
	StorageLimit        int64      `json:"storage_limit"`
	StoragePercentage   float64    `json:"storage_percentage"`
	BandwidthUsed       int64      `json:"bandwidth_used"`
	BandwidthLimit      int64      `json:"bandwidth_limit"`
	BandwidthPercentage float64    `json:"bandwidth_percentage"`
	ResetDate           *time.Time `json:"reset_date,omitempty"`
}

// handleShares manages file sharing operations
func (e *CloudStorageExtension) handleShares(w http.ResponseWriter, r *http.Request) {
	if e.shareService == nil {
		http.Error(w, "Sharing is not enabled", http.StatusNotImplemented)
		return
	}

	ctx := r.Context()
	// SECURITY: Get user info from JWT token context, not headers
	user, ok := ctx.Value("user").(*auth.User)
	if !ok {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	userID := user.ID.String()

	switch r.Method {
	case http.MethodPost:
		// Create a new share
		var req struct {
			ObjectID          string     `json:"object_id"`
			SharedWithUserID  string     `json:"shared_with_user_id,omitempty"`
			SharedWithEmail   string     `json:"shared_with_email,omitempty"`
			PermissionLevel   string     `json:"permission_level"`
			InheritToChildren bool       `json:"inherit_to_children"`
			GenerateToken     bool       `json:"generate_token"`
			IsPublic          bool       `json:"is_public"`
			ExpiresAt         *time.Time `json:"expires_at,omitempty"`
		}

		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			http.Error(w, "Invalid request body", http.StatusBadRequest)
			return
		}

		opts := ShareOptions{
			SharedWithUserID:  req.SharedWithUserID,
			SharedWithEmail:   req.SharedWithEmail,
			PermissionLevel:   PermissionLevel(req.PermissionLevel),
			InheritToChildren: req.InheritToChildren,
			GenerateToken:     req.GenerateToken,
			IsPublic:          req.IsPublic,
			ExpiresAt:         req.ExpiresAt,
		}

		share, err := e.shareService.CreateShare(ctx, req.ObjectID, userID, opts)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		// Log the share action
		if e.accessLogService != nil {
			e.accessLogService.LogAccess(ctx, req.ObjectID, ActionShare, LogOptions{
				UserID:    userID,
				IPAddress: parseIPAddress(r.RemoteAddr),
				UserAgent: r.UserAgent(),
			})
		}

		response := ShareResponse{
			ID:              share.ID,
			ExpiresAt:       share.ExpiresAt,
			PermissionLevel: string(share.PermissionLevel),
		}
		if share.ShareToken != nil {
			response.ShareToken = *share.ShareToken
			response.ShareURL = fmt.Sprintf("/share/%s", *share.ShareToken)
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		json.NewEncoder(w).Encode(response)

	case http.MethodGet:
		// List user's shares
		shares, err := e.shareService.GetUserShares(ctx, userID)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(shares)

	default:
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
	}
}

// handleShareAccess handles accessing a shared file via token
func (e *CloudStorageExtension) handleShareAccess(w http.ResponseWriter, r *http.Request) {
	if e.shareService == nil {
		http.Error(w, "Sharing is not enabled", http.StatusNotImplemented)
		return
	}

	ctx := r.Context()

	// Extract share token from URL
	parts := strings.Split(r.URL.Path, "/")
	if len(parts) < 3 {
		http.Error(w, "Invalid URL", http.StatusBadRequest)
		return
	}
	token := parts[2]

	// Get the share
	share, err := e.shareService.GetShareByToken(ctx, token)
	if err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}

	// Get the object
	var obj pkgstorage.StorageObject
	if err := e.db.Where("id = ?", share.ObjectID).First(&obj).Error; err != nil {
		http.Error(w, "Object not found", http.StatusNotFound)
		return
	}

	// Log access
	if e.accessLogService != nil {
		e.accessLogService.LogAccess(ctx, share.ObjectID, ActionView, LogOptions{
			ShareID:   share.ID,
			IPAddress: parseIPAddress(r.RemoteAddr),
			UserAgent: r.UserAgent(),
		})
	}

	// Check permission level
	switch share.PermissionLevel {
	case PermissionView:
		// Allow view/download only
		if r.Method != http.MethodGet {
			http.Error(w, "Permission denied", http.StatusForbidden)
			return
		}
	case PermissionEdit:
		// Allow view/download/upload
		if r.Method != http.MethodGet && r.Method != http.MethodPost && r.Method != http.MethodPut {
			http.Error(w, "Permission denied", http.StatusForbidden)
			return
		}
	case PermissionAdmin:
		// Allow all operations
	}

	// Serve the file or handle upload based on method
	switch r.Method {
	case http.MethodGet:
		// Download the file
		content, contentType, err := e.manager.GetFile(ctx, obj.ID)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		// Update bandwidth usage for the file owner
		if e.quotaService != nil && e.config.EnableQuotas && obj.UserID != "" {
			if err := e.quotaService.UpdateBandwidthUsage(ctx, obj.UserID, obj.Size); err != nil {
				// Log error but don't fail the download
				fmt.Printf("Failed to update bandwidth usage: %v\n", err)
			}
		}

		// Set headers
		w.Header().Set("Content-Type", contentType)
		w.Header().Set("Content-Length", strconv.FormatInt(obj.Size, 10))
		w.Header().Set("Content-Disposition", fmt.Sprintf("attachment; filename=\"%s\"", obj.ObjectName))

		// Write content
		w.Write(content)

	default:
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
	}
}

// handleQuota manages storage quota operations
func (e *CloudStorageExtension) handleQuota(w http.ResponseWriter, r *http.Request) {
	if e.quotaService == nil {
		http.Error(w, "Quotas are not enabled", http.StatusNotImplemented)
		return
	}

	ctx := r.Context()
	// SECURITY: Get user info from JWT token context, not headers
	user, ok := ctx.Value("user").(*auth.User)
	if !ok {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	userID := user.ID.String()
	// Check admin role via IAM service
	isAdmin := false
	if e.services != nil && e.services.IAM() != nil {
		isAdmin, _ = e.services.IAM().UserHasRole(userID, "admin")
	}

	switch r.Method {
	case http.MethodGet:
		// Get quota stats - either for a specific user or all users (admin only)
		queryUserID := r.URL.Query().Get("user_id")

		if queryUserID != "" && isAdmin {
			// Admin getting specific user's quota
			stats, err := e.quotaService.GetQuotaStats(ctx, queryUserID)
			if err != nil {
				http.Error(w, err.Error(), http.StatusInternalServerError)
				return
			}
			w.Header().Set("Content-Type", "application/json")
			json.NewEncoder(w).Encode(stats)
		} else if isAdmin && queryUserID == "" {
			// Admin getting all quotas
			var quotas []StorageQuota
			if err := e.db.Find(&quotas).Error; err != nil {
				http.Error(w, err.Error(), http.StatusInternalServerError)
				return
			}
			w.Header().Set("Content-Type", "application/json")
			json.NewEncoder(w).Encode(quotas)
		} else {
			// Regular user getting their own quota
			stats, err := e.quotaService.GetQuotaStats(ctx, userID)
			if err != nil {
				http.Error(w, err.Error(), http.StatusInternalServerError)
				return
			}
			w.Header().Set("Content-Type", "application/json")
			json.NewEncoder(w).Encode(stats)
		}

	case http.MethodPut:
		// Update quota (admin only)
		if !isAdmin {
			http.Error(w, "Admin access required", http.StatusForbidden)
			return
		}

		var req struct {
			UserID            string `json:"user_id"`
			MaxStorageBytes   int64  `json:"max_storage_bytes"`
			MaxBandwidthBytes int64  `json:"max_bandwidth_bytes"`
		}

		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			http.Error(w, "Invalid request body", http.StatusBadRequest)
			return
		}

		quota, err := e.quotaService.GetOrCreateQuota(ctx, req.UserID)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		quota.MaxStorageBytes = req.MaxStorageBytes
		quota.MaxBandwidthBytes = req.MaxBandwidthBytes
		quota.UpdatedAt = time.Now()

		if err := e.db.Save(quota).Error; err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(quota)

	default:
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
	}
}

// handleAccessLogs retrieves access logs
func (e *CloudStorageExtension) handleAccessLogs(w http.ResponseWriter, r *http.Request) {
	if e.accessLogService == nil {
		http.Error(w, "Access logging is not enabled", http.StatusNotImplemented)
		return
	}

	ctx := r.Context()
	// SECURITY: Get user info from JWT token context, not headers
	user, ok := ctx.Value("user").(*auth.User)
	if !ok {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	userID := user.ID.String()

	// Parse query parameters
	filters := AccessLogFilters{
		UserID:   userID,
		ObjectID: r.URL.Query().Get("object_id"),
		Action:   r.URL.Query().Get("action"),
		Limit:    100,
	}

	if limitStr := r.URL.Query().Get("limit"); limitStr != "" {
		if limit, err := strconv.Atoi(limitStr); err == nil {
			filters.Limit = limit
		}
	}

	// Parse date filters
	if startStr := r.URL.Query().Get("start_date"); startStr != "" {
		if start, err := time.Parse(time.RFC3339, startStr); err == nil {
			filters.StartDate = &start
		}
	}
	if endStr := r.URL.Query().Get("end_date"); endStr != "" {
		if end, err := time.Parse(time.RFC3339, endStr); err == nil {
			filters.EndDate = &end
		}
	}

	logs, err := e.accessLogService.GetAccessLogs(ctx, filters)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(logs)
}

// handleAccessStats retrieves access statistics
func (e *CloudStorageExtension) handleAccessStats(w http.ResponseWriter, r *http.Request) {
	if e.accessLogService == nil {
		http.Error(w, "Access logging is not enabled", http.StatusNotImplemented)
		return
	}

	ctx := r.Context()
	// SECURITY: Get user info from JWT token context, not headers
	user, ok := ctx.Value("user").(*auth.User)
	if !ok {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	userID := user.ID.String()

	filters := StatsFilters{
		UserID:   userID,
		ObjectID: r.URL.Query().Get("object_id"),
	}

	// Parse date filters
	if startStr := r.URL.Query().Get("start_date"); startStr != "" {
		if start, err := time.Parse(time.RFC3339, startStr); err == nil {
			filters.StartDate = &start
		}
	}
	if endStr := r.URL.Query().Get("end_date"); endStr != "" {
		if end, err := time.Parse(time.RFC3339, endStr); err == nil {
			filters.EndDate = &end
		}
	}

	stats, err := e.accessLogService.GetAccessStats(ctx, filters)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(stats)
}

// handleUpload handles file uploads with quota checking
func (e *CloudStorageExtension) handleUpload(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	// SECURITY: Get user info from JWT token context, not headers
	user, ok := ctx.Value("user").(*auth.User)
	if !ok {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	userID := user.ID.String()

	// Parse multipart form
	err := r.ParseMultipartForm(32 << 20) // 32MB max memory
	if err != nil {
		http.Error(w, "Failed to parse form", http.StatusBadRequest)
		return
	}

	bucketName := r.FormValue("bucket")
	if bucketName == "" {
		bucketName = "uploads" // Default bucket
	}

	file, header, err := r.FormFile("file")
	if err != nil {
		http.Error(w, "Failed to get file", http.StatusBadRequest)
		return
	}
	defer file.Close()

	// Check quota before upload
	if e.quotaService != nil && e.config.EnableQuotas {
		if err := e.quotaService.CheckUploadAllowed(ctx, userID, header.Size, header.Filename); err != nil {
			http.Error(w, err.Error(), http.StatusInsufficientStorage)
			return
		}
	}

	// Read file content
	content, err := io.ReadAll(file)
	if err != nil {
		http.Error(w, "Failed to read file", http.StatusInternalServerError)
		return
	}

	// Upload file
	filename := header.Filename
	contentType := header.Header.Get("Content-Type")
	if contentType == "" {
		contentType = "application/octet-stream"
	}

	// Parse userID as UUID if needed
	var userUUID *uuid.UUID
	if userID != "" {
		id, err := uuid.Parse(userID)
		if err == nil {
			userUUID = &id
		}
	}

	// Get parent folder ID from form (optional)
	parentFolderID := r.FormValue("parent_folder_id")
	var parentFolderPtr *string
	if parentFolderID != "" {
		parentFolderPtr = &parentFolderID
	}

	// AppID for the cloudstorage extension
	appID := "cloudstorage"

	obj, err := e.manager.UploadObject(ctx, bucketName, filename, parentFolderPtr, content, contentType, userUUID, &appID)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	// Update quota usage
	if e.quotaService != nil && e.config.EnableQuotas {
		if err := e.quotaService.UpdateStorageUsage(ctx, userID, header.Size); err != nil {
			// Log error but don't fail the upload
			fmt.Printf("Failed to update quota: %v\n", err)
		}
	}

	// Log the upload
	if e.accessLogService != nil && e.config.EnableAccessLogs {
		startTime := time.Now()
		success := true
		e.accessLogService.LogAccess(ctx, obj.ID, ActionUpload, LogOptions{
			UserID:    userID,
			IPAddress: parseIPAddress(r.RemoteAddr),
			UserAgent: r.UserAgent(),
			Success:   &success,
			BytesSize: header.Size,
			Duration:  time.Since(startTime),
		})
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(obj)
}

// handleStats returns storage statistics
func (e *CloudStorageExtension) handleStats(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	// SECURITY: Get user info from JWT token context, not headers
	user, ok := ctx.Value("user").(*auth.User)
	if !ok {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	userID := user.ID.String()
	// Check admin role via IAM service
	isAdmin := false
	if e.services != nil && e.services.IAM() != nil {
		isAdmin, _ = e.services.IAM().UserHasRole(userID, "admin")
	}

	stats := make(map[string]interface{})

	// Get storage stats
	var storageStats struct {
		TotalObjects int64 `json:"total_objects"`
		TotalSize    int64 `json:"total_size"`
	}

	// For admins, show all storage stats; for users, show only their own
	if isAdmin {
		e.db.Model(&pkgstorage.StorageObject{}).
			Select("COUNT(*) as total_objects, COALESCE(SUM(size), 0) as total_size").
			Scan(&storageStats)
	} else {
		e.db.Model(&pkgstorage.StorageObject{}).
			Where("user_id = ?", userID).
			Select("COUNT(*) as total_objects, COALESCE(SUM(size), 0) as total_size").
			Scan(&storageStats)
	}

	stats["storage"] = storageStats

	// Add provider type and bucket count
	if e.services != nil && e.services.Storage() != nil {
		// Get provider type from storage service
		if storageService, ok := e.services.Storage().(interface{ GetProviderType() string }); ok {
			stats["provider"] = storageService.GetProviderType()
		} else {
			stats["provider"] = "local" // Default to local
		}
	} else {
		stats["provider"] = "local"
	}

	// Get bucket count
	if e.manager != nil {
		buckets, _ := e.manager.ListBuckets(ctx)
		stats["total_buckets"] = len(buckets)
	} else {
		stats["total_buckets"] = 1 // Default int_storage bucket
	}

	// Get quota stats if enabled
	if e.quotaService != nil && e.config.EnableQuotas {
		if isAdmin {
			// Get overall quota usage for admin
			var totalQuotaStats struct {
				TotalUsers          int64 `json:"total_users"`
				TotalStorageUsed    int64 `json:"total_storage_used"`
				TotalStorageLimit   int64 `json:"total_storage_limit"`
				TotalBandwidthUsed  int64 `json:"total_bandwidth_used"`
				TotalBandwidthLimit int64 `json:"total_bandwidth_limit"`
			}
			e.db.Model(&StorageQuota{}).
				Select(`
					COUNT(*) as total_users,
					COALESCE(SUM(storage_used), 0) as total_storage_used,
					COALESCE(SUM(max_storage_bytes), 0) as total_storage_limit,
					COALESCE(SUM(bandwidth_used), 0) as total_bandwidth_used,
					COALESCE(SUM(max_bandwidth_bytes), 0) as total_bandwidth_limit
				`).
				Scan(&totalQuotaStats)

			var storagePercentage, bandwidthPercentage float64
			if totalQuotaStats.TotalStorageLimit > 0 {
				storagePercentage = float64(totalQuotaStats.TotalStorageUsed) / float64(totalQuotaStats.TotalStorageLimit) * 100
			}
			if totalQuotaStats.TotalBandwidthLimit > 0 {
				bandwidthPercentage = float64(totalQuotaStats.TotalBandwidthUsed) / float64(totalQuotaStats.TotalBandwidthLimit) * 100
			}

			// Count users near their storage limit (>80%)
			var usersNearLimit int64
			e.db.Model(&StorageQuota{}).
				Where("(storage_used * 100.0 / max_storage_bytes) > 80 OR (bandwidth_used * 100.0 / max_bandwidth_bytes) > 80").
				Count(&usersNearLimit)

			stats["quota"] = map[string]interface{}{
				"total_users":          totalQuotaStats.TotalUsers,
				"storage_used":         totalQuotaStats.TotalStorageUsed,
				"storage_limit":        totalQuotaStats.TotalStorageLimit,
				"storage_percentage":   storagePercentage,
				"bandwidth_used":       totalQuotaStats.TotalBandwidthUsed,
				"bandwidth_limit":      totalQuotaStats.TotalBandwidthLimit,
				"bandwidth_percentage": bandwidthPercentage,
				"users_near_limit":     usersNearLimit,
			}
		} else {
			quotaStats, err := e.quotaService.GetQuotaStats(ctx, userID)
			if err == nil {
				stats["quota"] = quotaStats
			}
		}
	}

	// Get share stats if enabled
	if e.shareService != nil && e.config.EnableSharing {
		var shareStats struct {
			TotalShares   int64 `json:"total_shares"`
			PublicShares  int64 `json:"public_shares"`
			PrivateShares int64 `json:"private_shares"`
			FoldersShared int64 `json:"folders_shared"`
			FilesShared   int64 `json:"files_shared"`
			ActiveShares  int64 `json:"active_shares"`
			ExpiredShares int64 `json:"expired_shares"`
		}

		if isAdmin {
			// Get all shares for admin
			e.db.Model(&StorageShare{}).Count(&shareStats.TotalShares)
			e.db.Model(&StorageShare{}).Where("is_public = ?", true).Count(&shareStats.PublicShares)
			e.db.Model(&StorageShare{}).Where("is_public = ?", false).Count(&shareStats.PrivateShares)
			e.db.Model(&StorageShare{}).Where("expires_at IS NULL OR expires_at > ?", time.Now()).Count(&shareStats.ActiveShares)
			e.db.Model(&StorageShare{}).Where("expires_at IS NOT NULL AND expires_at <= ?", time.Now()).Count(&shareStats.ExpiredShares)

			// Count shared folders vs files
			e.db.Table("ext_cloudstorage_storage_shares ss").
				Joins("JOIN storage_objects so ON ss.object_id = so.id").
				Where("so.content_type = 'application/x-directory'").
				Count(&shareStats.FoldersShared)
			shareStats.FilesShared = shareStats.TotalShares - shareStats.FoldersShared
		} else {
			// Get user's own shares
			e.db.Model(&StorageShare{}).Where("created_by = ?", userID).Count(&shareStats.TotalShares)
			e.db.Model(&StorageShare{}).Where("created_by = ? AND is_public = ?", userID, true).Count(&shareStats.PublicShares)
			e.db.Model(&StorageShare{}).Where("created_by = ? AND is_public = ?", userID, false).Count(&shareStats.PrivateShares)
			e.db.Model(&StorageShare{}).Where("created_by = ? AND (expires_at IS NULL OR expires_at > ?)", userID, time.Now()).Count(&shareStats.ActiveShares)
			e.db.Model(&StorageShare{}).Where("created_by = ? AND expires_at IS NOT NULL AND expires_at <= ?", userID, time.Now()).Count(&shareStats.ExpiredShares)

			// Count shared folders vs files for user
			e.db.Table("ext_cloudstorage_storage_shares ss").
				Joins("JOIN storage_objects so ON ss.object_id = so.id").
				Where("ss.created_by = ? AND so.content_type = 'application/x-directory'", userID).
				Count(&shareStats.FoldersShared)
			shareStats.FilesShared = shareStats.TotalShares - shareStats.FoldersShared
		}

		stats["shares"] = shareStats
	}

	// Get access stats if enabled
	if e.accessLogService != nil && e.config.EnableAccessLogs {
		if isAdmin {
			// Get overall access stats for admin
			accessStats, err := e.accessLogService.GetAccessStats(ctx, StatsFilters{})
			if err == nil {
				stats["access"] = accessStats
			}
		} else {
			accessStats, err := e.accessLogService.GetAccessStats(ctx, StatsFilters{UserID: userID})
			if err == nil {
				stats["access"] = accessStats
			}
		}
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(stats)
}

// handleBuckets lists all buckets (for compatibility)
func (e *CloudStorageExtension) handleBuckets(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()

	buckets, err := e.manager.ListBuckets(ctx)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(buckets)
}

// handleDownload handles file downloads with bandwidth tracking
func (e *CloudStorageExtension) handleDownload(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	// SECURITY: Get user info from JWT token context, not headers
	user, ok := ctx.Value("user").(*auth.User)
	if !ok {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	userID := user.ID.String()

	// Get object ID from query params
	objectID := r.URL.Query().Get("id")
	if objectID == "" {
		http.Error(w, "Object ID is required", http.StatusBadRequest)
		return
	}

	// Get the object
	var obj pkgstorage.StorageObject
	if err := e.db.Where("id = ?", objectID).First(&obj).Error; err != nil {
		http.Error(w, "Object not found", http.StatusNotFound)
		return
	}

	// Check if user has access (owns the file or it's public)
	// TODO: Add more sophisticated access control
	if obj.UserID != userID {
		// Check if there's a public share for this object
		var share StorageShare
		if err := e.db.Where("object_id = ? AND is_public = ?", objectID, true).First(&share).Error; err != nil {
			http.Error(w, "Access denied", http.StatusForbidden)
			return
		}
	}

	// Download the file
	content, contentType, err := e.manager.GetFile(ctx, obj.ID)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	// Update bandwidth usage for the file owner
	if e.quotaService != nil && e.config.EnableQuotas && obj.UserID != "" {
		if err := e.quotaService.UpdateBandwidthUsage(ctx, obj.UserID, obj.Size); err != nil {
			// Log error but don't fail the download
			fmt.Printf("Failed to update bandwidth usage: %v\n", err)
		}
	}

	// Log the download
	if e.accessLogService != nil && e.config.EnableAccessLogs {
		success := true
		e.accessLogService.LogAccess(ctx, obj.ID, ActionDownload, LogOptions{
			UserID:    userID,
			IPAddress: parseIPAddress(r.RemoteAddr),
			UserAgent: r.UserAgent(),
			Success:   &success,
			BytesSize: obj.Size,
		})
	}

	// Set headers
	w.Header().Set("Content-Type", contentType)
	w.Header().Set("Content-Length", strconv.FormatInt(obj.Size, 10))
	w.Header().Set("Content-Disposition", fmt.Sprintf("attachment; filename=\"%s\"", obj.ObjectName))

	// Write content
	w.Write(content)
}

// handleUserSearch handles user search for admin panel
func (e *CloudStorageExtension) handleUserSearch(w http.ResponseWriter, r *http.Request) {
	// SECURITY: Get user info from JWT token context, not headers
	user, ok := r.Context().Value("user").(*auth.User)
	if !ok {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	// Check admin role via IAM service
	isAdmin := false
	if e.services != nil && e.services.IAM() != nil {
		isAdmin, _ = e.services.IAM().UserHasRole(user.ID.String(), "admin")
	}

	// Only admins can search users
	if !isAdmin {
		http.Error(w, "Admin access required", http.StatusForbidden)
		return
	}

	query := r.URL.Query().Get("q")
	if query == "" || len(query) < 2 {
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte("[]"))
		return
	}

	// Search for users by email, name, or ID
	type UserSearchResult struct {
		ID    string `json:"id"`
		Email string `json:"email"`
		Name  string `json:"name,omitempty"`
	}

	var users []UserSearchResult

	// Search in auth_users table
	e.db.Table("auth_users").
		Select("id, email, COALESCE(raw_user_meta_data->>'name', '') as name").
		Where("email ILIKE ? OR id::text ILIKE ? OR raw_user_meta_data->>'name' ILIKE ?",
			"%"+query+"%", "%"+query+"%", "%"+query+"%").
		Limit(10).
		Scan(&users)

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(users)
}

// handleGetRoleQuotas returns all role quotas
func (e *CloudStorageExtension) handleGetRoleQuotas(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	// SECURITY: Get user info from JWT token context, not headers
	user, ok := ctx.Value("user").(*auth.User)
	// Check admin role via IAM service
	isAdmin := false
	if ok && e.services != nil && e.services.IAM() != nil {
		isAdmin, _ = e.services.IAM().UserHasRole(user.ID.String(), "admin")
	}
	if !ok || !isAdmin {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	var quotas []RoleQuota
	if err := e.db.Find(&quotas).Error; err != nil {
		http.Error(w, "Failed to retrieve role quotas", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(quotas)
}

// handleUpdateRoleQuota updates quota for a specific role
func (e *CloudStorageExtension) handleUpdateRoleQuota(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	// SECURITY: Get user info from JWT token context, not headers  
	user, ok := ctx.Value("user").(*auth.User)
	// Check admin role via IAM service
	isAdmin := false
	if ok && e.services != nil && e.services.IAM() != nil {
		isAdmin, _ = e.services.IAM().UserHasRole(user.ID.String(), "admin")
	}
	if !ok || !isAdmin {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	// Get role ID from URL path
	parts := strings.Split(r.URL.Path, "/")
	if len(parts) < 2 {
		http.Error(w, "Invalid role ID", http.StatusBadRequest)
		return
	}
	roleID := parts[len(parts)-1]

	var update RoleQuota
	if err := json.NewDecoder(r.Body).Decode(&update); err != nil {
		http.Error(w, "Invalid request body", http.StatusBadRequest)
		return
	}

	// Validate input
	if update.MaxStorageBytes < 0 || update.MaxBandwidthBytes < 0 || 
	   update.MaxUploadSize < 0 || update.MaxFilesCount < 0 {
		http.Error(w, "Invalid quota values: cannot be negative", http.StatusBadRequest)
		return
	}
	
	// Validate reasonable limits (max 10TB storage, 100TB bandwidth)
	if update.MaxStorageBytes > 10995116277760 || update.MaxBandwidthBytes > 109951162777600 {
		http.Error(w, "Quota values exceed maximum allowed limits", http.StatusBadRequest)
		return
	}

	update.RoleID = roleID
	
	// Update or create role quota
	if err := e.db.Where("role_id = ?", roleID).
		Assign(update).
		FirstOrCreate(&RoleQuota{}).Error; err != nil {
		http.Error(w, "Failed to update role quota", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(update)
}

// handleGetUserOverrides returns all user quota overrides
func (e *CloudStorageExtension) handleGetUserOverrides(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	// SECURITY: Get user info from JWT token context, not headers
	user, ok := ctx.Value("user").(*auth.User)
	// Check admin role via IAM service
	isAdmin := false
	if ok && e.services != nil && e.services.IAM() != nil {
		isAdmin, _ = e.services.IAM().UserHasRole(user.ID.String(), "admin")
	}
	if !ok || !isAdmin {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	var overrides []UserQuotaOverride
	if err := e.db.Where("expires_at IS NULL OR expires_at > NOW()").
		Find(&overrides).Error; err != nil {
		http.Error(w, "Failed to retrieve user overrides", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(overrides)
}

// handleCreateUserOverride creates a new user quota override
func (e *CloudStorageExtension) handleCreateUserOverride(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	// SECURITY: Get user info from JWT token context, not headers
	user, ok := ctx.Value("user").(*auth.User)
	// Check admin role via IAM service
	isAdmin := false
	if ok && e.services != nil && e.services.IAM() != nil {
		isAdmin, _ = e.services.IAM().UserHasRole(user.ID.String(), "admin")
	}
	if !ok || !isAdmin {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	var override UserQuotaOverride
	if err := json.NewDecoder(r.Body).Decode(&override); err != nil {
		http.Error(w, "Invalid request body", http.StatusBadRequest)
		return
	}

	// Validate required fields
	if override.UserID == "" {
		http.Error(w, "User ID is required", http.StatusBadRequest)
		return
	}
	
	// Validate quota values if provided
	if override.MaxStorageBytes != nil && *override.MaxStorageBytes < 0 {
		http.Error(w, "Invalid storage quota: cannot be negative", http.StatusBadRequest)
		return
	}
	if override.MaxBandwidthBytes != nil && *override.MaxBandwidthBytes < 0 {
		http.Error(w, "Invalid bandwidth quota: cannot be negative", http.StatusBadRequest)
		return
	}
	if override.MaxUploadSize != nil && *override.MaxUploadSize < 0 {
		http.Error(w, "Invalid upload size: cannot be negative", http.StatusBadRequest)
		return
	}
	if override.MaxFilesCount != nil && *override.MaxFilesCount < 0 {
		http.Error(w, "Invalid file count: cannot be negative", http.StatusBadRequest)
		return
	}

	override.CreatedBy = user.ID.String()
	override.ID = uuid.New().String()

	if err := e.db.Create(&override).Error; err != nil {
		http.Error(w, "Failed to create user override", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(override)
}

// handleDeleteUserOverride deletes a user quota override
func (e *CloudStorageExtension) handleDeleteUserOverride(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	// SECURITY: Get user info from JWT token context, not headers
	user, ok := ctx.Value("user").(*auth.User)
	// Check admin role via IAM service
	isAdmin := false
	if ok && e.services != nil && e.services.IAM() != nil {
		isAdmin, _ = e.services.IAM().UserHasRole(user.ID.String(), "admin")
	}
	if !ok || !isAdmin {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	// Get override ID from URL path
	parts := strings.Split(r.URL.Path, "/")
	if len(parts) < 2 {
		http.Error(w, "Invalid override ID", http.StatusBadRequest)
		return
	}
	overrideID := parts[len(parts)-1]

	if err := e.db.Where("id = ?", overrideID).
		Delete(&UserQuotaOverride{}).Error; err != nil {
		http.Error(w, "Failed to delete user override", http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// handleRoleQuotas handles both GET and PUT for role quotas
func (e *CloudStorageExtension) handleRoleQuotas(w http.ResponseWriter, r *http.Request) {
	switch r.Method {
	case http.MethodGet:
		e.handleGetRoleQuotas(w, r)
	case http.MethodPost, http.MethodPut:
		e.handleUpdateRoleQuota(w, r)
	default:
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
	}
}

// handleUserOverrides handles both GET and POST for user overrides
func (e *CloudStorageExtension) handleUserOverrides(w http.ResponseWriter, r *http.Request) {
	switch r.Method {
	case http.MethodGet:
		e.handleGetUserOverrides(w, r)
	case http.MethodPost:
		e.handleCreateUserOverride(w, r)
	default:
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
	}
}

// handleGetUserQuota returns effective quota for a specific user
func (e *CloudStorageExtension) handleGetUserQuota(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	// SECURITY: Get user info from JWT token context, not headers
	user, ok := ctx.Value("user").(*auth.User)
	if !ok {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	// Get user ID from query or use current user
	userID := r.URL.Query().Get("user_id")
	if userID == "" {
		userID = user.ID.String()
	} else {
		// Check if user is admin via IAM service
		isAdmin := false
		if e.services != nil && e.services.IAM() != nil {
			isAdmin, _ = e.services.IAM().UserHasRole(user.ID.String(), "admin")
		}
		if !isAdmin && userID != user.ID.String() {
			// Non-admins can only view their own quota
			http.Error(w, "Forbidden", http.StatusForbidden)
			return
		}
	}

	if e.quotaService == nil {
		http.Error(w, "Quota service not initialized", http.StatusInternalServerError)
		return
	}

	quota, err := e.quotaService.GetUserQuota(ctx, userID)
	if err != nil {
		http.Error(w, fmt.Sprintf("Failed to get user quota: %v", err), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(quota)
}
