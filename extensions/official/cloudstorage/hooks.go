package cloudstorage

import (
	"context"
	"fmt"
	"log"

	"github.com/google/uuid"
	"github.com/suppers-ai/solobase/extensions/core"
	pkgstorage "github.com/suppers-ai/solobase/internal/pkg/storage"
	"gorm.io/gorm"
)

// checkStorageQuotaHook checks if user has enough storage quota before upload
func (e *CloudStorageExtension) checkStorageQuotaHook(ctx context.Context, hookCtx *core.HookContext) error {
	if e.db == nil || e.quotaService == nil {
		return nil // Skip if not properly initialized
	}

	// Extract user ID and file size from hook context
	userID, ok := hookCtx.Data["userID"].(string)
	if !ok || userID == "" {
		return nil // Skip for anonymous uploads
	}

	fileSize, ok := hookCtx.Data["fileSize"].(int64)
	if !ok || fileSize == 0 {
		return nil // Skip if no size info
	}

	// Get or create quota for user
	quota, err := e.quotaService.GetOrCreateQuota(ctx, userID)
	if err != nil {
		log.Printf("Failed to get quota for user %s: %v", userID, err)
		return nil // Don't block upload on quota check failure
	}

	// Check if user has enough space
	if quota.MaxStorageBytes > 0 && quota.StorageUsed+fileSize > quota.MaxStorageBytes {
		available := quota.MaxStorageBytes - quota.StorageUsed
		return fmt.Errorf("storage quota exceeded: %d bytes available", available)
	}

	return nil
}

// updateStorageUsageHook updates storage usage after successful upload
func (e *CloudStorageExtension) updateStorageUsageHook(ctx context.Context, hookCtx *core.HookContext) error {
	if e.db == nil || e.quotaService == nil {
		return nil
	}

	userID, ok := hookCtx.Data["userID"].(string)
	if !ok || userID == "" {
		return nil
	}

	fileSize, ok := hookCtx.Data["fileSize"].(int64)
	if !ok || fileSize == 0 {
		return nil
	}

	// Update storage usage asynchronously
	go func() {
		// Ensure quota exists for user
		_, err := e.quotaService.GetOrCreateQuota(context.Background(), userID)
		if err != nil {
			log.Printf("Failed to get/create quota for user %s: %v", userID, err)
			return
		}

		// Update storage usage
		if err := e.quotaService.UpdateStorageUsage(context.Background(), userID, fileSize); err != nil {
			log.Printf("Failed to update storage usage for user %s: %v", userID, err)
		}
	}()

	return nil
}

// updateBandwidthUsageHook updates bandwidth usage after download
func (e *CloudStorageExtension) updateBandwidthUsageHook(ctx context.Context, hookCtx *core.HookContext) error {
	if e.db == nil || e.quotaService == nil {
		return nil
	}

	userID, ok := hookCtx.Data["userID"].(string)
	if !ok || userID == "" {
		return nil
	}

	bytesRead, ok := hookCtx.Data["bytesRead"].(int64)
	if !ok || bytesRead == 0 {
		return nil
	}

	// Update bandwidth usage asynchronously
	go func() {
		// Ensure quota exists for user
		_, err := e.quotaService.GetOrCreateQuota(context.Background(), userID)
		if err != nil {
			log.Printf("Failed to get/create quota for user %s: %v", userID, err)
			return
		}

		// Update bandwidth usage
		if err := e.quotaService.UpdateBandwidthUsage(context.Background(), userID, bytesRead); err != nil {
			log.Printf("Failed to update bandwidth usage for user %s: %v", userID, err)
		}
	}()

	return nil
}

// logUploadAccessHook logs upload access
func (e *CloudStorageExtension) logUploadAccessHook(ctx context.Context, hookCtx *core.HookContext) error {
	if e.db == nil || e.accessLogService == nil {
		return nil
	}

	// Extract needed data
	objectID, _ := hookCtx.Data["objectID"].(string)
	userID, _ := hookCtx.Data["userID"].(string)
	// bucket, _ := hookCtx.Data["bucket"].(string)  // Reserved for future use
	// filename, _ := hookCtx.Data["filename"].(string)  // Reserved for future use

	// Log asynchronously
	go func() {
		var userIDPtr *string
		if userID != "" {
			userIDPtr = &userID
		}

		accessLog := &StorageAccessLog{
			ID:       uuid.New().String(),
			ObjectID: objectID,
			UserID:   userIDPtr,
			Action:   "upload",
		}

		if err := e.db.Create(accessLog).Error; err != nil {
			log.Printf("Failed to log upload access: %v", err)
		}
	}()

	return nil
}

