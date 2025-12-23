package cloudstorage

import (
	"context"
	"fmt"

	"github.com/suppers-ai/solobase/extensions/core"
	"github.com/suppers-ai/solobase/internal/iam"
)

// CheckUploadPermission checks if a user can upload based on IAM policies and CloudStorage quotas
func (e *CloudStorageExtension) CheckUploadPermission(ctx context.Context, userID string, fileSize int64, iamService *iam.Service) error {
	// Get user's effective metadata for access control only
	metadata, err := iamService.GetUserEffectiveMetadata(ctx, userID)
	if err != nil {
		return fmt.Errorf("failed to get user metadata: %w", err)
	}

	// Check if uploads are disabled for this user (access control)
	for _, feature := range metadata.DisabledFeatures {
		if feature == "uploads" || feature == "storage_upload" {
			return fmt.Errorf("uploads are disabled for your account")
		}
	}

	// Check storage quota using CloudStorage's own quota system
	if e.quotaService != nil {
		// Check quota (including file size limits managed by CloudStorage)
		if err := e.quotaService.CheckStorageQuota(ctx, userID, fileSize); err != nil {
			return err
		}
	}

	return nil
}

// EnhancedUploadHook checks upload permission with IAM integration
func (e *CloudStorageExtension) EnhancedUploadHook(ctx context.Context, hookCtx *core.HookContext, iamService *iam.Service) error {
	// Extract user ID and file size from hook context
	userID, ok := hookCtx.Data["userID"].(string)
	if !ok || userID == "" {
		return nil // Skip for anonymous uploads
	}

	fileSize, ok := hookCtx.Data["fileSize"].(int64)
	if !ok {
		// Try to get from content length
		if contentLength, ok := hookCtx.Data["contentLength"].(int64); ok {
			fileSize = contentLength
		} else {
			return nil // Can't determine file size
		}
	}

	// Check upload permission with IAM
	if err := e.CheckUploadPermission(ctx, userID, fileSize, iamService); err != nil {
		return fmt.Errorf("upload not allowed: %w", err)
	}

	return nil
}
