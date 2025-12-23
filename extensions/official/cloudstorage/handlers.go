package cloudstorage

import (
	auth "github.com/suppers-ai/solobase/internal/pkg/auth"
)

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strconv"
	"strings"

	googleuuid "github.com/google/uuid"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	"github.com/suppers-ai/solobase/utils"
)

// Response structs
type ShareResponse struct {
	ID              string     `json:"id"`
	ShareURL        string     `json:"shareUrl,omitempty"`
	ShareToken      string     `json:"shareToken,omitempty"`
	ExpiresAt       *apptime.Time `json:"expiresAt,omitempty"`
	PermissionLevel string     `json:"permissionLevel"`
}

type QuotaResponse struct {
	StorageUsed         int64      `json:"storageUsed"`
	StorageLimit        int64      `json:"storageLimit"`
	StoragePercentage   float64    `json:"storagePercentage"`
	BandwidthUsed       int64      `json:"bandwidthUsed"`
	BandwidthLimit      int64      `json:"bandwidthLimit"`
	BandwidthPercentage float64    `json:"bandwidthPercentage"`
	ResetDate           *apptime.Time `json:"resetDate,omitempty"`
}

// handleShares manages file sharing operations
func (e *CloudStorageExtension) handleShares(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	// SECURITY: Get user info from JWT token context, not headers
	user, ok := ctx.Value("user").(*auth.User)
	if !ok {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	userID := user.ID.String()

	// Check for special share types
	shareType := r.URL.Query().Get("type")
	if shareType == "shared-with-me" {
		e.handleSharedWithMe(w, r, userID)
		return
	} else if shareType == "shared-by-me" {
		e.handleSharedByMe(w, r, userID)
		return
	}

	// If share service is not initialized, return empty array for GET, error for POST
	if e.shareService == nil {
		if r.Method == http.MethodGet {
			// Return empty array for GET requests
			w.Header().Set("Content-Type", "application/json")
			json.NewEncoder(w).Encode([]interface{}{})
			return
		}
		http.Error(w, "Sharing is not enabled", http.StatusNotImplemented)
		return
	}

	switch r.Method {
	case http.MethodPost:
		// Create a new share
		var req struct {
			ObjectID          string     `json:"objectId"`
			SharedWithUserID  string     `json:"sharedWithUserId,omitempty"`
			SharedWithEmail   string     `json:"sharedWithEmail,omitempty"`
			PermissionLevel   string     `json:"permissionLevel"`
			InheritToChildren bool       `json:"inheritToChildren"`
			GenerateToken     bool       `json:"generateToken"`
			IsPublic          bool       `json:"isPublic"`
			ExpiresAt         *apptime.Time `json:"expiresAt,omitempty"`
		}

		if !utils.DecodeJSONBody(w, r, &req) {
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
			ExpiresAt:       share.ExpiresAt.ToTimePtr(),
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
	obj, err := e.getStorageObjectByID(share.ObjectID)
	if err != nil {
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
			if updateErr := e.quotaService.UpdateBandwidthUsage(ctx, obj.UserID, obj.Size); updateErr != nil {
				// Log error but don't fail the download
				fmt.Printf("Failed to update bandwidth usage: %v\n", updateErr)
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
			quotas, err := e.getAllStorageQuotas()
			if err != nil {
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
			UserID            string `json:"userId"`
			MaxStorageBytes   int64  `json:"maxStorageBytes"`
			MaxBandwidthBytes int64  `json:"maxBandwidthBytes"`
		}

		if !utils.DecodeJSONBody(w, r, &req) {
			return
		}

		quota, err := e.quotaService.GetOrCreateQuota(ctx, req.UserID)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		quota.MaxStorageBytes = req.MaxStorageBytes
		quota.MaxBandwidthBytes = req.MaxBandwidthBytes
		quota.UpdatedAt = apptime.NowTime()

		if err := e.saveStorageQuota(quota); err != nil {
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
		if start, err := apptime.ParseWithLayout(apptime.TimeFormat, startStr); err == nil {
			filters.StartDate = &start
		}
	}
	if endStr := r.URL.Query().Get("end_date"); endStr != "" {
		if end, err := apptime.ParseWithLayout(apptime.TimeFormat, endStr); err == nil {
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
		if start, err := apptime.ParseWithLayout(apptime.TimeFormat, startStr); err == nil {
			filters.StartDate = &start
		}
	}
	if endStr := r.URL.Query().Get("end_date"); endStr != "" {
		if end, err := apptime.ParseWithLayout(apptime.TimeFormat, endStr); err == nil {
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

	// Parse userID as UUID if needed (using google/uuid for storage manager compatibility)
	var userUUID *googleuuid.UUID
	if userID != "" {
		id, err := googleuuid.Parse(userID)
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
		startTime := apptime.NowTime()
		success := true
		e.accessLogService.LogAccess(ctx, obj.ID, ActionUpload, LogOptions{
			UserID:    userID,
			IPAddress: parseIPAddress(r.RemoteAddr),
			UserAgent: r.UserAgent(),
			Success:   &success,
			BytesSize: header.Size,
			Duration:  apptime.Since(startTime),
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
	totalObjects, totalSize, _ := e.getStorageStats(userID, isAdmin)
	storageStats := struct {
		TotalObjects int64 `json:"totalObjects"`
		TotalSize    int64 `json:"totalSize"`
	}{
		TotalObjects: totalObjects,
		TotalSize:    totalSize,
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
		stats["totalBuckets"] = len(buckets)
	} else {
		stats["totalBuckets"] = 1 // Default int_storage bucket
	}

	// Get quota stats if enabled
	if e.quotaService != nil && e.config.EnableQuotas {
		if isAdmin {
			// Get overall quota usage for admin
			totalUsers, totalStorageUsed, totalStorageLimit, totalBandwidthUsed, totalBandwidthLimit, _ := e.getQuotaAggregateStats()

			var storagePercentage, bandwidthPercentage float64
			if totalStorageLimit > 0 {
				storagePercentage = float64(totalStorageUsed) / float64(totalStorageLimit) * 100
			}
			if totalBandwidthLimit > 0 {
				bandwidthPercentage = float64(totalBandwidthUsed) / float64(totalBandwidthLimit) * 100
			}

			// Count users near their storage limit (>80%)
			usersNearLimit, _ := e.countUsersNearQuotaLimit()

			stats["quota"] = map[string]interface{}{
				"totalUsers":          totalUsers,
				"storageUsed":         totalStorageUsed,
				"storageLimit":        totalStorageLimit,
				"storagePercentage":   storagePercentage,
				"bandwidthUsed":       totalBandwidthUsed,
				"bandwidthLimit":      totalBandwidthLimit,
				"bandwidthPercentage": bandwidthPercentage,
				"usersNearLimit":      usersNearLimit,
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
			TotalShares   int64 `json:"totalShares"`
			PublicShares  int64 `json:"publicShares"`
			PrivateShares int64 `json:"privateShares"`
			FoldersShared int64 `json:"foldersShared"`
			FilesShared   int64 `json:"filesShared"`
			ActiveShares  int64 `json:"activeShares"`
			ExpiredShares int64 `json:"expiredShares"`
		}

		now := apptime.NowTime().Format("2006-01-02 15:04:05")
		if isAdmin {
			// Get all shares for admin
			shareStats.TotalShares, _ = e.countShares("", nil)
			shareStats.PublicShares, _ = e.countShares("is_public = 1")
			shareStats.PrivateShares, _ = e.countShares("is_public = 0")
			shareStats.ActiveShares, _ = e.countShares("expires_at IS NULL OR expires_at > ?", now)
			shareStats.ExpiredShares, _ = e.countShares("expires_at IS NOT NULL AND expires_at <= ?", now)

			// Count shared folders vs files
			shareStats.FoldersShared, _ = e.countSharedFolders("", true)
			shareStats.FilesShared = shareStats.TotalShares - shareStats.FoldersShared
		} else {
			// Get user's own shares
			shareStats.TotalShares, _ = e.countShares("created_by = ?", userID)
			shareStats.PublicShares, _ = e.countShares("created_by = ? AND is_public = 1", userID)
			shareStats.PrivateShares, _ = e.countShares("created_by = ? AND is_public = 0", userID)
			shareStats.ActiveShares, _ = e.countShares("created_by = ? AND (expires_at IS NULL OR expires_at > ?)", userID, now)
			shareStats.ExpiredShares, _ = e.countShares("created_by = ? AND expires_at IS NOT NULL AND expires_at <= ?", userID, now)

			// Count shared folders vs files for user
			shareStats.FoldersShared, _ = e.countSharedFolders(userID, false)
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
	obj, err := e.getStorageObjectByID(objectID)
	if err != nil {
		http.Error(w, "Object not found", http.StatusNotFound)
		return
	}

	// Check if user has access
	hasAccess := false
	var accessShare *StorageShare

	// Check if user owns the file
	if obj.UserID == userID {
		hasAccess = true
	} else {
		// Check for direct share or public access
		share, err := e.getShareByObjectAndUser(objectID, userID)
		if err == nil && share != nil {
			hasAccess = true
			accessShare = share
		} else if e.shareService != nil {
			// Check for inherited permissions from parent folders
			// Get user's email for email-based shares
			userEmail, _ := e.getUserEmail(userID)

			inheritedShare, err := e.shareService.CheckInheritedPermissions(ctx, objectID, userID, userEmail)
			if err == nil && inheritedShare != nil {
				hasAccess = true
				accessShare = inheritedShare
			}
		}
	}

	if !hasAccess {
		http.Error(w, "Access denied", http.StatusForbidden)
		return
	}

	// Check permission level if accessing through a share
	if accessShare != nil && accessShare.PermissionLevel == PermissionView && r.Method != http.MethodGet {
		http.Error(w, "Permission denied for this operation", http.StatusForbidden)
		return
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

	// Search for users by email or ID
	users, err := e.searchUsers(query)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte("[]"))
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(users)
}

// handleGetRoleQuotas returns all role quotas
func (e *CloudStorageExtension) handleGetRoleQuotas(w http.ResponseWriter, r *http.Request) {
	// Note: Admin check is already done by AdminMiddleware in router
	// This handler is only called for admin routes

	if e.sqlDB == nil {
		// Return empty array if database not initialized
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode([]RoleQuota{})
		return
	}

	quotas, err := e.getAllRoleQuotas()
	if err != nil {
		// Return empty array on error instead of 500
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode([]RoleQuota{})
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(quotas)
}

// handleUpdateRoleQuota updates quota for a specific role
func (e *CloudStorageExtension) handleUpdateRoleQuota(w http.ResponseWriter, r *http.Request) {
	// Note: Admin check is already done by AdminMiddleware in router

	// Get role ID from URL path
	parts := strings.Split(r.URL.Path, "/")
	if len(parts) < 2 {
		http.Error(w, "Invalid role ID", http.StatusBadRequest)
		return
	}
	roleID := parts[len(parts)-1]

	var update RoleQuota
	if !utils.DecodeJSONBody(w, r, &update) {
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
	if update.ID == "" {
		update.ID = uuid.New().String()
	}

	// Update or create role quota
	if err := e.upsertRoleQuota(&update); err != nil {
		http.Error(w, "Failed to update role quota", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(update)
}

// handleGetUserOverrides returns all user quota overrides
func (e *CloudStorageExtension) handleGetUserOverrides(w http.ResponseWriter, r *http.Request) {
	// Note: Admin check is already done by AdminMiddleware in router

	if e.sqlDB == nil {
		// Return empty array if database not initialized
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode([]UserQuotaOverride{})
		return
	}

	overrides, err := e.getActiveUserQuotaOverrides()
	if err != nil {
		// Return empty array on error instead of 500
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode([]UserQuotaOverride{})
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(overrides)
}

// handleCreateUserOverride creates a new user quota override
func (e *CloudStorageExtension) handleCreateUserOverride(w http.ResponseWriter, r *http.Request) {
	// Note: Admin check is already done by AdminMiddleware in router
	ctx := r.Context()

	// Get user for audit purposes
	user, _ := ctx.Value("user").(*auth.User)

	var override UserQuotaOverride
	if !utils.DecodeJSONBody(w, r, &override) {
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
	override.CreatedAt = apptime.NowTime()
	override.UpdatedAt = apptime.NowTime()

	if err := e.createUserQuotaOverride(&override); err != nil {
		http.Error(w, "Failed to create user override", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(override)
}

// handleDeleteUserOverride deletes a user quota override
func (e *CloudStorageExtension) handleDeleteUserOverride(w http.ResponseWriter, r *http.Request) {
	// Note: Admin check is already done by AdminMiddleware in router

	// Get override ID from URL path
	parts := strings.Split(r.URL.Path, "/")
	if len(parts) < 2 {
		http.Error(w, "Invalid override ID", http.StatusBadRequest)
		return
	}
	overrideID := parts[len(parts)-1]

	if err := e.deleteUserQuotaOverrideByID(overrideID); err != nil {
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

// handleDefaultQuotas manages default quota settings
func (e *CloudStorageExtension) handleDefaultQuotas(w http.ResponseWriter, r *http.Request) {
	// Note: Admin check is already done by AdminMiddleware in router

	if e.quotaService == nil {
		http.Error(w, "Quota service not available", http.StatusServiceUnavailable)
		return
	}

	switch r.Method {
	case http.MethodGet:
		// Get default quotas
		quotas := map[string]interface{}{
			"defaultStorage":   e.config.DefaultStorageLimit,
			"defaultBandwidth": e.config.DefaultBandwidthLimit,
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(quotas)

	case http.MethodPut:
		// Update default quotas
		var req struct {
			DefaultStorage   int64 `json:"defaultStorage"`
			DefaultBandwidth int64 `json:"defaultBandwidth"`
		}

		if !utils.DecodeJSONBody(w, r, &req) {
			return
		}

		// Update configuration
		e.config.DefaultStorageLimit = req.DefaultStorage
		e.config.DefaultBandwidthLimit = req.DefaultBandwidth

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]string{
			"message": "Default quotas updated successfully",
		})

	default:
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
	}
}

// handleSharedWithMe returns items shared with the current user
func (e *CloudStorageExtension) handleSharedWithMe(w http.ResponseWriter, r *http.Request, userID string) {
	if e.shareService == nil {
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(struct {
			Files   []interface{} `json:"files"`
			Folders []interface{} `json:"folders"`
		}{
			Files:   []interface{}{},
			Folders: []interface{}{},
		})
		return
	}

	ctx := r.Context()
	shares, err := e.shareService.GetSharedWithMe(ctx, userID)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	// Separate files and folders
	files := []StorageShareWithObject{}
	folders := []StorageShareWithObject{}

	for _, share := range shares {
		if share.ContentType == "application/x-directory" {
			folders = append(folders, share)
		} else {
			files = append(files, share)
		}
	}

	result := struct {
		Files   []StorageShareWithObject `json:"files"`
		Folders []StorageShareWithObject `json:"folders"`
	}{
		Files:   files,
		Folders: folders,
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(result)
}

// handleSharedByMe returns items shared by the current user
func (e *CloudStorageExtension) handleSharedByMe(w http.ResponseWriter, r *http.Request, userID string) {
	if e.shareService == nil {
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode([]interface{}{})
		return
	}

	ctx := r.Context()
	shares, err := e.shareService.GetSharedByMe(ctx, userID)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(shares)
}
