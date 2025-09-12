package services

import (
	"encoding/json"
	"fmt"
	"strconv"

	"github.com/suppers-ai/solobase/database"
	"github.com/suppers-ai/solobase/internal/data/models"
	"gorm.io/gorm"
)

type SettingsService struct {
	db *database.DB
}

func NewSettingsService(db *database.DB) *SettingsService {
	service := &SettingsService{db: db}
	// Initialize default settings on first run
	service.initializeDefaults()
	// Initialize extension-specific settings
	service.initializeExtensionSettings()
	return service
}

// initializeExtensionSettings creates extension-specific default settings
func (s *SettingsService) initializeExtensionSettings() error {
	// Initialize CloudStorage extension setting for showing usage in profile
	// This is set to true by default when CloudStorage is available
	err := s.setSetting("ext_cloudstorage_profile_show_usage", true)
	if err != nil {
		// Log but don't fail - setting might already exist
		return nil
	}
	return nil
}

// initializeDefaults creates default settings if they don't exist
func (s *SettingsService) initializeDefaults() error {
	defaults := models.DefaultSettings()

	// Check if any settings exist
	var count int64
	s.db.Model(&models.Setting{}).Count(&count)
	if count > 0 {
		return nil // Settings already initialized
	}

	// Convert defaults to individual settings
	settings := map[string]interface{}{
		"app_name":                   defaults.AppName,
		"app_url":                    defaults.AppURL,
		"allow_signup":               defaults.AllowSignup,
		"require_email_confirmation": defaults.RequireEmailConfirmation,
		"smtp_enabled":               defaults.SMTPEnabled,
		"smtp_host":                  defaults.SMTPHost,
		"smtp_port":                  defaults.SMTPPort,
		"smtp_user":                  defaults.SMTPUser,
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
	for key, value := range settings {
		if err := s.setSetting(key, value); err != nil {
			return err
		}
	}

	return nil
}

// GetSettings retrieves all settings as AppSettings struct
func (s *SettingsService) GetSettings() (*models.AppSettings, error) {
	var settings []models.Setting
	if err := s.db.Find(&settings).Error; err != nil {
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
	// Validate and update each setting
	for key, value := range updates {
		if err := s.setSetting(key, value); err != nil {
			return nil, fmt.Errorf("failed to update setting %s: %w", key, err)
		}
	}

	// Return updated settings
	return s.GetSettings()
}

// GetSetting retrieves a single setting value
func (s *SettingsService) GetSetting(key string) (interface{}, error) {
	var setting models.Setting
	if err := s.db.Where("key = ?", key).First(&setting).Error; err != nil {
		if err == gorm.ErrRecordNotFound {
			return nil, fmt.Errorf("setting not found: %s", key)
		}
		return nil, err
	}

	return s.parseValue(setting.Value, setting.Type)
}

// SetSetting updates or creates a single setting (public method)
func (s *SettingsService) SetSetting(key string, value interface{}) error {
	return s.setSetting(key, value)
}

// setSetting updates or creates a single setting
func (s *SettingsService) setSetting(key string, value interface{}) error {
	var setting models.Setting

	// Determine type and convert value to string
	valueStr, valueType := s.serializeValue(value)

	// Check if setting exists
	err := s.db.Where("key = ?", key).First(&setting).Error
	if err == gorm.ErrRecordNotFound {
		// Create new setting
		setting = models.Setting{
			Key:   key,
			Value: valueStr,
			Type:  valueType,
		}
		return s.db.Create(&setting).Error
	} else if err != nil {
		return err
	}

	// Update existing setting
	setting.Value = valueStr
	setting.Type = valueType
	return s.db.Save(&setting).Error
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
func (s *SettingsService) applySetting(appSettings *models.AppSettings, setting models.Setting) error {
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
	case "smtp_enabled":
		if v, ok := value.(bool); ok {
			appSettings.SMTPEnabled = v
		}
	case "smtp_host":
		if v, ok := value.(string); ok {
			appSettings.SMTPHost = v
		}
	case "smtp_port":
		if v, ok := value.(int); ok {
			appSettings.SMTPPort = v
		}
	case "smtp_user":
		if v, ok := value.(string); ok {
			appSettings.SMTPUser = v
		}
	case "smtp_password":
		if v, ok := value.(string); ok {
			appSettings.SMTPPassword = v
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
	return s.db.Where("key = ?", key).Delete(&models.Setting{}).Error
}

// ResetToDefaults resets all settings to default values
func (s *SettingsService) ResetToDefaults() error {
	// Delete all existing settings
	if err := s.db.Exec("DELETE FROM settings").Error; err != nil {
		return err
	}

	// Reinitialize defaults
	return s.initializeDefaults()
}
