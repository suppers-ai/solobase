package models

import (
	"time"

	"github.com/google/uuid"
	"gorm.io/gorm"
)

type Setting struct {
	ID        uuid.UUID      `gorm:"type:uuid;primary_key" json:"id"`
	Key       string         `gorm:"uniqueIndex;not null" json:"key"`
	Value     string         `gorm:"type:text" json:"value"`
	Type      string         `gorm:"default:'string'" json:"type"` // string, bool, int, json
	CreatedAt time.Time      `json:"created_at"`
	UpdatedAt time.Time      `json:"updated_at"`
	DeletedAt gorm.DeletedAt `gorm:"index" json:"-"`
}

func (Setting) TableName() string {
	return "settings"
}

func (s *Setting) BeforeCreate(tx *gorm.DB) error {
	if s.ID == uuid.Nil {
		s.ID = uuid.New()
	}
	return nil
}

// AppSettings represents the structured application settings
type AppSettings struct {
	AppName                  string `json:"app_name"`
	AppURL                   string `json:"app_url"`
	AllowSignup              bool   `json:"allow_signup"`
	RequireEmailConfirmation bool   `json:"require_email_confirmation"`
	SMTPEnabled              bool   `json:"smtp_enabled"`
	SMTPHost                 string `json:"smtp_host,omitempty"`
	SMTPPort                 int    `json:"smtp_port,omitempty"`
	SMTPUser                 string `json:"smtp_user,omitempty"`
	SMTPPassword             string `json:"-"` // Never expose password in JSON
	StorageProvider          string `json:"storage_provider"`
	S3Bucket                 string `json:"s3_bucket,omitempty"`
	S3Region                 string `json:"s3_region,omitempty"`
	S3AccessKey              string `json:"-"` // Never expose in JSON
	S3SecretKey              string `json:"-"` // Never expose in JSON
	MaxUploadSize            int64  `json:"max_upload_size"`
	AllowedFileTypes         string `json:"allowed_file_types"`
	SessionTimeout           int    `json:"session_timeout"` // in minutes
	PasswordMinLength        int    `json:"password_min_length"`
	EnableAPILogs            bool   `json:"enable_api_logs"`
	EnableDebugMode          bool   `json:"enable_debug_mode"`
	MaintenanceMode          bool   `json:"maintenance_mode"`
	MaintenanceMessage       string `json:"maintenance_message,omitempty"`
	Notification             string `json:"notification,omitempty"`
}

// DefaultSettings returns the default application settings
func DefaultSettings() *AppSettings {
	return &AppSettings{
		AppName:                  "Solobase",
		AppURL:                   "http://localhost:8080",
		AllowSignup:              true,
		RequireEmailConfirmation: false,
		SMTPEnabled:              false,
		SMTPPort:                 587,
		StorageProvider:          "local",
		MaxUploadSize:            10 * 1024 * 1024, // 10MB
		AllowedFileTypes:         "image/*,application/pdf,text/*",
		SessionTimeout:           1440, // 24 hours
		PasswordMinLength:        8,
		EnableAPILogs:            true,
		EnableDebugMode:          false,
		MaintenanceMode:          false,
	}
}
