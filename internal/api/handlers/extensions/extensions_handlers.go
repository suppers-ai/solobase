package extensions

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"github.com/google/uuid"
	"github.com/gorilla/mux"
)

// Products Extension Handlers

type Product struct {
	ID          string    `json:"id"`
	Name        string    `json:"name"`
	Category    string    `json:"category"`
	Price       float64   `json:"price"`
	Currency    string    `json:"currency"`
	Status      string    `json:"status"`
	Sales       int       `json:"sales"`
	Revenue     float64   `json:"revenue"`
	Description string    `json:"description"`
	CreatedAt   time.Time `json:"created_at"`
	UpdatedAt   time.Time `json:"updated_at"`
}

type ProductsStats struct {
	TotalProducts  int     `json:"totalProducts"`
	ActiveProducts int     `json:"activeProducts"`
	TotalRevenue   float64 `json:"totalRevenue"`
	AvgPrice       float64 `json:"avgPrice"`
}

func HandleProductsList() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// TODO: Fetch from database when products table is implemented
		products := []Product{}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(products)
	}
}

func HandleProductsCreate() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var product Product
		if err := json.NewDecoder(r.Body).Decode(&product); err != nil {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}

		// TODO: Save to database
		product.ID = generateID()
		product.CreatedAt = time.Now()
		product.UpdatedAt = time.Now()

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		json.NewEncoder(w).Encode(product)
	}
}

func HandleProductsUpdate() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		id := vars["id"]

		var product Product
		if err := json.NewDecoder(r.Body).Decode(&product); err != nil {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}

		// TODO: Update in database
		product.ID = id
		product.UpdatedAt = time.Now()

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(product)
	}
}

func HandleProductsDelete() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		_ = vars["id"]

		// TODO: Delete from database

		w.WriteHeader(http.StatusNoContent)
	}
}

func HandleProductsStats() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// TODO: Calculate from database
		stats := ProductsStats{
			TotalProducts:  0,
			ActiveProducts: 0,
			TotalRevenue:   0,
			AvgPrice:       0,
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(stats)
	}
}

// Hugo Extension Handlers

type HugoSite struct {
	ID        string    `json:"id"`
	Name      string    `json:"name"`
	Domain    string    `json:"domain"`
	Status    string    `json:"status"`
	Theme     string    `json:"theme"`
	LastBuild string    `json:"lastBuild"`
	BuildTime string    `json:"buildTime"`
	Size      string    `json:"size"`
	Pages     int       `json:"pages"`
	Visits    int       `json:"visits"`
	CreatedAt time.Time `json:"created_at"`
	UpdatedAt time.Time `json:"updated_at"`
}

type HugoStats struct {
	TotalSites  int    `json:"totalSites"`
	ActiveSites int    `json:"activeSites"`
	TotalBuilds int    `json:"totalBuilds"`
	StorageUsed string `json:"storageUsed"`
}

func HandleHugoSitesList() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Get sites from extension storage directory
		sitesDir := filepath.Join(".data", "storage", "ext", "hugo", "sites")

		sites := []HugoSite{}

		// Check if directory exists
		if _, err := os.Stat(sitesDir); os.IsNotExist(err) {
			w.Header().Set("Content-Type", "application/json")
			json.NewEncoder(w).Encode(sites)
			return
		}

		// Read site directories
		entries, err := os.ReadDir(sitesDir)
		if err != nil {
			w.Header().Set("Content-Type", "application/json")
			json.NewEncoder(w).Encode(sites)
			return
		}

		for _, entry := range entries {
			if entry.IsDir() {
				// Read site metadata if exists
				metaPath := filepath.Join(sitesDir, entry.Name(), "site.json")
				if metaData, err := os.ReadFile(metaPath); err == nil {
					var site HugoSite
					if err := json.Unmarshal(metaData, &site); err == nil {
						sites = append(sites, site)
					}
				}
			}
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(sites)
	}
}

