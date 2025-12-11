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
	CreatedAt time.Time      `json:"createdAt"`
	UpdatedAt time.Time      `json:"updatedAt"`
	DeletedAt gorm.DeletedAt `gorm:"index" json:"-"`
}

func (Setting) TableName() string {
	return "sys_settings"
}

func (s *Setting) BeforeCreate(tx *gorm.DB) error {
	if s.ID == uuid.Nil {
		s.ID = uuid.New()
	}
	return nil
}

// AppSettings represents the structured application settings
type AppSettings struct {
	AppName                  string `json:"appName"`
	AppURL                   string `json:"appUrl"`
	AllowSignup              bool   `json:"allowSignup"`
	RequireEmailConfirmation bool   `json:"requireEmailConfirmation"`
	SMTPEnabled              bool   `json:"smtpEnabled"`
	SMTPHost                 string `json:"smtpHost,omitempty"`
	SMTPPort                 int    `json:"smtpPort,omitempty"`
	SMTPUser                 string `json:"smtpUser,omitempty"`
	SMTPPassword             string `json:"-"` // Never expose password in JSON
	StorageProvider          string `json:"storageProvider"`
	S3Bucket                 string `json:"s3Bucket,omitempty"`
	S3Region                 string `json:"s3Region,omitempty"`
	S3AccessKey              string `json:"-"` // Never expose in JSON
	S3SecretKey              string `json:"-"` // Never expose in JSON
	MaxUploadSize            int64  `json:"maxUploadSize"`
	AllowedFileTypes         string `json:"allowedFileTypes"`
	SessionTimeout           int    `json:"sessionTimeout"` // in minutes
	PasswordMinLength        int    `json:"passwordMinLength"`
	EnableAPILogs            bool   `json:"enableApiLogs"`
	EnableDebugMode          bool   `json:"enableDebugMode"`
	MaintenanceMode          bool   `json:"maintenanceMode"`
	MaintenanceMessage       string `json:"maintenanceMessage,omitempty"`
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
