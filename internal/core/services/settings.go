package services

import (
	"context"
	"encoding/json"
	"fmt"
	"strconv"

	"github.com/suppers-ai/solobase/internal/data/models"
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
	"github.com/suppers-ai/solobase/pkg/adapters/repos"
)

type SettingsService struct {
	repo repos.SettingsRepository
}

func NewSettingsService(repo repos.SettingsRepository) *SettingsService {
	service := &SettingsService{repo: repo}
	// Initialize default settings on first run
	service.initializeDefaults(context.Background())
	// Initialize extension-specific settings
	service.initializeExtensionSettings(context.Background())
	return service
}

// initializeExtensionSettings creates extension-specific default settings
func (s *SettingsService) initializeExtensionSettings(ctx context.Context) error {
	// Initialize CloudStorage extension setting for showing usage in profile
	// This is set to true by default when CloudStorage is available
	err := s.setSetting(ctx, "ext_cloudstorage_profile_show_usage", true)
	if err != nil {
		// Log but don't fail - setting might already exist
		return nil
	}
	return nil
}

// initializeDefaults creates default settings if they don't exist
func (s *SettingsService) initializeDefaults(ctx context.Context) error {
	defaults := models.DefaultSettings()

	// Check if any settings exist
	settings, err := s.repo.List(ctx)
	if err != nil {
		return err
	}
	if len(settings) > 0 {
		return nil // Settings already initialized
	}

	// Convert defaults to individual settings
	settingsMap := map[string]interface{}{
		"app_name":                   defaults.AppName,
		"app_url":                    defaults.AppURL,
		"allow_signup":               defaults.AllowSignup,
		"require_email_confirmation": defaults.RequireEmailConfirmation,
		"mailer_provider":            defaults.MailerProvider,
		"mailgun_domain":             defaults.MailgunDomain,
		"mailgun_region":             defaults.MailgunRegion,
		"storage_provider":           defaults.StorageProvider,
		"s3_bucket":                  defaults.S3Bucket,
		"s3_region":                  defaults.S3Region,
		"max_upload_size":            defaults.MaxUploadSize,
		"allowed_file_types":         defaults.AllowedFileTypes,
		"session_timeout":            defaults.SessionTimeout,
		"password_min_length":        defaults.PasswordMinLength,
		"enable_api_logs":            defaults.EnableAPILogs,
		"enable_debug_mode":          defaults.EnableDebugMode,
		"maintenance_mode":           defaults.MaintenanceMode,
		"maintenance_message":        defaults.MaintenanceMessage,
		"notification":               defaults.Notification,
	}

	// Save each setting
	for key, value := range settingsMap {
		if err := s.setSetting(ctx, key, value); err != nil {
			return err
		}
	}

	return nil
}

// GetSettings retrieves all settings as AppSettings struct
func (s *SettingsService) GetSettings() (*models.AppSettings, error) {
	ctx := context.Background()
	settings, err := s.repo.List(ctx)
	if err != nil {
		return nil, err
	}

	// Convert to AppSettings struct
	appSettings := models.DefaultSettings()
	for _, setting := range settings {
		if err := s.applySetting(appSettings, setting); err != nil {
			// Log error but continue processing other settings
			fmt.Printf("Error applying setting %s: %v\n", setting.Key, err)
		}
	}

	return appSettings, nil
}

// UpdateSettings updates multiple settings at once
func (s *SettingsService) UpdateSettings(updates map[string]interface{}) (*models.AppSettings, error) {
	ctx := context.Background()
	// Validate and update each setting
	for key, value := range updates {
		if err := s.setSetting(ctx, key, value); err != nil {
			return nil, fmt.Errorf("failed to update setting %s: %w", key, err)
		}
	}

	// Return updated settings
	return s.GetSettings()
}

// GetSetting retrieves a single setting value
func (s *SettingsService) GetSetting(key string) (interface{}, error) {
	ctx := context.Background()
	setting, err := s.repo.GetByKey(ctx, key)
	if err != nil {
		if err == repos.ErrNotFound {
			return nil, fmt.Errorf("setting not found: %s", key)
		}
		return nil, err
	}

	return s.parseValue(setting.Value, setting.Type)
}

// SetSetting updates or creates a single setting (public method)
func (s *SettingsService) SetSetting(key string, value interface{}) error {
	return s.setSetting(context.Background(), key, value)
}

// setSetting updates or creates a single setting
func (s *SettingsService) setSetting(ctx context.Context, key string, value interface{}) error {
	// Determine type and convert value to string
	valueStr, valueType := s.serializeValue(value)

	now := apptime.NowTime()
	setting := &models.Setting{
		ID:        uuid.New(),
		Key:       key,
		Value:     valueStr,
		Type:      valueType,
		CreatedAt: now,
		UpdatedAt: now,
	}
	return s.repo.Upsert(ctx, setting)
}

