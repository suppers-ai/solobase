package tenant

import (
	"fmt"
	"os"
	"path/filepath"
	"text/template"
)

// SolobaseConfig is the template data for generating per-tenant solobase.toml.
type SolobaseConfig struct {
	DatabaseType string
	DatabasePath string
	DatabaseURL  string
	StorageType  string
	StorageRoot  string
	StorageBucket string
	StorageRegion string
	StorageEndpoint string
	StoragePrefix string
	JWTSecret    string
	BindAddr     string
	Features     map[string]bool
}

const solobaseTomlTemplate = `# Auto-generated solobase config for tenant
[server]
bind = "{{.BindAddr}}"

[database]
type = "{{.DatabaseType}}"
{{- if eq .DatabaseType "sqlite"}}
path = "{{.DatabasePath}}"
{{- else}}
url = "{{.DatabaseURL}}"
{{- end}}

[storage]
type = "{{.StorageType}}"
{{- if eq .StorageType "local"}}
root = "{{.StorageRoot}}"
{{- else}}
bucket = "{{.StorageBucket}}"
region = "{{.StorageRegion}}"
{{- if .StorageEndpoint}}
endpoint = "{{.StorageEndpoint}}"
{{- end}}
{{- if .StoragePrefix}}
prefix = "{{.StoragePrefix}}"
{{- end}}
{{- end}}

[auth]
jwt_secret = "{{.JWTSecret}}"

[features]
{{- range $k, $v := .Features}}
{{$k}} = {{$v}}
{{- end}}
`

// ProvisionOverlay creates the per-tenant overlay filesystem structure
// and writes the solobase.toml config into it.
func ProvisionOverlay(dataDir, tenantID string, cfg *SolobaseConfig) (overlayDir string, err error) {
	overlayDir = filepath.Join(dataDir, "tenants", tenantID)
	upperDir := filepath.Join(overlayDir, "upper")
	workDir := filepath.Join(overlayDir, "work")
	configDir := filepath.Join(upperDir, "etc", "solobase")
	dataSubDir := filepath.Join(upperDir, "data")

	for _, dir := range []string{upperDir, workDir, configDir, dataSubDir} {
		if err := os.MkdirAll(dir, 0755); err != nil {
			return "", fmt.Errorf("create overlay dir %s: %w", dir, err)
		}
	}

	// Write solobase.toml
	tmpl, err := template.New("config").Parse(solobaseTomlTemplate)
	if err != nil {
		return "", fmt.Errorf("parse config template: %w", err)
	}

	configPath := filepath.Join(configDir, "solobase.toml")
	f, err := os.Create(configPath)
	if err != nil {
		return "", fmt.Errorf("create config file: %w", err)
	}
	defer f.Close()

	if err := tmpl.Execute(f, cfg); err != nil {
		return "", fmt.Errorf("render config template: %w", err)
	}

	return overlayDir, nil
}

// CleanupOverlay removes the per-tenant overlay directory.
func CleanupOverlay(dataDir, tenantID string) error {
	overlayDir := filepath.Join(dataDir, "tenants", tenantID)
	return os.RemoveAll(overlayDir)
}
