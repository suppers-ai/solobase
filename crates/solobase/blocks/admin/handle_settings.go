package admin

import (
	"context"
	"encoding/json"
	"fmt"
	"strconv"

	"github.com/suppers-ai/solobase/core/models"
	waffle "github.com/suppers-ai/waffle-go"
	"github.com/suppers-ai/waffle-go/services/database"
)

const settingsCollection = "sys_settings"

func (b *AdminBlock) registerSettingsRoutes() {
	// Protected (read-only)
	b.router.Retrieve("/settings", b.handleGetSettings)
	b.router.Retrieve("/settings/{key}", b.handleGetSetting)
	// Admin (write)
	b.router.Update("/admin/settings", b.handleUpdateSettings)
	b.router.Create("/admin/settings", b.handleSetSetting)
	b.router.Create("/admin/settings/reset", b.handleResetSettings)
}

func (b *AdminBlock) handleGetSettings(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	appSettings, err := b.getSettings(db)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to fetch settings")
	}
	return waffle.JSONRespond(msg, 200, appSettings)
}

func (b *AdminBlock) handleGetSetting(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	key := msg.Var("key")
	record, err := database.GetByField(context.Background(), db, settingsCollection, "key", key)
	if err != nil {
		if err == database.ErrNotFound {
			return waffle.Error(msg, 404, "not_found", "Setting not found")
		}
		return waffle.Error(msg, 500, "internal_error", "Failed to fetch setting")
	}

	value, err := parseSettingValue(
		fmt.Sprintf("%v", record.Data["value"]),
		fmt.Sprintf("%v", record.Data["type"]),
	)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to parse setting value")
	}

	return waffle.JSONRespond(msg, 200, map[string]any{
		"key":   key,
		"value": value,
	})
}

func (b *AdminBlock) handleUpdateSettings(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	var updates map[string]any
	if err := msg.Decode(&updates); err != nil {
		return waffle.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	for key, value := range updates {
		if err := b.upsertSetting(db, key, value); err != nil {
			return waffle.Error(msg, 500, "internal_error", fmt.Sprintf("Failed to update setting %s", key))
		}
	}

	appSettings, err := b.getSettings(db)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to fetch settings")
	}
	return waffle.JSONRespond(msg, 200, appSettings)
}

func (b *AdminBlock) handleSetSetting(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	var body struct {
		Key   string `json:"key"`
		Value any    `json:"value"`
		Type  string `json:"type,omitempty"`
	}
	if err := msg.Decode(&body); err != nil {
		return waffle.Error(msg, 400, "bad_request", "Invalid JSON body")
	}

	if body.Key == "" {
		return waffle.Error(msg, 400, "bad_request", "Setting key is required")
	}

	if err := b.upsertSetting(db, body.Key, body.Value); err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to update setting")
	}

	return waffle.JSONRespond(msg, 200, map[string]any{
		"success": true,
		"key":     body.Key,
		"value":   body.Value,
	})
}

func (b *AdminBlock) handleResetSettings(ctx waffle.Context, msg *waffle.Message) waffle.Result {
	db := ctx.Services().Database
	if db == nil {
		return waffle.Error(msg, 503, "unavailable", "Database service not available")
	}

	records, err := database.ListAll(context.Background(), db, settingsCollection)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to fetch settings")
	}
	for _, r := range records {
		_ = db.Delete(context.Background(), settingsCollection, r.ID)
	}

	if err := b.initializeDefaults(db); err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to reset settings")
	}

	appSettings, err := b.getSettings(db)
	if err != nil {
		return waffle.Error(msg, 500, "internal_error", "Failed to fetch settings after reset")
	}
	return waffle.JSONRespond(msg, 200, appSettings)
}

// --- Internal settings helpers ---

func (b *AdminBlock) getSettings(db database.Service) (*models.AppSettings, error) {
	ctx := context.Background()
	records, err := database.ListAll(ctx, db, settingsCollection,
		database.Filter{Field: "deleted_at", Operator: database.OpIsNull},
	)
	if err != nil {
		return nil, err
	}

	appSettings := models.DefaultSettings()
	for _, r := range records {
		key, _ := r.Data["key"].(string)
		valueStr := fmt.Sprintf("%v", r.Data["value"])
		typeStr := fmt.Sprintf("%v", r.Data["type"])
		value, err := parseSettingValue(valueStr, typeStr)
		if err != nil {
			continue
		}
		applySettingToAppSettings(appSettings, key, value)
	}
	return appSettings, nil
}

