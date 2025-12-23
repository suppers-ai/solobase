package repos

import (
	"context"

	"github.com/suppers-ai/solobase/internal/data/models"
)

// SettingsRepository provides settings operations
type SettingsRepository interface {
	// Read
	GetByID(ctx context.Context, id string) (*models.Setting, error)
	GetByKey(ctx context.Context, key string) (*models.Setting, error)
	List(ctx context.Context) ([]*models.Setting, error)
	ListByType(ctx context.Context, settingType string) ([]*models.Setting, error)

	// Write
	Create(ctx context.Context, setting *models.Setting) (*models.Setting, error)
	Update(ctx context.Context, setting *models.Setting) error
	UpdateByKey(ctx context.Context, key string, value *string, settingType *string) error
	Upsert(ctx context.Context, setting *models.Setting) error

	// Delete
	SoftDelete(ctx context.Context, id string) error
	SoftDeleteByKey(ctx context.Context, key string) error
	HardDelete(ctx context.Context, id string) error
}
