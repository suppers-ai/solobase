package files

import (
	"context"
	"fmt"
	"io"
	"strconv"

	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/solobase/core/uuid"
	wafer "github.com/wafer-run/wafer-go"
)

// ========== Cloud Stats ==========

func (b *FilesBlock) handleCloudStats(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Unauthorized")
	}
	isAdmin := msg.IsAdmin()

	stats := make(map[string]any)
	totalObjects, totalSize, _ := b.getStorageStats(userID, isAdmin)
	stats["storage"] = map[string]any{
		"totalObjects": totalObjects,
		"totalSize":    totalSize,
	}
	stats["provider"] = "local"

	if b.storageSvc != nil {
		folders, _ := b.storageSvc.ListFolders(context.Background())
		stats["totalBuckets"] = len(folders)
	} else {
		stats["totalBuckets"] = 1
	}

	if b.quotaService != nil && b.cloudConfig.EnableQuotas {
		if isAdmin {
			totalUsers, totalStorageUsed, totalStorageLimit, totalBandwidthUsed, totalBandwidthLimit, _ := b.getQuotaAggregateStats()
			var storagePercentage, bandwidthPercentage float64
			if totalStorageLimit > 0 {
				storagePercentage = float64(totalStorageUsed) / float64(totalStorageLimit) * 100
			}
			if totalBandwidthLimit > 0 {
				bandwidthPercentage = float64(totalBandwidthUsed) / float64(totalBandwidthLimit) * 100
			}
			usersNearLimit, _ := b.countUsersNearQuotaLimit()
			stats["quota"] = map[string]any{
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
			quotaStats, err := b.quotaService.GetQuotaStats(context.Background(), userID)
			if err == nil {
				stats["quota"] = quotaStats
			}
		}
	}

	if b.shareService != nil && b.cloudConfig.EnableSharing {
		now := apptime.NowTime().Format("2006-01-02 15:04:05")
		shareStats := map[string]any{}
		if isAdmin {
			total, _ := b.countShares("", nil)
			publicShares, _ := b.countShares("is_public = 1")
			privateShares, _ := b.countShares("is_public = 0")
			activeShares, _ := b.countShares("expires_at IS NULL OR expires_at > ?", now)
			expiredShares, _ := b.countShares("expires_at IS NOT NULL AND expires_at <= ?", now)
			foldersShared, _ := b.countSharedFolders("", true)
			shareStats["totalShares"] = total
			shareStats["publicShares"] = publicShares
			shareStats["privateShares"] = privateShares
			shareStats["activeShares"] = activeShares
			shareStats["expiredShares"] = expiredShares
			shareStats["foldersShared"] = foldersShared
			shareStats["filesShared"] = total - foldersShared
		} else {
			total, _ := b.countShares("created_by = ?", userID)
			publicShares, _ := b.countShares("created_by = ? AND is_public = 1", userID)
			privateShares, _ := b.countShares("created_by = ? AND is_public = 0", userID)
			activeShares, _ := b.countShares("created_by = ? AND (expires_at IS NULL OR expires_at > ?)", userID, now)
			expiredShares, _ := b.countShares("created_by = ? AND expires_at IS NOT NULL AND expires_at <= ?", userID, now)
			foldersShared, _ := b.countSharedFolders(userID, false)
			shareStats["totalShares"] = total
			shareStats["publicShares"] = publicShares
			shareStats["privateShares"] = privateShares
			shareStats["activeShares"] = activeShares
			shareStats["expiredShares"] = expiredShares
			shareStats["foldersShared"] = foldersShared
			shareStats["filesShared"] = total - foldersShared
		}
		stats["shares"] = shareStats
	}

	if b.accessLogService != nil && b.cloudConfig.EnableAccessLogs {
		if isAdmin {
			accessStats, err := b.accessLogService.GetAccessStats(context.Background(), StatsFilters{})
			if err == nil {
				stats["access"] = accessStats
			}
		} else {
			accessStats, err := b.accessLogService.GetAccessStats(context.Background(), StatsFilters{UserID: userID})
			if err == nil {
				stats["access"] = accessStats
			}
		}
	}

	return wafer.JSONRespond(msg, 200, stats)
}

