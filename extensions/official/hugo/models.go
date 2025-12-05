package hugo

import (
	"time"
)

// HugoSite represents a Hugo static site
type HugoSite struct {
	ID          string     `json:"id" gorm:"primaryKey"`
	UserID      string     `json:"user_id" gorm:"index;not null"`
	Name        string     `json:"name" gorm:"not null"`
	Domain      string     `json:"domain"`
	Theme       string     `json:"theme" gorm:"default:'default'"`
	Status      string     `json:"status" gorm:"default:'draft'"`
	LastBuild   *time.Time `json:"lastBuild"`
	BuildTime   string     `json:"buildTime"`
	Size        string     `json:"size"`
	Pages       int        `json:"pages"`
	Visits      int        `json:"visits"`
	IsExample   bool       `json:"isExample" gorm:"default:false"`
	CreatedAt   time.Time  `json:"created_at"`
	UpdatedAt   time.Time  `json:"updated_at"`
}

// HugoStats represents aggregated statistics
type HugoStats struct {
	TotalSites   int    `json:"totalSites"`
	ActiveSites  int    `json:"activeSites"`
	TotalBuilds  int    `json:"totalBuilds"`
	StorageUsed  string `json:"storageUsed"`
}

// HugoFileNode represents a file or directory in the Hugo site
type HugoFileNode struct {
	ID       string          `json:"id"`
	Name     string          `json:"name"`
	Path     string          `json:"path"`
	Type     string          `json:"type"` // "file" or "directory"
	Children []HugoFileNode  `json:"children,omitempty"`
}

// HugoConfig represents Hugo configuration
type HugoConfig struct {
	HugoBinaryPath   string   `json:"hugo_binary_path"`
	MaxSitesPerUser  int      `json:"max_sites_per_user"`
	MaxSiteSize      int64    `json:"max_site_size"`
	BuildTimeout     string   `json:"build_timeout"`
	AllowedThemes    []string `json:"allowed_themes"`
	DefaultTheme     string   `json:"default_theme"`
	StorageBucket    string   `json:"storage_bucket"`
}