func HandleHugoSitesCreate() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Parse request including optional isExample flag
		var reqData struct {
			HugoSite
			IsExample bool `json:"isExample"`
		}
		if err := json.NewDecoder(r.Body).Decode(&reqData); err != nil {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}

		site := reqData.HugoSite

		// Generate unique ID
		site.ID = uuid.New().String()
		site.Status = "draft"
		site.CreatedAt = time.Now()
		site.UpdatedAt = time.Now()
		site.LastBuild = "Never"
		site.BuildTime = "0s"
		site.Size = "0 MB"
		site.Pages = 0
		site.Visits = 0

		// Create site directory using extension storage structure
		// Hugo works with filesystem, so we use ./.data/storage/ext/hugo/sites/{site-id}
		siteDir := filepath.Join(".data", "storage", "ext", "hugo", "sites", site.ID)
		if err := os.MkdirAll(siteDir, 0755); err != nil {
			http.Error(w, "Failed to create site directory", http.StatusInternalServerError)
			return
		}

		// Create Hugo site using hugo command
		hugoPath := filepath.Join(os.Getenv("HOME"), "bin", "hugo")
		if _, err := os.Stat(hugoPath); os.IsNotExist(err) {
			hugoPath = "hugo" // Fallback to system hugo
		}

		cmd := exec.Command(hugoPath, "new", "site", siteDir, "--force")
		if output, err := cmd.CombinedOutput(); err != nil {
			os.RemoveAll(siteDir) // Clean up on failure
			http.Error(w, fmt.Sprintf("Failed to create Hugo site: %s", output), http.StatusInternalServerError)
			return
		}

		// Create hugo.toml configuration
		themeConfig := ""
		if site.Theme != "" && site.Theme != "default" {
			themeConfig = fmt.Sprintf("theme = \"%s\"\n", site.Theme)
		}

		config := fmt.Sprintf(`baseURL = "https://%s/"
languageCode = "en-us"
title = "%s"
%s
[module]
  [module.hugoVersion]
    extended = true
    min = "0.110.0"

[outputs]
  home = ["HTML", "RSS"]

[params]
  description = "%s"
  author = "Solobase User"
`, site.Domain, site.Name, themeConfig, site.Name)

		configPath := filepath.Join(siteDir, "hugo.toml")
		if err := os.WriteFile(configPath, []byte(config), 0644); err != nil {
			os.RemoveAll(siteDir)
			http.Error(w, "Failed to create site configuration", http.StatusInternalServerError)
			return
		}

		// Create content directory structure
		contentDir := filepath.Join(siteDir, "content")
		os.MkdirAll(filepath.Join(contentDir, "posts"), 0755)
		os.MkdirAll(filepath.Join(contentDir, "pages"), 0755)

		// Create basic layouts for default theme
		if site.Theme == "" || site.Theme == "default" {
			layoutsDir := filepath.Join(siteDir, "layouts")
			os.MkdirAll(filepath.Join(layoutsDir, "_default"), 0755)
			os.MkdirAll(filepath.Join(layoutsDir, "partials"), 0755)

			// Create base layout
			baseLayout := `<!DOCTYPE html>
<html lang="{{ .Site.LanguageCode }}">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{{ block "title" . }}{{ .Site.Title }}{{ end }}</title>
    <style>
        body { font-family: system-ui, -apple-system, sans-serif; margin: 0; padding: 0; background: #f9fafb; }
        header { background: #06b6d4; color: white; padding: 2rem; }
        nav { background: #0891b2; padding: 1rem 2rem; }
        nav a { color: white; text-decoration: none; margin-right: 1rem; }
        main { max-width: 800px; margin: 2rem auto; padding: 0 1rem; }
        article { background: white; padding: 2rem; margin-bottom: 2rem; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }
        footer { background: #374151; color: white; padding: 2rem; text-align: center; margin-top: 4rem; }
        h1 { margin: 0; }
        .meta { color: #6b7280; font-size: 0.875rem; margin: 1rem 0; }
    </style>
</head>
<body>
    {{ block "header" . }}{{ partial "header.html" . }}{{ end }}
    <main>
        {{ block "main" . }}{{ end }}
    </main>
    {{ block "footer" . }}{{ partial "footer.html" . }}{{ end }}
</body>
</html>`
			os.WriteFile(filepath.Join(layoutsDir, "_default", "baseof.html"), []byte(baseLayout), 0644)

			// Create list layout
			listLayout := `{{ define "main" }}
    <h1>{{ .Title }}</h1>
    {{ range .Pages }}
    <article>
        <h2><a href="{{ .Permalink }}">{{ .Title }}</a></h2>
        <div class="meta">{{ .Date.Format "January 2, 2006" }}</div>
        <p>{{ .Summary }}</p>
    </article>
    {{ end }}
{{ end }}`
			os.WriteFile(filepath.Join(layoutsDir, "_default", "list.html"), []byte(listLayout), 0644)

			// Create single layout
			singleLayout := `{{ define "main" }}
    <article>
        <h1>{{ .Title }}</h1>
        <div class="meta">{{ .Date.Format "January 2, 2006" }}</div>
        {{ .Content }}
    </article>
{{ end }}`
			os.WriteFile(filepath.Join(layoutsDir, "_default", "single.html"), []byte(singleLayout), 0644)

			// Create index layout
			indexLayout := `{{ define "main" }}
    <h1>Welcome to {{ .Site.Title }}</h1>
    {{ .Content }}
    
    <h2>Recent Posts</h2>
    {{ range first 5 (where .Site.RegularPages "Section" "posts") }}
    <article>
        <h3><a href="{{ .Permalink }}">{{ .Title }}</a></h3>
        <div class="meta">{{ .Date.Format "January 2, 2006" }}</div>
        <p>{{ .Summary }}</p>
    </article>
    {{ end }}
{{ end }}`
			os.WriteFile(filepath.Join(layoutsDir, "index.html"), []byte(indexLayout), 0644)

			// Create header partial
			headerPartial := `<header>
    <h1>{{ .Site.Title }}</h1>
    <p>{{ .Site.Params.description }}</p>
</header>
<nav>
    <a href="/">Home</a>
    <a href="/posts/">Posts</a>
    <a href="/pages/">Pages</a>
</nav>`
			os.WriteFile(filepath.Join(layoutsDir, "partials", "header.html"), []byte(headerPartial), 0644)

			// Create footer partial
			footerPartial := `<footer>
    <p>&copy; {{ now.Year }} {{ .Site.Title }}. Built with Hugo and Solobase.</p>
</footer>`
			os.WriteFile(filepath.Join(layoutsDir, "partials", "footer.html"), []byte(footerPartial), 0644)
		}

		// Create content based on whether it's an example site
		if reqData.IsExample {
			// Create rich example content
			createExampleContent(contentDir, site.Name)
		} else {
			// Create minimal starter content
			samplePost := `---
title: "Welcome to Your New Site"
date: %s
draft: false
---

# Welcome!

This is your new Hugo site. You can edit this content or create new posts.

## Getting Started

1. Add new content in the content directory
2. Customize your theme
3. Build and deploy your site
`
			postPath := filepath.Join(contentDir, "posts", "welcome.md")
			os.WriteFile(postPath, []byte(fmt.Sprintf(samplePost, time.Now().Format(time.RFC3339))), 0644)

			// Create index page
			indexContent := `---
title: "Home"
---

# Welcome to %s

Your new Hugo site is ready!
`
			indexPath := filepath.Join(contentDir, "_index.md")
			os.WriteFile(indexPath, []byte(fmt.Sprintf(indexContent, site.Name)), 0644)
		}

		// Download and install theme if not default
		if site.Theme != "" && site.Theme != "default" {
			themesDir := filepath.Join(siteDir, "themes")
			os.MkdirAll(themesDir, 0755)

			// Map theme names to actual Hugo themes
			themeMap := map[string]string{
				"business-pro": "github.com/gohugoio/hugo-theme-ananke",
				"docs-minimal": "github.com/google/docsy",
				"blog-modern":  "github.com/chipzoller/hugo-clarity",
				"portfolio":    "github.com/kishaningithub/hugo-creative-portfolio-theme",
			}

			if _, ok := themeMap[site.Theme]; ok {
				// For simplicity, we'll use a basic theme for now
				// In production, you'd clone the actual theme repository from themeMap
				cmd := exec.Command("git", "clone", "--depth", "1", "https://github.com/theNewDynamic/gohugo-theme-ananke.git", filepath.Join(themesDir, "ananke"))
				cmd.Dir = siteDir
				cmd.Run()

				// Update config to use the actual cloned theme
				config = strings.Replace(config, fmt.Sprintf("theme = \"%s\"", site.Theme), "theme = \"ananke\"", 1)
				os.WriteFile(configPath, []byte(config), 0644)
			}
		}

		// Save site metadata
		metaPath := filepath.Join(siteDir, "site.json")
		metaData, _ := json.Marshal(site)
		os.WriteFile(metaPath, metaData, 0644)

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		json.NewEncoder(w).Encode(site)
	}
}