// ========== Shares ==========

func (b *FilesBlock) handleSharesGet(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	shareType := msg.Query("type")
	if shareType == "shared-with-me" {
		return b.handleSharedWithMe(msg, userID)
	} else if shareType == "shared-by-me" {
		return b.handleSharedByMe(msg, userID)
	}

	if b.shareService == nil {
		return wafer.JSONRespond(msg, 200, []any{})
	}

	shares, err := b.shareService.GetUserShares(context.Background(), userID)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 200, shares)
}

func (b *FilesBlock) handleSharesPost(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	if b.shareService == nil {
		return wafer.Error(msg, 501, "not_implemented", "Sharing is not enabled")
	}

	var body struct {
		ObjectID          string        `json:"objectId"`
		SharedWithUserID  string        `json:"sharedWithUserId,omitempty"`
		SharedWithEmail   string        `json:"sharedWithEmail,omitempty"`
		PermissionLevel   string        `json:"permissionLevel"`
		InheritToChildren bool          `json:"inheritToChildren"`
		GenerateToken     bool          `json:"generateToken"`
		IsPublic          bool          `json:"isPublic"`
		ExpiresAt         *apptime.Time `json:"expiresAt,omitempty"`
	}
	if err := msg.Decode(&body); err != nil {
		return wafer.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	opts := ShareOptions{
		SharedWithUserID:  body.SharedWithUserID,
		SharedWithEmail:   body.SharedWithEmail,
		PermissionLevel:   PermissionLevel(body.PermissionLevel),
		InheritToChildren: body.InheritToChildren,
		GenerateToken:     body.GenerateToken,
		IsPublic:          body.IsPublic,
		ExpiresAt:         body.ExpiresAt,
	}

	share, err := b.shareService.CreateShare(context.Background(), body.ObjectID, userID, opts)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}

	if b.accessLogService != nil {
		b.accessLogService.LogAccess(context.Background(), body.ObjectID, ActionShare, LogOptions{
			UserID:    userID,
			IPAddress: parseIPAddress(msg.RemoteAddr()),
			UserAgent: msg.Header("User-Agent"),
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

	return wafer.JSONRespond(msg, 201, response)
}

func (b *FilesBlock) handleShareAccess(_ wafer.Context, msg *wafer.Message) wafer.Result {
	if b.shareService == nil {
		return wafer.Error(msg, 501, "not_implemented", "Sharing is not enabled")
	}

	token := msg.Var("token")
	if token == "" {
		return wafer.Error(msg, 400, "bad_request", "Invalid URL")
	}

	share, err := b.shareService.GetShareByToken(context.Background(), token)
	if err != nil {
		return wafer.Error(msg, 404, "not_found", err.Error())
	}

	obj, err := b.getStorageObjectByID(share.ObjectID)
	if err != nil {
		return wafer.Error(msg, 404, "not_found", "Object not found")
	}

	if b.accessLogService != nil {
		b.accessLogService.LogAccess(context.Background(), share.ObjectID, ActionView, LogOptions{
			ShareID:   share.ID,
			IPAddress: parseIPAddress(msg.RemoteAddr()),
			UserAgent: msg.Header("User-Agent"),
		})
	}

	// Download the file via storage service
	if b.storageSvc == nil {
		return wafer.Error(msg, 503, "unavailable", "Storage service not available")
	}
	reader, _, err := b.storageSvc.Get(context.Background(), obj.BucketName, obj.ID)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	defer reader.Close()
	content, err := io.ReadAll(reader)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}

	if b.quotaService != nil && b.cloudConfig.EnableQuotas && obj.UserID != "" {
		_ = b.quotaService.UpdateBandwidthUsage(context.Background(), obj.UserID, obj.Size)
	}

	return wafer.NewResponse(msg, 200).
		SetHeader("Content-Length", strconv.FormatInt(obj.Size, 10)).
		SetHeader("Content-Disposition", fmt.Sprintf("attachment; filename=\"%s\"", obj.ObjectName)).
		Body(content, obj.ContentType)
}

// ========== Quotas ==========

func (b *FilesBlock) handleCloudQuotaGet(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	if b.quotaService == nil {
		return wafer.Error(msg, 501, "not_implemented", "Quotas are not enabled")
	}

	isAdmin := msg.IsAdmin()
	queryUserID := msg.Query("user_id")

	if queryUserID != "" && isAdmin {
		stats, err := b.quotaService.GetQuotaStats(context.Background(), queryUserID)
		if err != nil {
			return wafer.Error(msg, 500, "internal_error", err.Error())
		}
		return wafer.JSONRespond(msg, 200, stats)
	} else if isAdmin && queryUserID == "" {
		quotas, err := b.getAllStorageQuotas()
		if err != nil {
			return wafer.Error(msg, 500, "internal_error", err.Error())
		}
		return wafer.JSONRespond(msg, 200, quotas)
	}

	stats, err := b.quotaService.GetQuotaStats(context.Background(), userID)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}
	return wafer.JSONRespond(msg, 200, stats)
}

func (b *FilesBlock) handleCloudQuotaPut(_ wafer.Context, msg *wafer.Message) wafer.Result {
	if !msg.IsAdmin() {
		return wafer.Error(msg, 403, "forbidden", "Admin access required")
	}

	if b.quotaService == nil {
		return wafer.Error(msg, 501, "not_implemented", "Quotas are not enabled")
	}

	var body struct {
		UserID            string `json:"userId"`
		MaxStorageBytes   int64  `json:"maxStorageBytes"`
		MaxBandwidthBytes int64  `json:"maxBandwidthBytes"`
	}
	if err := msg.Decode(&body); err != nil {
		return wafer.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	quota, err := b.quotaService.GetOrCreateQuota(context.Background(), body.UserID)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}

	quota.MaxStorageBytes = body.MaxStorageBytes
	quota.MaxBandwidthBytes = body.MaxBandwidthBytes
	quota.UpdatedAt = apptime.NowTime()

	if err := b.saveStorageQuota(quota); err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}

	return wafer.JSONRespond(msg, 200, quota)
}

