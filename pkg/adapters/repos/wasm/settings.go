//go:build wasm

package wasm

import (
	"context"

	"github.com/suppers-ai/solobase/internal/data/models"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

type settingsRepository struct{}

// Read

func (r *settingsRepository) GetByID(ctx context.Context, id string) (*models.Setting, error) {
	return nil, ErrNotImplemented
}

func (r *settingsRepository) GetByKey(ctx context.Context, key string) (*models.Setting, error) {
	return nil, ErrNotImplemented
}

func (r *settingsRepository) List(ctx context.Context) ([]*models.Setting, error) {
	return nil, ErrNotImplemented
}

func (r *settingsRepository) ListByType(ctx context.Context, settingType string) ([]*models.Setting, error) {
	return nil, ErrNotImplemented
}

// Write

func (r *settingsRepository) Create(ctx context.Context, setting *models.Setting) (*models.Setting, error) {
	return nil, ErrNotImplemented
}

func (r *settingsRepository) Update(ctx context.Context, setting *models.Setting) error {
	return ErrNotImplemented
}

func (r *settingsRepository) UpdateByKey(ctx context.Context, key string, value *string, settingType *string) error {
	return ErrNotImplemented
}

func (r *settingsRepository) Upsert(ctx context.Context, setting *models.Setting) error {
	return ErrNotImplemented
}

// Delete

func (r *settingsRepository) SoftDelete(ctx context.Context, id string) error {
	return ErrNotImplemented
}

func (r *settingsRepository) SoftDeleteByKey(ctx context.Context, key string) error {
	return ErrNotImplemented
}

func (r *settingsRepository) HardDelete(ctx context.Context, id string) error {
	return ErrNotImplemented
}

// Ensure settingsRepository implements SettingsRepository
var _ repos.SettingsRepository = (*settingsRepository)(nil)