func HandleHugoSitesBuild() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		id := vars["id"]

		// Check if site exists in extension storage
		siteDir := filepath.Join(".data", "storage", "ext", "hugo", "sites", id)
		if _, err := os.Stat(siteDir); os.IsNotExist(err) {
			http.Error(w, "Site not found", http.StatusNotFound)
			return
		}

		// Read site metadata
		metaPath := filepath.Join(siteDir, "site.json")
		metaData, err := os.ReadFile(metaPath)
		if err != nil {
			http.Error(w, "Failed to read site metadata", http.StatusInternalServerError)
			return
		}

		var site HugoSite
		if err := json.Unmarshal(metaData, &site); err != nil {
			http.Error(w, "Invalid site metadata", http.StatusInternalServerError)
			return
		}

		// Create public directory for built site
		publicDir := filepath.Join(siteDir, "public")
		os.RemoveAll(publicDir) // Clean previous build

		// Build the site
		hugoPath := filepath.Join(os.Getenv("HOME"), "bin", "hugo")
		if _, err := os.Stat(hugoPath); os.IsNotExist(err) {
			hugoPath = "hugo"
		}

		startTime := time.Now()
		cmd := exec.Command(hugoPath)
		cmd.Dir = siteDir

		output, err := cmd.CombinedOutput()
		if err != nil {
			result := map[string]interface{}{
				"id":      id,
				"status":  "error",
				"message": fmt.Sprintf("Build failed: %s", output),
			}
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(result)
			return
		}

		// Update site metadata
		site.Status = "published"
		site.LastBuild = time.Now().Format("Jan 2, 2006 3:04 PM")
		site.BuildTime = fmt.Sprintf("%.2fs", time.Since(startTime).Seconds())

		// Calculate site size
		var totalSize int64
		filepath.Walk(publicDir, func(path string, info os.FileInfo, err error) error {
			if err == nil && !info.IsDir() {
				totalSize += info.Size()
			}
			return nil
		})
		site.Size = fmt.Sprintf("%.2f MB", float64(totalSize)/(1024*1024))

		// Count pages
		var pageCount int
		filepath.Walk(publicDir, func(path string, info os.FileInfo, err error) error {
			if err == nil && !info.IsDir() && strings.HasSuffix(path, ".html") {
				pageCount++
			}
			return nil
		})
		site.Pages = pageCount

		// Save updated metadata
		site.UpdatedAt = time.Now()
		metaData, _ = json.Marshal(site)
		os.WriteFile(metaPath, metaData, 0644)

		// Set up public file serving
		// Copy built files to public serving directory
		// Using ./.data/storage/ext/hugo/public/{site-id} for served content
		publicServeDir := filepath.Join(".data", "storage", "ext", "hugo", "public", id)
		os.RemoveAll(publicServeDir)
		os.MkdirAll(filepath.Dir(publicServeDir), 0755)

		// Copy public files to serving directory
		copyDir(publicDir, publicServeDir)

		// Get the port from environment or use default
		port := os.Getenv("PORT")
		if port == "" {
			port = "8080"
		}

		result := map[string]interface{}{
			"id":        id,
			"status":    "completed",
			"message":   "Build successful",
			"buildTime": site.BuildTime,
			"pages":     site.Pages,
			"size":      site.Size,
			"url":       fmt.Sprintf("/storage/ext/hugo/public/%s/", id),
			"viewUrl":   fmt.Sprintf("http://localhost:%s/storage/ext/hugo/public/%s/", port, id),
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(result)
	}
}

