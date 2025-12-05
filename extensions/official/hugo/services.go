package hugo

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"github.com/google/uuid"
	"gorm.io/gorm"
)

// HugoService handles Hugo-related operations
type HugoService struct {
	db     *gorm.DB
	config HugoConfig
}

// NewHugoService creates a new Hugo service
func NewHugoService(db *gorm.DB, config HugoConfig) *HugoService {
	return &HugoService{
		db:     db,
		config: config,
	}
}

// CheckHugoInstalled checks if Hugo is installed
func (s *HugoService) CheckHugoInstalled() bool {
	cmd := exec.Command(s.config.HugoBinaryPath, "version")
	return cmd.Run() == nil
}

// ListSites lists all sites for a user
func (s *HugoService) ListSites(userID string) ([]HugoSite, error) {
	var sites []HugoSite
	err := s.db.Where("user_id = ?", userID).Order("created_at DESC").Find(&sites).Error
	return sites, err
}

// GetSite gets a site by ID
func (s *HugoService) GetSite(siteID string) (*HugoSite, error) {
	var site HugoSite
	err := s.db.Where("id = ?", siteID).First(&site).Error
	return &site, err
}

// CreateSite creates a new Hugo site
func (s *HugoService) CreateSite(userID, name, domain, theme string, isExample bool) (*HugoSite, error) {
	// Generate unique ID
	id := uuid.New().String()

	// Create site record
	site := &HugoSite{
		ID:        id,
		UserID:    userID,
		Name:      name,
		Domain:    domain,
		Theme:     theme,
		Status:    "draft",
		Size:      "0 MB",
		Pages:     0,
		Visits:    0,
		IsExample: isExample,
		CreatedAt: time.Now(),
		UpdatedAt: time.Now(),
	}

	// Save to database
	if err := s.db.Create(site).Error; err != nil {
		return nil, fmt.Errorf("failed to create site in database: %w", err)
	}

	// Create site directory structure
	if err := s.initializeSiteStructure(site); err != nil {
		// Rollback database entry
		s.db.Delete(site)
		return nil, fmt.Errorf("failed to initialize site structure: %w", err)
	}

	return site, nil
}

// DeleteSite deletes a Hugo site
func (s *HugoService) DeleteSite(siteID string) error {
	// Get site first
	site, err := s.GetSite(siteID)
	if err != nil {
		return err
	}

	// Delete site files
	sitePath := s.getSitePath(siteID)
	if err := os.RemoveAll(sitePath); err != nil {
		return fmt.Errorf("failed to delete site files: %w", err)
	}

	// Delete from database
	return s.db.Delete(site).Error
}

// BuildSite builds a Hugo site
func (s *HugoService) BuildSite(siteID string) (map[string]interface{}, error) {
	site, err := s.GetSite(siteID)
	if err != nil {
		return nil, err
	}

	sitePath := s.getSitePath(siteID)

	// Update status to building
	site.Status = "building"
	s.db.Save(site)

	// Run Hugo build
	start := time.Now()
	cmd := exec.Command(s.config.HugoBinaryPath, "-s", sitePath, "-d", filepath.Join(sitePath, "public"))
	output, err := cmd.CombinedOutput()

	buildTime := time.Since(start)
	now := time.Now()

	if err != nil {
		site.Status = "error"
		site.UpdatedAt = now
		s.db.Save(site)
		return nil, fmt.Errorf("hugo build failed: %w\nOutput: %s", err, string(output))
	}

	// Update site with build info
	site.Status = "published"
	site.LastBuild = &now
	site.BuildTime = fmt.Sprintf("%.2fs", buildTime.Seconds())
	site.Pages = s.countPages(sitePath)
	site.Size = s.calculateSize(sitePath)
	site.UpdatedAt = now
	s.db.Save(site)

	return map[string]interface{}{
		"status":    "success",
		"buildTime": buildTime.Seconds(),
		"pages":     site.Pages,
		"output":    string(output),
	}, nil
}