// serializeValue converts a value to string and determines its type
func (s *SettingsService) serializeValue(value interface{}) (string, string) {
	switch v := value.(type) {
	case bool:
		return strconv.FormatBool(v), "bool"
	case int:
		return strconv.Itoa(v), "int"
	case int64:
		return strconv.FormatInt(v, 10), "int"
	case float64:
		return strconv.FormatFloat(v, 'f', -1, 64), "float"
	case string:
		return v, "string"
	case map[string]interface{}, []interface{}:
		data, _ := json.Marshal(v)
		return string(data), "json"
	default:
		// Try JSON serialization for complex types
		if data, err := json.Marshal(v); err == nil {
			return string(data), "json"
		}
		return fmt.Sprintf("%v", v), "string"
	}
}

// parseValue converts a string value back to its original type
func (s *SettingsService) parseValue(value, valueType string) (interface{}, error) {
	switch valueType {
	case "bool":
		return strconv.ParseBool(value)
	case "int":
		return strconv.Atoi(value)
	case "float":
		return strconv.ParseFloat(value, 64)
	case "json":
		var result interface{}
		err := json.Unmarshal([]byte(value), &result)
		return result, err
	default:
		return value, nil
	}
}

// applySetting applies a setting to the AppSettings struct
func (s *SettingsService) applySetting(appSettings *models.AppSettings, setting *models.Setting) error {
	value, err := s.parseValue(setting.Value, setting.Type)
	if err != nil {
		return err
	}

	switch setting.Key {
	case "app_name":
		if v, ok := value.(string); ok {
			appSettings.AppName = v
		}
	case "app_url":
		if v, ok := value.(string); ok {
			appSettings.AppURL = v
		}
	case "allow_signup":
		if v, ok := value.(bool); ok {
			appSettings.AllowSignup = v
		}
	case "require_email_confirmation":
		if v, ok := value.(bool); ok {
			appSettings.RequireEmailConfirmation = v
		}
	case "mailer_provider":
		if v, ok := value.(string); ok {
			appSettings.MailerProvider = v
		}
	case "mailgun_domain":
		if v, ok := value.(string); ok {
			appSettings.MailgunDomain = v
		}
	case "mailgun_region":
		if v, ok := value.(string); ok {
			appSettings.MailgunRegion = v
		}
	case "mailgun_api_key":
		if v, ok := value.(string); ok {
			appSettings.MailgunAPIKey = v
		}
	case "storage_provider":
		if v, ok := value.(string); ok {
			appSettings.StorageProvider = v
		}
	case "s3_bucket":
		if v, ok := value.(string); ok {
			appSettings.S3Bucket = v
		}
	case "s3_region":
		if v, ok := value.(string); ok {
			appSettings.S3Region = v
		}
	case "s3_access_key":
		if v, ok := value.(string); ok {
			appSettings.S3AccessKey = v
		}
	case "s3_secret_key":
		if v, ok := value.(string); ok {
			appSettings.S3SecretKey = v
		}
	case "max_upload_size":
		if v, ok := value.(int); ok {
			appSettings.MaxUploadSize = int64(v)
		}
	case "allowed_file_types":
		if v, ok := value.(string); ok {
			appSettings.AllowedFileTypes = v
		}
	case "session_timeout":
		if v, ok := value.(int); ok {
			appSettings.SessionTimeout = v
		}
	case "password_min_length":
		if v, ok := value.(int); ok {
			appSettings.PasswordMinLength = v
		}
	case "enable_api_logs":
		if v, ok := value.(bool); ok {
			appSettings.EnableAPILogs = v
		}
	case "enable_debug_mode":
		if v, ok := value.(bool); ok {
			appSettings.EnableDebugMode = v
		}
	case "maintenance_mode":
		if v, ok := value.(bool); ok {
			appSettings.MaintenanceMode = v
		}
	case "maintenance_message":
		if v, ok := value.(string); ok {
			appSettings.MaintenanceMessage = v
		}
	case "notification":
		if v, ok := value.(string); ok {
			appSettings.Notification = v
		}
	}

	return nil
}

// DeleteSetting removes a setting
func (s *SettingsService) DeleteSetting(key string) error {
	ctx := context.Background()
	return s.repo.SoftDeleteByKey(ctx, key)
}

// ResetToDefaults resets all settings to default values
func (s *SettingsService) ResetToDefaults() error {
	ctx := context.Background()

	// Get all settings and delete them
	settings, err := s.repo.List(ctx)
	if err != nil {
		return err
	}

	for _, setting := range settings {
		if err := s.repo.HardDelete(ctx, setting.ID.String()); err != nil {
			return err
		}
	}

	// Reinitialize defaults
	return s.initializeDefaults(ctx)
}
