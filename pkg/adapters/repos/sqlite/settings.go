//go:build !wasm

package sqlite

import (
	"context"
	"database/sql"

	"github.com/suppers-ai/solobase/internal/data/models"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	db "github.com/suppers-ai/solobase/internal/sqlc/gen"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

type settingsRepository struct {
	sqlDB   *sql.DB
	queries *db.Queries
}

// NewSettingsRepository creates a new SQLite settings repository
func NewSettingsRepository(sqlDB *sql.DB, queries *db.Queries) repos.SettingsRepository {
	return &settingsRepository{
		sqlDB:   sqlDB,
		queries: queries,
	}
}

func (r *settingsRepository) GetByID(ctx context.Context, id string) (*models.Setting, error) {
	dbSetting, err := r.queries.GetSettingByID(ctx, id)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBSettingToModel(dbSetting), nil
}

func (r *settingsRepository) GetByKey(ctx context.Context, key string) (*models.Setting, error) {
	dbSetting, err := r.queries.GetSettingByKey(ctx, key)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, repos.ErrNotFound
		}
		return nil, err
	}
	return convertDBSettingToModel(dbSetting), nil
}

func (r *settingsRepository) List(ctx context.Context) ([]*models.Setting, error) {
	dbSettings, err := r.queries.ListSettings(ctx)
	if err != nil {
		return nil, err
	}
	settings := make([]*models.Setting, len(dbSettings))
	for i, s := range dbSettings {
		settings[i] = convertDBSettingToModel(s)
	}
	return settings, nil
}

func (r *settingsRepository) ListByType(ctx context.Context, settingType string) ([]*models.Setting, error) {
	dbSettings, err := r.queries.ListSettingsByType(ctx, &settingType)
	if err != nil {
		return nil, err
	}
	settings := make([]*models.Setting, len(dbSettings))
	for i, s := range dbSettings {
		settings[i] = convertDBSettingToModel(s)
	}
	return settings, nil
}

func (r *settingsRepository) Create(ctx context.Context, setting *models.Setting) (*models.Setting, error) {
	now := apptime.NowString()
	if setting.ID == uuid.Nil {
		setting.ID = uuid.New()
	}

	value := setting.Value
	settingType := setting.Type

	dbSetting, err := r.queries.CreateSetting(ctx, db.CreateSettingParams{
		ID:        setting.ID.String(),
		Key:       setting.Key,
		Value:     &value,
		Type:      &settingType,
		CreatedAt: now,
		UpdatedAt: now,
	})
	if err != nil {
		return nil, err
	}
	return convertDBSettingToModel(dbSetting), nil
}

func (r *settingsRepository) Update(ctx context.Context, setting *models.Setting) error {
	now := apptime.NowString()
	value := setting.Value
	settingType := setting.Type

	return r.queries.UpdateSetting(ctx, db.UpdateSettingParams{
		ID:        setting.ID.String(),
		Value:     &value,
		Type:      &settingType,
		UpdatedAt: now,
	})
}

func (r *settingsRepository) UpdateByKey(ctx context.Context, key string, value *string, settingType *string) error {
	now := apptime.NowString()
	return r.queries.UpdateSettingByKey(ctx, db.UpdateSettingByKeyParams{
		Key:       key,
		Value:     value,
		Type:      settingType,
		UpdatedAt: now,
	})
}

func (r *settingsRepository) Upsert(ctx context.Context, setting *models.Setting) error {
	now := apptime.NowString()
	if setting.ID == uuid.Nil {
		setting.ID = uuid.New()
	}

	value := setting.Value
	settingType := setting.Type

	return r.queries.UpsertSetting(ctx, db.UpsertSettingParams{
		ID:        setting.ID.String(),
		Key:       setting.Key,
		Value:     &value,
		Type:      &settingType,
		CreatedAt: now,
		UpdatedAt: now,
	})
}

func (r *settingsRepository) SoftDelete(ctx context.Context, id string) error {
	now := apptime.NowString()
	return r.queries.SoftDeleteSetting(ctx, db.SoftDeleteSettingParams{
		ID:        id,
		DeletedAt: apptime.NewNullTimeNow(),
		UpdatedAt: now,
	})
}

func (r *settingsRepository) SoftDeleteByKey(ctx context.Context, key string) error {
	now := apptime.NowString()
	return r.queries.SoftDeleteSettingByKey(ctx, db.SoftDeleteSettingByKeyParams{
		Key:       key,
		DeletedAt: apptime.NewNullTimeNow(),
		UpdatedAt: now,
	})
}

func (r *settingsRepository) HardDelete(ctx context.Context, id string) error {
	return r.queries.HardDeleteSetting(ctx, id)
}

// Conversion helpers

func convertDBSettingToModel(dbSetting db.SysSetting) *models.Setting {
	var value, settingType string
	if dbSetting.Value != nil {
		value = *dbSetting.Value
	}
	if dbSetting.Type != nil {
		settingType = *dbSetting.Type
	}

	return &models.Setting{
		ID:        uuid.MustParse(dbSetting.ID),
		Key:       dbSetting.Key,
		Value:     value,
		Type:      settingType,
		CreatedAt: apptime.MustParse(dbSetting.CreatedAt),
		UpdatedAt: apptime.MustParse(dbSetting.UpdatedAt),
		DeletedAt: dbSetting.DeletedAt,
	}
}

// Ensure settingsRepository implements SettingsRepository
var _ repos.SettingsRepository = (*settingsRepository)(nil)