// ListFiles lists all files in a Hugo site
func (s *HugoService) ListFiles(siteID string) ([]HugoFileNode, error) {
	sitePath := s.getSitePath(siteID)

	// Check if site exists
	if _, err := os.Stat(sitePath); os.IsNotExist(err) {
		return nil, fmt.Errorf("site directory not found")
	}

	return s.buildFileTree(sitePath, "")
}

// ReadFile reads a file from a Hugo site
func (s *HugoService) ReadFile(siteID, filePath string) (string, error) {
	sitePath := s.getSitePath(siteID)
	fullPath := filepath.Join(sitePath, filePath)

	// Security check - ensure path is within site directory
	if !strings.HasPrefix(filepath.Clean(fullPath), filepath.Clean(sitePath)) {
		return "", fmt.Errorf("invalid file path")
	}

	content, err := os.ReadFile(fullPath)
	if err != nil {
		return "", fmt.Errorf("failed to read file: %w", err)
	}

	return string(content), nil
}

// SaveFile saves a file in a Hugo site
func (s *HugoService) SaveFile(siteID, filePath, content string) error {
	sitePath := s.getSitePath(siteID)
	fullPath := filepath.Join(sitePath, filePath)

	// Security check - ensure path is within site directory
	if !strings.HasPrefix(filepath.Clean(fullPath), filepath.Clean(sitePath)) {
		return fmt.Errorf("invalid file path")
	}

	// Ensure directory exists
	dir := filepath.Dir(fullPath)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return fmt.Errorf("failed to create directory: %w", err)
	}

	// Write file
	if err := os.WriteFile(fullPath, []byte(content), 0644); err != nil {
		return fmt.Errorf("failed to write file: %w", err)
	}

	// Update site timestamp
	s.db.Model(&HugoSite{}).Where("id = ?", siteID).Update("updated_at", time.Now())

	return nil
}

// GetStats returns statistics for a user
func (s *HugoService) GetStats(userID string) (*HugoStats, error) {
	var stats HugoStats

	// Count total sites
	s.db.Model(&HugoSite{}).Where("user_id = ?", userID).Count(&stats.TotalSites)

	// Count active sites
	s.db.Model(&HugoSite{}).Where("user_id = ? AND status = ?", userID, "published").Count(&stats.ActiveSites)

	// Count total builds (sites that have been built at least once)
	var buildCount int64
	s.db.Model(&HugoSite{}).Where("user_id = ? AND last_build IS NOT NULL", userID).Count(&buildCount)
	stats.TotalBuilds = int(buildCount)

	// Calculate storage used
	var sites []HugoSite
	s.db.Where("user_id = ?", userID).Find(&sites)
	totalSize := int64(0)
	for _, site := range sites {
		sitePath := s.getSitePath(site.ID)
		totalSize += s.getDirSize(sitePath)
	}
	stats.StorageUsed = formatSize(totalSize)

	return &stats, nil
}

// Helper methods

// getSitePath returns the filesystem path for a site
func (s *HugoService) getSitePath(siteID string) string {
	// Store sites in storage/ext/hugo/sites/{siteID}
	return filepath.Join("storage", "ext", "hugo", "sites", siteID)
}

// initializeSiteStructure creates the initial Hugo site structure
func (s *HugoService) initializeSiteStructure(site *HugoSite) error {
	sitePath := s.getSitePath(site.ID)

	// Create site directory
	if err := os.MkdirAll(sitePath, 0755); err != nil {
		return err
	}

	// Create Hugo site structure
	dirs := []string{
		"content/posts",
		"content/pages",
		"layouts/_default",
		"layouts/partials",
		"static/css",
		"static/js",
		"themes",
	}

	for _, dir := range dirs {
		if err := os.MkdirAll(filepath.Join(sitePath, dir), 0755); err != nil {
			return err
		}
	}

	// Create config file
	configContent := fmt.Sprintf(`baseURL = "https://%s/"
languageCode = "en-us"
title = "%s"
theme = "%s"

[params]
  description = "A Hugo site powered by Solobase"
`, site.Domain, site.Name, site.Theme)

	if err := os.WriteFile(filepath.Join(sitePath, "config.toml"), []byte(configContent), 0644); err != nil {
		return err
	}

	// Create initial content if this is an example site
	if site.IsExample {
		if err := s.createExampleContent(sitePath); err != nil {
			return err
		}
	}

	return nil
}

