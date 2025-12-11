package cloudstorage

import (
	"context"
	"fmt"
	"strconv"

	"github.com/suppers-ai/solobase/extensions/core"
	"github.com/suppers-ai/solobase/internal/iam"
)

// UpdateQuotaFromIAM is deprecated - quotas are now managed directly by CloudStorage extension
// Kept for backward compatibility but does nothing
func (q *QuotaService) UpdateQuotaFromIAM(ctx context.Context, userID string, iamService *iam.Service) error {
	// Quotas are now managed directly by CloudStorage extension, not from IAM metadata
	// This function is kept for backward compatibility but does nothing
	return nil
}

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

// GetQuotaFromPolicies gets quota values from Casbin policies
func GetQuotaFromPolicies(iamService *iam.Service, userID string) (storageQuota int64, bandwidthQuota int64, err error) {
	// Get user's roles
	roles, err := iamService.GetUserRoles(context.Background(), userID)
	if err != nil {
		return 0, 0, fmt.Errorf("failed to get user roles: %w", err)
	}

	// Get quota policies for each role and take the maximum
	for _, role := range roles {
		// Get storage quota policy
		policies, _ := iamService.GetEnforcer().GetFilteredPolicy(0, role, "storage", "quota")
		for _, policy := range policies {
			if len(policy) >= 4 {
				if quota, err := strconv.ParseInt(policy[3], 10, 64); err == nil && quota > storageQuota {
					storageQuota = quota
				}
			}
		}

		// Get bandwidth quota policy
		policies, _ = iamService.GetEnforcer().GetFilteredPolicy(0, role, "bandwidth", "quota")
		for _, policy := range policies {
			if len(policy) >= 4 {
				if quota, err := strconv.ParseInt(policy[3], 10, 64); err == nil && quota > bandwidthQuota {
					bandwidthQuota = quota
				}
			}
		}
	}

	// Default quotas if none specified
	if storageQuota == 0 {
		storageQuota = 1073741824 // 1GB default
	}
	if bandwidthQuota == 0 {
		bandwidthQuota = 10737418240 // 10GB default
	}

	return storageQuota, bandwidthQuota, nil
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