func HandleHugoSitesDelete() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		id := vars["id"]

		// Remove site directory from extension storage
		siteDir := filepath.Join(".data", "storage", "ext", "hugo", "sites", id)
		if err := os.RemoveAll(siteDir); err != nil {
			http.Error(w, "Failed to delete site", http.StatusInternalServerError)
			return
		}

		// Remove public files
		publicDir := filepath.Join(".data", "storage", "ext", "hugo", "public", id)
		os.RemoveAll(publicDir)

		w.WriteHeader(http.StatusNoContent)
	}
}

func HandleHugoStats() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		sitesDir := filepath.Join(".data", "storage", "ext", "hugo", "sites")

		stats := HugoStats{
			TotalSites:  0,
			ActiveSites: 0,
			TotalBuilds: 0,
			StorageUsed: "0 MB",
		}

		// Check if directory exists
		if _, err := os.Stat(sitesDir); !os.IsNotExist(err) {
			entries, _ := os.ReadDir(sitesDir)
			stats.TotalSites = len(entries)

			// Count active sites and calculate storage
			var totalSize int64
			for _, entry := range entries {
				if entry.IsDir() {
					metaPath := filepath.Join(sitesDir, entry.Name(), "site.json")
					if metaData, err := os.ReadFile(metaPath); err == nil {
						var site HugoSite
						if err := json.Unmarshal(metaData, &site); err == nil {
							if site.Status == "published" {
								stats.ActiveSites++
							}
						}
					}

					// Calculate directory size
					filepath.Walk(filepath.Join(sitesDir, entry.Name()), func(path string, info os.FileInfo, err error) error {
						if err == nil && !info.IsDir() {
							totalSize += info.Size()
						}
						return nil
					})
				}
			}

			stats.StorageUsed = fmt.Sprintf("%.2f MB", float64(totalSize)/(1024*1024))
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(stats)
	}
}

// Hugo File Management Handlers

type FileNode struct {
	Name     string     `json:"name"`
	Path     string     `json:"path"`
	Type     string     `json:"type"` // "file" or "directory"
	Children []FileNode `json:"children,omitempty"`
	Size     int64      `json:"size,omitempty"`
	Modified time.Time  `json:"modified,omitempty"`
}

func HandleHugoSiteFiles() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		siteID := vars["id"]

		// Get site directory
		siteDir := filepath.Join(".data", "storage", "ext", "hugo", "sites", siteID)
		if _, err := os.Stat(siteDir); os.IsNotExist(err) {
			http.Error(w, "Site not found", http.StatusNotFound)
			return
		}

		// Get the path parameter for subdirectory navigation
		requestedPath := r.URL.Query().Get("path")
		fullPath := filepath.Join(siteDir, requestedPath)

		// Security check - ensure the path doesn't escape the site directory
		if !strings.HasPrefix(fullPath, siteDir) {
			http.Error(w, "Invalid path", http.StatusBadRequest)
			return
		}

		// Build file tree for the requested path
		fileTree, err := buildFileTree(fullPath, siteDir)
		if err != nil {
			http.Error(w, "Failed to read directory", http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(fileTree)
	}
}