// createExampleContent creates example content for a new site
func (s *HugoService) createExampleContent(sitePath string) error {
	// Create example post
	examplePost := `---
title: "Welcome to Hugo"
date: ` + time.Now().Format("2006-01-02") + `
draft: false
tags: ["welcome", "getting-started"]
---

# Welcome to Your Hugo Site!

This is an example post to get you started. You can edit this file or create new ones.

## Features

- Fast static site generation
- Markdown support
- Custom themes
- Easy deployment

Happy blogging!
`
	postPath := filepath.Join(sitePath, "content", "posts", "welcome.md")
	if err := os.WriteFile(postPath, []byte(examplePost), 0644); err != nil {
		return err
	}

	// Create about page
	aboutPage := `---
title: "About"
date: ` + time.Now().Format("2006-01-02") + `
draft: false
---

# About This Site

This is a Hugo static site created with Solobase.
`
	aboutPath := filepath.Join(sitePath, "content", "pages", "about.md")
	if err := os.WriteFile(aboutPath, []byte(aboutPage), 0644); err != nil {
		return err
	}

	return nil
}

// buildFileTree recursively builds a file tree
func (s *HugoService) buildFileTree(basePath, relativePath string) ([]HugoFileNode, error) {
	fullPath := filepath.Join(basePath, relativePath)
	entries, err := os.ReadDir(fullPath)
	if err != nil {
		return nil, err
	}

	var nodes []HugoFileNode
	for _, entry := range entries {
		name := entry.Name()

		// Skip hidden files and build artifacts
		if strings.HasPrefix(name, ".") || name == "public" || name == "resources" {
			continue
		}

		nodePath := filepath.Join(relativePath, name)
		nodeType := "file"
		var children []HugoFileNode

		if entry.IsDir() {
			nodeType = "directory"
			// Recursively build children
			children, _ = s.buildFileTree(basePath, nodePath)
		}

		nodes = append(nodes, HugoFileNode{
			ID:       strings.ReplaceAll(nodePath, "/", "-"),
			Name:     name,
			Path:     nodePath,
			Type:     nodeType,
			Children: children,
		})
	}

	return nodes, nil
}

// countPages counts the number of content pages in a site
func (s *HugoService) countPages(sitePath string) int {
	count := 0
	contentPath := filepath.Join(sitePath, "content")

	filepath.Walk(contentPath, func(path string, info os.FileInfo, err error) error {
		if err == nil && !info.IsDir() && strings.HasSuffix(path, ".md") {
			count++
		}
		return nil
	})

	return count
}

// calculateSize calculates the size of a site directory
func (s *HugoService) calculateSize(sitePath string) string {
	size := s.getDirSize(sitePath)
	return formatSize(size)
}

// getDirSize calculates directory size in bytes
func (s *HugoService) getDirSize(path string) int64 {
	var size int64
	filepath.Walk(path, func(_ string, info os.FileInfo, err error) error {
		if err == nil && !info.IsDir() {
			size += info.Size()
		}
		return nil
	})
	return size
}

// formatSize formats bytes into human-readable size
func formatSize(bytes int64) string {
	const unit = 1024
	if bytes < unit {
		return fmt.Sprintf("%d B", bytes)
	}
	div, exp := int64(unit), 0
	for n := bytes / unit; n >= unit; n /= unit {
		div *= unit
		exp++
	}
	return fmt.Sprintf("%.1f %cB", float64(bytes)/float64(div), "KMGTPE"[exp])
}