func (b *FilesBlock) handleGetUserQuota(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	queryUserID := msg.Query("user_id")
	if queryUserID == "" {
		queryUserID = userID
	} else if !msg.IsAdmin() && queryUserID != userID {
		return wafer.Error(msg, 403, "forbidden", "Forbidden")
	}

	if b.quotaService == nil {
		return wafer.Error(msg, 500, "internal_error", "Quota service not initialized")
	}

	quota, err := b.quotaService.GetUserQuota(context.Background(), queryUserID)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", fmt.Sprintf("Failed to get user quota: %v", err))
	}

	return wafer.JSONRespond(msg, 200, quota)
}

// ========== Access Logs ==========

func (b *FilesBlock) handleAccessLogs(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	if b.accessLogService == nil {
		return wafer.Error(msg, 501, "not_implemented", "Access logging is not enabled")
	}

	filters := AccessLogFilters{
		UserID:   userID,
		ObjectID: msg.Query("object_id"),
		Action:   msg.Query("action"),
		Limit:    100,
	}

	if limitStr := msg.Query("limit"); limitStr != "" {
		if limit, err := strconv.Atoi(limitStr); err == nil {
			filters.Limit = limit
		}
	}
	if startStr := msg.Query("start_date"); startStr != "" {
		if start, err := apptime.ParseWithLayout(apptime.TimeFormat, startStr); err == nil {
			filters.StartDate = &start
		}
	}
	if endStr := msg.Query("end_date"); endStr != "" {
		if end, err := apptime.ParseWithLayout(apptime.TimeFormat, endStr); err == nil {
			filters.EndDate = &end
		}
	}

	logs, err := b.accessLogService.GetAccessLogs(context.Background(), filters)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}

	return wafer.JSONRespond(msg, 200, logs)
}

