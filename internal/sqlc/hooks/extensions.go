package hooks

import (
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
)

// LegalDocumentBeforeCreate prepares a legal document for creation
func LegalDocumentBeforeCreate(params *db.CreateLegalDocumentParams) error {
	if params.ID == "" {
		params.ID = uuid.New().String()
	}
	// Timestamps are handled by database defaults (CURRENT_TIMESTAMP)
	return nil
}

// StorageShareBeforeCreate prepares a storage share for creation
func StorageShareBeforeCreate(params *db.CreateStorageShareParams) error {
	if params.ID == "" {
		params.ID = uuid.New().String()
	}
	now := apptime.NowString()
	params.CreatedAt = now
	params.UpdatedAt = now
	return nil
}

// StorageAccessLogBeforeCreate prepares a storage access log for creation
func StorageAccessLogBeforeCreate(params *db.CreateStorageAccessLogParams) error {
	if params.ID == "" {
		params.ID = uuid.New().String()
	}
	params.CreatedAt = apptime.NowString()
	return nil
}

// StorageQuotaBeforeCreate prepares a storage quota for creation
func StorageQuotaBeforeCreate(params *db.CreateStorageQuotaParams) error {
	if params.ID == "" {
		params.ID = uuid.New().String()
	}
	now := apptime.NowString()
	params.CreatedAt = now
	params.UpdatedAt = now
	return nil
}

// RoleQuotaBeforeCreate prepares a role quota for creation
func RoleQuotaBeforeCreate(params *db.CreateRoleQuotaParams) error {
	if params.ID == "" {
		params.ID = uuid.New().String()
	}
	now := apptime.NowString()
	params.CreatedAt = now
	params.UpdatedAt = now
	return nil
}

// UserQuotaOverrideBeforeCreate prepares a user quota override for creation
func UserQuotaOverrideBeforeCreate(params *db.CreateUserQuotaOverrideParams) error {
	if params.ID == "" {
		params.ID = uuid.New().String()
	}
	now := apptime.NowString()
	params.CreatedAt = now
	params.UpdatedAt = now
	return nil
}