// logDownloadAccessHook logs download access
func (e *CloudStorageExtension) logDownloadAccessHook(ctx context.Context, hookCtx *core.HookContext) error {
	if e.db == nil || e.accessLogService == nil {
		return nil
	}

	// Extract needed data
	objectID, _ := hookCtx.Data["objectID"].(string)
	userID, _ := hookCtx.Data["userID"].(string)
	// bucket, _ := hookCtx.Data["bucket"].(string)  // Reserved for future use
	// bytesRead, _ := hookCtx.Data["bytesRead"].(int64)  // Reserved for future use

	// Log asynchronously
	go func() {
		var userIDPtr *string
		if userID != "" {
			userIDPtr = &userID
		}

		accessLog := &StorageAccessLog{
			ID:       uuid.New().String(),
			ObjectID: objectID,
			UserID:   userIDPtr,
			Action:   "download",
		}

		if err := e.db.Create(accessLog).Error; err != nil {
			log.Printf("Failed to log download access: %v", err)
		}
	}()

	return nil
}

// setupUserResourcesHook creates the user's "My Files" folder on login
func (e *CloudStorageExtension) setupUserResourcesHook(ctx context.Context, hookCtx *core.HookContext) error {
	// Extract user data
	userID, ok := hookCtx.Data["userID"].(string)
	if !ok || userID == "" {
		log.Printf("setupUserResourcesHook: No userID found in context, skipping")
		return nil // Skip if no user ID
	}

	// Get app ID from context (defaults to "solobase" if not set)
	appID := "solobase"
	if id, ok := hookCtx.Data["appID"].(string); ok && id != "" {
		appID = id
	}

	log.Printf("setupUserResourcesHook: Starting for userID=%s, appID=%s", userID, appID)

	// Ensure database is available
	if e.db == nil {
		log.Printf("ERROR: setupUserResourcesHook: Database is nil, cannot create My Files folder")
		return fmt.Errorf("database not available")
	}

	// Check if user already has a "My Files" folder (root folder with no parent) for this app
	var existingFolder pkgstorage.StorageObject
	err := e.db.Where("bucket_name = ? AND user_id = ? AND app_id = ? AND object_name = ? AND content_type = ? AND parent_folder_id IS NULL",
		"int_storage", userID, appID, "My Files", "application/x-directory").
		First(&existingFolder).Error

	if err != nil && err != gorm.ErrRecordNotFound {
		// Log any unexpected error
		log.Printf("WARNING: Error checking for existing My Files folder: %v", err)
	}

	if err == gorm.ErrRecordNotFound || err == nil && existingFolder.ID == "" {
		// Folder doesn't exist, create it
		// Note: We check both gorm.ErrRecordNotFound and empty ID for extra safety

		// Create the "My Files" folder
		myFilesFolder := &pkgstorage.StorageObject{
			ID:          uuid.New().String(),
			BucketName:  "int_storage",
			ObjectName:  "My Files",
			UserID:      userID,
			AppID:       &appID,
			ContentType: "application/x-directory",
			Size:        0,
		}

		log.Printf("Creating My Files folder with ID=%s, appID=%s for user %s", myFilesFolder.ID, appID, userID)

		// Try to create the folder
		if createErr := e.db.Create(myFilesFolder).Error; createErr != nil {
			// Check if the error is due to a duplicate (might have been created concurrently)
			var checkAgain pkgstorage.StorageObject
			checkErr := e.db.Where("bucket_name = ? AND user_id = ? AND app_id = ? AND object_name = ? AND content_type = ? AND parent_folder_id IS NULL",
				"int_storage", userID, appID, "My Files", "application/x-directory").
				First(&checkAgain).Error

			if checkErr == nil {
				// Folder was created concurrently, that's fine
				log.Printf("My Files folder was created concurrently for user %s with ID %s", userID, checkAgain.ID)

				// Store folder ID in context
				if hookCtx.Data == nil {
					hookCtx.Data = make(map[string]interface{})
				}
				hookCtx.Data["myFilesFolderID"] = checkAgain.ID
				return nil
			}

			// Real error, log it with more detail
			log.Printf("ERROR: Failed to create My Files folder for user %s: %v", userID, createErr)
			return fmt.Errorf("failed to create My Files folder: %w", createErr)
		}

		log.Printf("Successfully created My Files folder for user %s with ID %s and appID=%s", userID, myFilesFolder.ID, appID)

		// Store folder ID in context
		if hookCtx.Data == nil {
			hookCtx.Data = make(map[string]interface{})
		}
		hookCtx.Data["myFilesFolderID"] = myFilesFolder.ID
	} else if err == nil && existingFolder.ID != "" {
		log.Printf("User %s already has My Files folder with ID %s", userID, existingFolder.ID)

		// Store existing folder ID in context
		if hookCtx.Data == nil {
			hookCtx.Data = make(map[string]interface{})
		}
		hookCtx.Data["myFilesFolderID"] = existingFolder.ID
	}

	return nil
}

// Helper methods for quota service