func (b *FilesBlock) handleAccessStats(_ wafer.Context, msg *wafer.Message) wafer.Result {
	userID := msg.UserID()
	if userID == "" {
		return wafer.Error(msg, 401, "unauthorized", "Unauthorized")
	}

	if b.accessLogService == nil {
		return wafer.Error(msg, 501, "not_implemented", "Access logging is not enabled")
	}

	filters := StatsFilters{
		UserID:   userID,
		ObjectID: msg.Query("object_id"),
	}
	if startStr := msg.Query("start_date"); startStr != "" {
		if start, err := apptime.ParseWithLayout(apptime.TimeFormat, startStr); err == nil {
			filters.StartDate = &start
		}
	}
	if endStr := msg.Query("end_date"); endStr != "" {
		if end, err := apptime.ParseWithLayout(apptime.TimeFormat, endStr); err == nil {
			filters.EndDate = &end
		}
	}

	stats, err := b.accessLogService.GetAccessStats(context.Background(), filters)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}

	return wafer.JSONRespond(msg, 200, stats)
}

// ========== Admin: Role Quotas ==========

func (b *FilesBlock) handleGetRoleQuotas(_ wafer.Context, msg *wafer.Message) wafer.Result {
	if b.db == nil {
		return wafer.JSONRespond(msg, 200, []RoleQuota{})
	}

	quotas, err := b.getAllRoleQuotas()
	if err != nil {
		return wafer.JSONRespond(msg, 200, []RoleQuota{})
	}

	return wafer.JSONRespond(msg, 200, quotas)
}

