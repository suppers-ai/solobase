package files

import (
	"context"
	"errors"
	"fmt"
	"log"

	"github.com/suppers-ai/solobase/core/apptime"
	"github.com/suppers-ai/solobase/core/uuid"
	"github.com/suppers-ai/waffle-go/services/database"
)

// beforeUpload checks if user has enough storage quota before upload
func (b *FilesBlock) beforeUpload(ctx context.Context, userID, bucket, filename string, fileSize int64) error {
	if b.db == nil || b.quotaService == nil {
		return nil
	}

	if userID == "" {
		return nil
	}

	if fileSize == 0 {
		return nil
	}

	quota, err := b.quotaService.GetOrCreateQuota(ctx, userID)
	if err != nil {
		log.Printf("Failed to get quota for user %s: %v", userID, err)
		return nil
	}

	if quota.MaxStorageBytes > 0 && quota.StorageUsed+fileSize > quota.MaxStorageBytes {
		available := quota.MaxStorageBytes - quota.StorageUsed
		return fmt.Errorf("storage quota exceeded: %d bytes available", available)
	}

	return nil
}

// afterUpload updates storage usage and logs access after successful upload
func (b *FilesBlock) afterUpload(ctx context.Context, userID, bucket, objectID, filename string, fileSize int64) {
	if b.db == nil {
		return
	}

	if b.quotaService != nil && userID != "" && fileSize > 0 {
		_, err := b.quotaService.GetOrCreateQuota(ctx, userID)
		if err != nil {
			log.Printf("Failed to get/create quota for user %s: %v", userID, err)
		} else {
			if err := b.quotaService.UpdateStorageUsage(ctx, userID, fileSize); err != nil {
				log.Printf("Failed to update storage usage for user %s: %v", userID, err)
			}
		}
	}

	if b.accessLogService != nil && b.cloudConfig.EnableAccessLogs {
		var userIDPtr *string
		if userID != "" {
			userIDPtr = &userID
		}

		accessLog := &StorageAccessLog{
			ID:        uuid.New().String(),
			ObjectID:  objectID,
			UserID:    userIDPtr,
			Action:    "upload",
			CreatedAt: apptime.NowTime(),
		}

		if err := b.createAccessLog(accessLog); err != nil {
			log.Printf("Failed to log upload access: %v", err)
		}
	}
}

// beforeDownload checks share permissions before downloads
func (b *FilesBlock) beforeDownload(ctx context.Context, userID, bucket, objectID string) error {
	if b.shareService == nil {
		return nil
	}

	if objectID == "" {
		return nil
	}

	obj, err := b.getStorageObjectByID(objectID)
	if err != nil {
		return nil
	}

	if userID != "" && obj.UserID == userID {
		return nil
	}

	var directShare *StorageShare
	if userID != "" {
		directShare, err = b.getShareByObjectAndUser(objectID, userID)
	} else {
		directShare, err = b.getShareByObjectPublicOnly(objectID)
	}

	if err == nil && directShare != nil {
		return nil
	}

	var userEmail string
	if userID != "" {
		userEmail, _ = b.getUserEmail(userID)
	}

	inheritedShare, err := b.shareService.CheckInheritedPermissions(ctx, objectID, userID, userEmail)
	if err == nil && inheritedShare != nil {
		return nil
	}

	if userID == "" {
		return fmt.Errorf("authentication required to access this file")
	}
	return fmt.Errorf("access denied: you don't have permission to access this file")
}

// afterDownload updates bandwidth usage and logs access after download
func (b *FilesBlock) afterDownload(ctx context.Context, userID, bucket, objectID string, bytesRead int64) {
	if b.db == nil {
		return
	}

	if b.quotaService != nil && userID != "" && bytesRead > 0 {
		_, err := b.quotaService.GetOrCreateQuota(ctx, userID)
		if err != nil {
			log.Printf("Failed to get/create quota for user %s: %v", userID, err)
		} else {
			if err := b.quotaService.UpdateBandwidthUsage(ctx, userID, bytesRead); err != nil {
				log.Printf("Failed to update bandwidth usage for user %s: %v", userID, err)
			}
		}
	}

	if b.accessLogService != nil && b.cloudConfig.EnableAccessLogs {
		var userIDPtr *string
		if userID != "" {
			userIDPtr = &userID
		}

		accessLog := &StorageAccessLog{
			ID:        uuid.New().String(),
			ObjectID:  objectID,
			UserID:    userIDPtr,
			Action:    "download",
			CreatedAt: apptime.NowTime(),
		}

		if err := b.createAccessLog(accessLog); err != nil {
			log.Printf("Failed to log download access: %v", err)
		}
	}
}

// setupUserResources creates the user's "My Files" folder on login
func (b *FilesBlock) setupUserResources(ctx context.Context, userID, appID string) error {
	if userID == "" || b.db == nil {
		return nil
	}

	if appID == "" {
		appID = "solobase"
	}

	existingFolder, err := b.getMyFilesFolder(userID, appID)

	if err != nil && !errors.Is(err, database.ErrNotFound) {
		log.Printf("WARNING: Error checking for existing My Files folder: %v", err)
	}

	if errors.Is(err, database.ErrNotFound) || (err == nil && existingFolder.ID == "") {
		myFilesFolder := &storageObject{
			ID:          uuid.New().String(),
			BucketName:  "int_storage",
			ObjectName:  "My Files",
			UserID:      userID,
			AppID:       &appID,
			ContentType: "application/x-directory",
			Size:        0,
		}

		if createErr := b.createMyFilesFolder(myFilesFolder); createErr != nil {
			checkAgain, checkErr := b.getMyFilesFolder(userID, appID)
			if checkErr == nil && checkAgain != nil {
				return nil
			}
			return fmt.Errorf("failed to create My Files folder: %w", createErr)
		}
	}

	return nil
}

// CheckUploadPermission checks if a user can upload based on quotas.
func (b *FilesBlock) CheckUploadPermission(ctx context.Context, userID string, fileSize int64) error {
	if b.quotaService != nil {
		if err := b.quotaService.CheckStorageQuota(ctx, userID, fileSize); err != nil {
			return err
		}
	}
	return nil
}