func HandleHugoFileRead() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		siteID := vars["id"]

		// Get file path from request
		var reqData struct {
			Path string `json:"path"`
		}
		if err := json.NewDecoder(r.Body).Decode(&reqData); err != nil {
			http.Error(w, "Invalid request", http.StatusBadRequest)
			return
		}

		// Build full path
		siteDir := filepath.Join(".data", "storage", "ext", "hugo", "sites", siteID)
		fullPath := filepath.Join(siteDir, reqData.Path)

		// Security check
		if !strings.HasPrefix(fullPath, siteDir) {
			http.Error(w, "Invalid path", http.StatusBadRequest)
			return
		}

		// Read file content
		content, err := os.ReadFile(fullPath)
		if err != nil {
			http.Error(w, "Failed to read file", http.StatusInternalServerError)
			return
		}

		// Get file info
		info, err := os.Stat(fullPath)
		if err != nil {
			http.Error(w, "Failed to get file info", http.StatusInternalServerError)
			return
		}

		response := map[string]interface{}{
			"path":     reqData.Path,
			"content":  string(content),
			"size":     info.Size(),
			"modified": info.ModTime(),
			"mode":     detectFileMode(reqData.Path),
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(response)
	}
}

func HandleHugoFileSave() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		siteID := vars["id"]

		// Get file path and content from request
		var reqData struct {
			Path    string `json:"path"`
			Content string `json:"content"`
		}
		if err := json.NewDecoder(r.Body).Decode(&reqData); err != nil {
			http.Error(w, "Invalid request", http.StatusBadRequest)
			return
		}

		// Build full path
		siteDir := filepath.Join(".data", "storage", "ext", "hugo", "sites", siteID)
		fullPath := filepath.Join(siteDir, reqData.Path)

		// Security check
		if !strings.HasPrefix(fullPath, siteDir) {
			http.Error(w, "Invalid path", http.StatusBadRequest)
			return
		}

		// Write file content
		if err := os.WriteFile(fullPath, []byte(reqData.Content), 0644); err != nil {
			http.Error(w, "Failed to save file", http.StatusInternalServerError)
			return
		}

		response := map[string]interface{}{
			"success": true,
			"path":    reqData.Path,
			"message": "File saved successfully",
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(response)
	}
}

func HandleHugoFileCreate() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		siteID := vars["id"]

		// Get file path and content from request
		var reqData struct {
			Path    string `json:"path"`
			Content string `json:"content"`
			IsDir   bool   `json:"isDir"`
		}
		if err := json.NewDecoder(r.Body).Decode(&reqData); err != nil {
			http.Error(w, "Invalid request", http.StatusBadRequest)
			return
		}

		// Build full path
		siteDir := filepath.Join(".data", "storage", "ext", "hugo", "sites", siteID)
		fullPath := filepath.Join(siteDir, reqData.Path)

		// Security check
		if !strings.HasPrefix(fullPath, siteDir) {
			http.Error(w, "Invalid path", http.StatusBadRequest)
			return
		}

		// Check if already exists
		if _, err := os.Stat(fullPath); err == nil {
			http.Error(w, "File or directory already exists", http.StatusConflict)
			return
		}

		if reqData.IsDir {
			// Create directory
			if err := os.MkdirAll(fullPath, 0755); err != nil {
				http.Error(w, "Failed to create directory", http.StatusInternalServerError)
				return
			}
		} else {
			// Ensure parent directory exists
			parentDir := filepath.Dir(fullPath)
			if err := os.MkdirAll(parentDir, 0755); err != nil {
				http.Error(w, "Failed to create parent directory", http.StatusInternalServerError)
				return
			}

			// Create file
			if err := os.WriteFile(fullPath, []byte(reqData.Content), 0644); err != nil {
				http.Error(w, "Failed to create file", http.StatusInternalServerError)
				return
			}
		}

		response := map[string]interface{}{
			"success": true,
			"path":    reqData.Path,
			"message": "Created successfully",
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(response)
	}
}

func HandleHugoFileDelete() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		siteID := vars["id"]

		// Get file path from request
		var reqData struct {
			Path string `json:"path"`
		}
		if err := json.NewDecoder(r.Body).Decode(&reqData); err != nil {
			http.Error(w, "Invalid request", http.StatusBadRequest)
			return
		}

		// Build full path
		siteDir := filepath.Join(".data", "storage", "ext", "hugo", "sites", siteID)
		fullPath := filepath.Join(siteDir, reqData.Path)

		// Security check
		if !strings.HasPrefix(fullPath, siteDir) {
			http.Error(w, "Invalid path", http.StatusBadRequest)
			return
		}

		// Delete file or directory
		if err := os.RemoveAll(fullPath); err != nil {
			http.Error(w, "Failed to delete", http.StatusInternalServerError)
			return
		}

		response := map[string]interface{}{
			"success": true,
			"path":    reqData.Path,
			"message": "Deleted successfully",
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(response)
	}
}

