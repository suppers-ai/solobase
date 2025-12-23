package models

import (
	"github.com/suppers-ai/solobase/internal/pkg/apptime"
	"github.com/suppers-ai/solobase/internal/pkg/uuid"
)

type Setting struct {
	ID        uuid.UUID        `json:"id"`
	Key       string           `json:"key"`
	Value     string           `json:"value"`
	Type      string           `json:"type"` // string, bool, int, json
	CreatedAt apptime.Time     `json:"createdAt"`
	UpdatedAt apptime.Time     `json:"updatedAt"`
	DeletedAt apptime.NullTime `json:"-"`
}

func (Setting) TableName() string {
	return "sys_settings"
}

// PrepareForCreate prepares the setting for database insertion
// Prepares model for database insert
func (s *Setting) PrepareForCreate() {
	if s.ID == uuid.Nil {
		s.ID = uuid.New()
	}
	now := apptime.NowTime()
	s.CreatedAt = now
	s.UpdatedAt = now
}

// AppSettings represents the structured application settings
type AppSettings struct {
	AppName                  string `json:"appName"`
	AppURL                   string `json:"appUrl"`
	AllowSignup              bool   `json:"allowSignup"`
	RequireEmailConfirmation bool   `json:"requireEmailConfirmation"`
	MailerProvider           string `json:"mailerProvider"` // none, mailgun
	MailgunDomain            string `json:"mailgunDomain,omitempty"`
	MailgunRegion            string `json:"mailgunRegion,omitempty"` // us, eu
	MailgunAPIKey            string `json:"-"`                       // Never expose in JSON
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
		MailerProvider:           "none",
		MailgunRegion:            "us",
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