func (b *AdminBlock) upsertSetting(db database.Service, key string, value any) error {
	valueStr, valueType := serializeSettingValue(value)
	data := map[string]any{
		"key":   key,
		"value": valueStr,
		"type":  valueType,
	}
	_, err := database.Upsert(context.Background(), db, settingsCollection, "key", key, data)
	return err
}

func (b *AdminBlock) initializeDefaults(db database.Service) error {
	ctx := context.Background()

	records, err := database.ListAll(ctx, db, settingsCollection)
	if err != nil {
		return err
	}
	if len(records) > 0 {
		return nil
	}

	defaults := models.DefaultSettings()
	settingsMap := map[string]any{
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
		"ext_cloudstorage_profile_show_usage": true,
	}

	for key, value := range settingsMap {
		if err := b.upsertSetting(db, key, value); err != nil {
			return err
		}
	}
	return nil
}

func serializeSettingValue(value any) (string, string) {
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
	case map[string]any, []any:
		data, _ := json.Marshal(v)
		return string(data), "json"
	default:
		if data, err := json.Marshal(v); err == nil {
			return string(data), "json"
		}
		return fmt.Sprintf("%v", v), "string"
	}
}

func parseSettingValue(value, valueType string) (any, error) {
	switch valueType {
	case "bool":
		return strconv.ParseBool(value)
	case "int":
		return strconv.Atoi(value)
	case "float":
		return strconv.ParseFloat(value, 64)
	case "json":
		var result any
		err := json.Unmarshal([]byte(value), &result)
		return result, err
	default:
		return value, nil
	}
}

func applySettingToAppSettings(s *models.AppSettings, key string, value any) {
	switch key {
	case "app_name":
		if v, ok := value.(string); ok {
			s.AppName = v
		}
	case "app_url":
		if v, ok := value.(string); ok {
			s.AppURL = v
		}
	case "allow_signup":
		if v, ok := value.(bool); ok {
			s.AllowSignup = v
		}
	case "require_email_confirmation":
		if v, ok := value.(bool); ok {
			s.RequireEmailConfirmation = v
		}
	case "mailer_provider":
		if v, ok := value.(string); ok {
			s.MailerProvider = v
		}
	case "mailgun_domain":
		if v, ok := value.(string); ok {
			s.MailgunDomain = v
		}
	case "mailgun_region":
		if v, ok := value.(string); ok {
			s.MailgunRegion = v
		}
	case "mailgun_api_key":
		if v, ok := value.(string); ok {
			s.MailgunAPIKey = v
		}
	case "storage_provider":
		if v, ok := value.(string); ok {
			s.StorageProvider = v
		}
	case "s3_bucket":
		if v, ok := value.(string); ok {
			s.S3Bucket = v
		}
	case "s3_region":
		if v, ok := value.(string); ok {
			s.S3Region = v
		}
	case "s3_access_key":
		if v, ok := value.(string); ok {
			s.S3AccessKey = v
		}
	case "s3_secret_key":
		if v, ok := value.(string); ok {
			s.S3SecretKey = v
		}
	case "max_upload_size":
		if v, ok := value.(int); ok {
			s.MaxUploadSize = int64(v)
		}
	case "allowed_file_types":
		if v, ok := value.(string); ok {
			s.AllowedFileTypes = v
		}
	case "session_timeout":
		if v, ok := value.(int); ok {
			s.SessionTimeout = v
		}
	case "password_min_length":
		if v, ok := value.(int); ok {
			s.PasswordMinLength = v
		}
	case "enable_api_logs":
		if v, ok := value.(bool); ok {
			s.EnableAPILogs = v
		}
	case "enable_debug_mode":
		if v, ok := value.(bool); ok {
			s.EnableDebugMode = v
		}
	case "maintenance_mode":
		if v, ok := value.(bool); ok {
			s.MaintenanceMode = v
		}
	case "maintenance_message":
		if v, ok := value.(string); ok {
			s.MaintenanceMessage = v
		}
	case "notification":
		if v, ok := value.(string); ok {
			s.Notification = v
		}
	}
}