// Helper function to build file tree
func buildFileTree(path string, baseDir string) ([]FileNode, error) {
	var nodes []FileNode

	entries, err := os.ReadDir(path)
	if err != nil {
		return nil, err
	}

	for _, entry := range entries {
		// Skip hidden files and Hugo's public directory
		if strings.HasPrefix(entry.Name(), ".") || entry.Name() == "public" {
			continue
		}

		info, err := entry.Info()
		if err != nil {
			continue
		}

		relPath, _ := filepath.Rel(baseDir, filepath.Join(path, entry.Name()))

		node := FileNode{
			Name:     entry.Name(),
			Path:     relPath,
			Modified: info.ModTime(),
		}

		if entry.IsDir() {
			node.Type = "directory"
			// Recursively get children for directories
			childPath := filepath.Join(path, entry.Name())
			children, _ := buildFileTree(childPath, baseDir)
			node.Children = children
		} else {
			node.Type = "file"
			node.Size = info.Size()
		}

		nodes = append(nodes, node)
	}

	return nodes, nil
}

// Helper function to detect file mode for syntax highlighting
func detectFileMode(path string) string {
	ext := filepath.Ext(path)
	switch ext {
	case ".md", ".markdown":
		return "markdown"
	case ".html", ".htm":
		return "html"
	case ".css":
		return "css"
	case ".js":
		return "javascript"
	case ".json":
		return "json"
	case ".yaml", ".yml":
		return "yaml"
	case ".toml":
		return "toml"
	case ".xml":
		return "xml"
	default:
		// Check if it's a Hugo config file
		base := filepath.Base(path)
		if base == "config.toml" || base == "hugo.toml" {
			return "toml"
		}
		if base == "config.yaml" || base == "hugo.yaml" {
			return "yaml"
		}
		return "text"
	}
}

// Cloud Storage Extension Handlers

type CloudProvider struct {
	ID          string    `json:"id"`
	Name        string    `json:"name"`
	Type        string    `json:"type"`
	Status      string    `json:"status"`
	Endpoint    string    `json:"endpoint"`
	Region      string    `json:"region"`
	BucketCount int       `json:"bucketCount"`
	TotalSize   string    `json:"totalSize"`
	LastSync    string    `json:"lastSync"`
	CreatedAt   time.Time `json:"created_at"`
	UpdatedAt   time.Time `json:"updated_at"`
}

type CloudStorageActivity struct {
	ID        string `json:"id"`
	Action    string `json:"action"`
	Provider  string `json:"provider"`
	Resource  string `json:"resource"`
	User      string `json:"user"`
	Timestamp string `json:"timestamp"`
}

type CloudStorageStats struct {
	TotalProviders int    `json:"totalProviders"`
	ActiveSyncs    int    `json:"activeSyncs"`
	TotalStorage   string `json:"totalStorage"`
	LastActivity   string `json:"lastActivity"`
}

func HandleCloudStorageProviders() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// TODO: Fetch from database when cloud_providers table is implemented
		providers := []CloudProvider{}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(providers)
	}
}

func HandleCloudStorageAddProvider() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var provider CloudProvider
		if err := json.NewDecoder(r.Body).Decode(&provider); err != nil {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}

		// TODO: Save to database and validate credentials
		provider.ID = generateID()
		provider.Status = "pending"
		provider.CreatedAt = time.Now()
		provider.UpdatedAt = time.Now()

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		json.NewEncoder(w).Encode(provider)
	}
}

func HandleCloudStorageActivity() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// TODO: Fetch from activity log
		activities := []CloudStorageActivity{}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(activities)
	}
}

func HandleCloudStorageStats() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// TODO: Calculate from database
		stats := CloudStorageStats{
			TotalProviders: 0,
			ActiveSyncs:    0,
			TotalStorage:   "0 GB",
			LastActivity:   "No activity",
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(stats)
	}
}

// Helper function to generate IDs (temporary until proper UUID implementation)
func generateID() string {
	return time.Now().Format("20060102150405")
}

// Helper function to copy directory
func copyDir(src, dst string) error {
	return filepath.Walk(src, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}

		relPath, err := filepath.Rel(src, path)
		if err != nil {
			return err
		}

		dstPath := filepath.Join(dst, relPath)

		if info.IsDir() {
			return os.MkdirAll(dstPath, info.Mode())
		}

		srcFile, err := os.Open(path)
		if err != nil {
			return err
		}
		defer srcFile.Close()

		dstFile, err := os.Create(dstPath)
		if err != nil {
			return err
		}
		defer dstFile.Close()

		_, err = io.Copy(dstFile, srcFile)
		return err
	})
}