func (b *FilesBlock) handleUpdateRoleQuota(_ wafer.Context, msg *wafer.Message) wafer.Result {
	roleID := msg.Var("role")
	if roleID == "" {
		return wafer.Error(msg, 400, "bad_request", "Invalid role ID")
	}

	var update RoleQuota
	if err := msg.Decode(&update); err != nil {
		return wafer.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	if update.MaxStorageBytes < 0 || update.MaxBandwidthBytes < 0 ||
		update.MaxUploadSize < 0 || update.MaxFilesCount < 0 {
		return wafer.Error(msg, 400, "bad_request", "Invalid quota values: cannot be negative")
	}

	if update.MaxStorageBytes > 10995116277760 || update.MaxBandwidthBytes > 109951162777600 {
		return wafer.Error(msg, 400, "bad_request", "Quota values exceed maximum allowed limits")
	}

	update.RoleID = roleID
	if update.ID == "" {
		update.ID = uuid.New().String()
	}

	if err := b.upsertRoleQuota(&update); err != nil {
		return wafer.Error(msg, 500, "internal_error", "Failed to update role quota")
	}

	return wafer.JSONRespond(msg, 200, update)
}

// ========== Admin: User Overrides ==========

func (b *FilesBlock) handleGetUserOverrides(_ wafer.Context, msg *wafer.Message) wafer.Result {
	if b.db == nil {
		return wafer.JSONRespond(msg, 200, []UserQuotaOverride{})
	}

	overrides, err := b.getActiveUserQuotaOverrides()
	if err != nil {
		return wafer.JSONRespond(msg, 200, []UserQuotaOverride{})
	}

	return wafer.JSONRespond(msg, 200, overrides)
}

func (b *FilesBlock) handleCreateUserOverride(_ wafer.Context, msg *wafer.Message) wafer.Result {
	adminUserID := msg.UserID()

	var override UserQuotaOverride
	if err := msg.Decode(&override); err != nil {
		return wafer.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	if override.UserID == "" {
		return wafer.Error(msg, 400, "bad_request", "User ID is required")
	}

	if override.MaxStorageBytes != nil && *override.MaxStorageBytes < 0 {
		return wafer.Error(msg, 400, "bad_request", "Invalid storage quota: cannot be negative")
	}
	if override.MaxBandwidthBytes != nil && *override.MaxBandwidthBytes < 0 {
		return wafer.Error(msg, 400, "bad_request", "Invalid bandwidth quota: cannot be negative")
	}
	if override.MaxUploadSize != nil && *override.MaxUploadSize < 0 {
		return wafer.Error(msg, 400, "bad_request", "Invalid upload size: cannot be negative")
	}
	if override.MaxFilesCount != nil && *override.MaxFilesCount < 0 {
		return wafer.Error(msg, 400, "bad_request", "Invalid file count: cannot be negative")
	}

	override.CreatedBy = adminUserID
	override.ID = uuid.New().String()
	override.CreatedAt = apptime.NowTime()
	override.UpdatedAt = apptime.NowTime()

	if err := b.createUserQuotaOverride(&override); err != nil {
		return wafer.Error(msg, 500, "internal_error", "Failed to create user override")
	}

	return wafer.JSONRespond(msg, 201, override)
}

func (b *FilesBlock) handleDeleteUserOverride(_ wafer.Context, msg *wafer.Message) wafer.Result {
	overrideID := msg.Var("user")
	if overrideID == "" {
		return wafer.Error(msg, 400, "bad_request", "Invalid override ID")
	}

	if err := b.deleteUserQuotaOverrideByID(overrideID); err != nil {
		return wafer.Error(msg, 500, "internal_error", "Failed to delete user override")
	}

	return wafer.Respond(msg, 204, nil, "")
}

// ========== Admin: User Search ==========

func (b *FilesBlock) handleUserSearch(_ wafer.Context, msg *wafer.Message) wafer.Result {
	query := msg.Query("q")
	if query == "" || len(query) < 2 {
		return wafer.JSONRespond(msg, 200, []any{})
	}

	users, err := b.searchUsers(query)
	if err != nil {
		return wafer.JSONRespond(msg, 200, []any{})
	}

	return wafer.JSONRespond(msg, 200, users)
}

// ========== Admin: Default Quotas ==========

func (b *FilesBlock) handleDefaultQuotasGet(_ wafer.Context, msg *wafer.Message) wafer.Result {
	if b.quotaService == nil {
		return wafer.Error(msg, 503, "unavailable", "Quota service not available")
	}

	return wafer.JSONRespond(msg, 200, map[string]any{
		"defaultStorage":   b.cloudConfig.DefaultStorageLimit,
		"defaultBandwidth": b.cloudConfig.DefaultBandwidthLimit,
	})
}

func (b *FilesBlock) handleDefaultQuotasPut(_ wafer.Context, msg *wafer.Message) wafer.Result {
	if b.quotaService == nil {
		return wafer.Error(msg, 503, "unavailable", "Quota service not available")
	}

	var body struct {
		DefaultStorage   int64 `json:"defaultStorage"`
		DefaultBandwidth int64 `json:"defaultBandwidth"`
	}
	if err := msg.Decode(&body); err != nil {
		return wafer.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	b.cloudConfig.DefaultStorageLimit = body.DefaultStorage
	b.cloudConfig.DefaultBandwidthLimit = body.DefaultBandwidth

	return wafer.JSONRespond(msg, 200, map[string]string{
		"message": "Default quotas updated successfully",
	})
}

// ========== Helpers ==========

func (b *FilesBlock) handleSharedWithMe(msg *wafer.Message, userID string) wafer.Result {
	if b.shareService == nil {
		return wafer.JSONRespond(msg, 200, map[string]any{
			"files":   []any{},
			"folders": []any{},
		})
	}

	shares, err := b.shareService.GetSharedWithMe(context.Background(), userID)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}

	files := []StorageShareWithObject{}
	folders := []StorageShareWithObject{}
	for _, share := range shares {
		if share.ContentType == "application/x-directory" {
			folders = append(folders, share)
		} else {
			files = append(files, share)
		}
	}

	return wafer.JSONRespond(msg, 200, map[string]any{
		"files":   files,
		"folders": folders,
	})
}

func (b *FilesBlock) handleSharedByMe(msg *wafer.Message, userID string) wafer.Result {
	if b.shareService == nil {
		return wafer.JSONRespond(msg, 200, []any{})
	}

	shares, err := b.shareService.GetSharedByMe(context.Background(), userID)
	if err != nil {
		return wafer.Error(msg, 500, "internal_error", err.Error())
	}

	return wafer.JSONRespond(msg, 200, shares)
}