// Helper function to create rich example content
func createExampleContent(contentDir, siteName string) {
	// Create home page with rich content
	indexContent := `---
title: "Home"
description: "Welcome to our example Hugo blog powered by Solobase"
---

# Welcome to %s

This is an example Hugo site demonstrating the power of static site generation with Solobase. Explore our sample content to see what's possible!

## Features

- **Lightning Fast** - Static sites load instantly
- **Secure** - No database or server-side processing
- **Scalable** - Serve millions of visitors effortlessly
- **SEO Friendly** - Perfect for search engine optimization

## Recent Articles

Check out our latest blog posts below to learn more about web development, Hugo, and static site generation.
`
	indexPath := filepath.Join(contentDir, "_index.md")
	os.WriteFile(indexPath, []byte(fmt.Sprintf(indexContent, siteName)), 0644)

	// Create about page
	aboutContent := `---
title: "About"
date: %s
menu: "main"
weight: 10
---

# About This Site

This is an example Hugo site created with Solobase's Hugo extension. It demonstrates how easy it is to create and manage static websites.

## Why Hugo?

Hugo is one of the most popular open-source static site generators. With its amazing speed and flexibility, Hugo makes building websites fun again.

### Key Benefits

- **Speed**: Hugo is incredibly fast at building sites
- **Flexibility**: Works with any theme and content structure
- **Simplicity**: No databases, no plugins, no dependencies
- **Security**: Static sites are inherently secure
- **Performance**: Sites load lightning fast

## Built with Solobase

Solobase provides a complete platform for managing your Hugo sites:

- Easy site creation and management
- One-click builds and deployments
- Custom domain support
- Integrated hosting
- Theme management

## Get Started

Ready to create your own Hugo site? Click the "New Site" button in the Solobase admin panel!
`
	aboutPath := filepath.Join(contentDir, "pages", "about.md")
	os.WriteFile(aboutPath, []byte(fmt.Sprintf(aboutContent, time.Now().Format(time.RFC3339))), 0644)

	// Create multiple blog posts
	posts := []struct {
		filename string
		title    string
		tags     string
		content  string
	}{
		{
			filename: "getting-started-with-hugo.md",
			title:    "Getting Started with Hugo",
			tags:     "[\"hugo\", \"tutorial\", \"beginner\"]",
			content: `
Hugo is a fast and modern static site generator written in Go. It's designed to make website creation fun again.

## Installation

Hugo is available for various platforms. You can install it using:

- **macOS**: ` + "`brew install hugo`" + `
- **Windows**: ` + "`choco install hugo`" + `
- **Linux**: ` + "`snap install hugo`" + `

## Creating Your First Site

1. Create a new site: ` + "`hugo new site mysite`" + `
2. Add a theme: ` + "`git submodule add <theme-url> themes/<theme-name>`" + `
3. Create content: ` + "`hugo new posts/my-first-post.md`" + `
4. Start the server: ` + "`hugo server -D`" + `

## Project Structure

- **content/**: Your site's content
- **layouts/**: Template files
- **static/**: Static assets (images, CSS, JS)
- **themes/**: Hugo themes
- **config.toml**: Site configuration

Start building your site today with Hugo and Solobase!`,
		},
		{
			filename: "static-vs-dynamic-websites.md",
			title:    "Static vs Dynamic Websites: Which to Choose?",
			tags:     "[\"web-development\", \"architecture\", \"performance\"]",
			content: `
Understanding the difference between static and dynamic websites is crucial for making the right choice for your project.

## Static Websites

Static websites consist of fixed content that doesn't change unless manually updated.

### Pros:
- **Performance**: Lightning fast load times
- **Security**: No database or server-side vulnerabilities
- **Cost**: Cheap to host, can use CDNs
- **Reliability**: Less that can go wrong

### Cons:
- **Functionality**: Limited interactive features
- **Updates**: Content changes require rebuilding

## Dynamic Websites

Dynamic websites generate content on-the-fly based on user interactions and database queries.

### Pros:
- **Interactivity**: Rich user interactions
- **Personalization**: Content tailored to users
- **Real-time**: Live data updates

### Cons:
- **Performance**: Slower due to server processing
- **Security**: More attack vectors
- **Cost**: Requires server resources

## The JAMstack Approach

Modern static site generators like Hugo offer the best of both worlds through the JAMstack architecture:

- **J**avaScript for dynamic functionality
- **A**PIs for data and services
- **M**arkup prebuilt at deploy time

This approach gives you static site performance with dynamic site capabilities!`,
		},
		{
			filename: "optimizing-hugo-builds.md",
			title:    "Optimizing Your Hugo Build Performance",
			tags:     "[\"hugo\", \"performance\", \"optimization\"]",
			content: `
As your Hugo site grows, build times can increase. Here are tips to keep your builds fast.

## 1. Use Hugo's Cache

Hugo caches processed images and data:

` + "```toml" + `
[caches]
[caches.images]
dir = ":resourceDir/_gen"
maxAge = "720h"
` + "```" + `

## 2. Optimize Images

- Use Hugo's image processing
- Implement lazy loading
- Choose appropriate formats (WebP, AVIF)

## 3. Minimize Template Complexity

- Avoid nested loops when possible
- Use partialCached for static components
- Leverage Hugo's built-in functions

## 4. Content Organization

- Use page bundles for better organization
- Implement proper taxonomies
- Avoid excessive front matter

## 5. Build Configuration

Enable fast render mode during development:

` + "```bash" + `
hugo server --fastRender
` + "```" + `

## Measuring Performance

Use Hugo's built-in metrics:

` + "```bash" + `
hugo --templateMetrics --templateMetricsHints
` + "```" + `

With these optimizations, even large sites can build in seconds!`,
		},
		{
			filename: "hugo-themes-guide.md",
			title:    "A Guide to Hugo Themes",
			tags:     "[\"hugo\", \"themes\", \"design\"]",
			content: `
Choosing the right theme is crucial for your Hugo site's success. Let's explore how to work with Hugo themes.

## Finding Themes

- **Hugo Themes Gallery**: The official collection at themes.gohugo.io
- **GitHub**: Search for "hugo-theme" repositories
- **JAMstack Themes**: Curated collection of quality themes

## Installing a Theme

### Method 1: Git Submodule (Recommended)
` + "```bash" + `
git submodule add https://github.com/user/theme.git themes/theme-name
` + "```" + `

### Method 2: Direct Download
Download and extract the theme to your themes directory.

## Customizing Themes

### Override Templates
Create files in your site's layouts directory to override theme templates:

` + "```" + `
layouts/
├── _default/
│   └── single.html  # Overrides theme's single.html
└── partials/
    └── header.html  # Overrides theme's header partial
` + "```" + `

### Custom CSS
Add custom styles in ` + "`static/css/custom.css`" + ` and include in your templates.

## Creating Your Own Theme

` + "```bash" + `
hugo new theme my-theme
` + "```" + `

This creates a scaffold with:
- Basic layouts
- Example archetypes
- Configuration file

## Theme Configuration

Configure your theme in config.toml:

` + "```toml" + `
theme = "theme-name"

[params]
  author = "Your Name"
  description = "Site description"
  # Theme-specific parameters
` + "```" + `

Choose a theme that matches your content and audience!`,
		},
		{
			filename: "markdown-tips.md",
			title:    "Mastering Markdown for Hugo",
			tags:     "[\"markdown\", \"writing\", \"content\"]",
			content: `
Markdown is the heart of content creation in Hugo. Here are tips to level up your Markdown game.

## Basic Formatting

### Headers
Use ` + "`#`" + ` symbols for headers (H1-H6)

### Emphasis
- **Bold**: ` + "`**text**`" + ` or ` + "`__text__`" + `
- *Italic*: ` + "`*text*`" + ` or ` + "`_text_`" + `
- ~~Strikethrough~~: ` + "`~~text~~`" + `

## Lists

### Unordered Lists
- First item
- Second item
  - Nested item
  - Another nested item

### Ordered Lists
1. First step
2. Second step
   1. Sub-step
   2. Another sub-step

## Code Blocks

Inline code: ` + "`code`" + `

Fenced code blocks with syntax highlighting:

` + "```javascript" + `
function greet(name) {
  return ` + "`Hello, ${name}!`" + `;
}
` + "```" + `

## Links and Images

- Link: ` + "`[text](url)`" + `
- Image: ` + "`![alt text](url)`" + `
- Reference links: ` + "`[text][ref]`" + ` and ` + "`[ref]: url`" + `

## Hugo-Specific Features

### Shortcodes
` + "{{< youtube dQw4w9WgXcQ >}}" + `
` + "{{< tweet user=\"jack\" id=\"20\" >}}" + `

### Front Matter
` + "```yaml" + `
---
title: "Post Title"
date: 2024-01-01
tags: ["tag1", "tag2"]
draft: false
---
` + "```" + `

## Advanced Tips

1. Use Hugo's figure shortcode for images with captions
2. Create custom shortcodes for repeated content
3. Leverage Hugo's table of contents generation
4. Use emoji support: :smile: :rocket:

Master Markdown and create beautiful content effortlessly!`,
		},
	}

	// Create all blog posts
	for i, post := range posts {
		date := time.Now().Add(-time.Duration(i*24) * time.Hour)
		postContent := fmt.Sprintf(`---
title: "%s"
date: %s
draft: false
tags: %s
author: "Solobase Team"
summary: "Learn about %s in this comprehensive guide."
---
%s`, post.title, date.Format(time.RFC3339), post.tags, strings.ToLower(post.title), post.content)

		postPath := filepath.Join(contentDir, "posts", post.filename)
		os.WriteFile(postPath, []byte(postContent), 0644)
	}

	// Create a contact page
	contactContent := `---
title: "Contact"
date: %s
menu: "main"
weight: 20
---

# Get in Touch

We'd love to hear from you! This is an example contact page for your Hugo site.

## Contact Information

- **Email**: hello@example.com
- **Phone**: +1 (555) 123-4567
- **Address**: 123 Web Street, Internet City, WWW 12345

## Office Hours

- Monday - Friday: 9:00 AM - 5:00 PM
- Saturday: 10:00 AM - 2:00 PM
- Sunday: Closed

## Follow Us

- [Twitter](https://twitter.com)
- [GitHub](https://github.com)
- [LinkedIn](https://linkedin.com)

Feel free to reach out with any questions about Hugo, static sites, or Solobase!
`
	contactPath := filepath.Join(contentDir, "pages", "contact.md")
	os.WriteFile(contactPath, []byte(fmt.Sprintf(contactContent, time.Now().Format(time.RFC3339))), 0644)
}
