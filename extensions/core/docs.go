package core

import (
	"bytes"
	"encoding/json"
	"fmt"
	"sort"
	"strings"
	"text/template"
	"time"
)

// DocumentationGenerator generates documentation for extensions
type DocumentationGenerator struct {
	registry *ExtensionRegistry
}

// NewDocumentationGenerator creates a new documentation generator
func NewDocumentationGenerator(registry *ExtensionRegistry) *DocumentationGenerator {
	return &DocumentationGenerator{
		registry: registry,
	}
}

// GenerateMarkdown generates Markdown documentation for all extensions
func (dg *DocumentationGenerator) GenerateMarkdown() (string, error) {
	extensions := dg.registry.List()

	// Sort extensions by name
	sort.Slice(extensions, func(i, j int) bool {
		return extensions[i].Name < extensions[j].Name
	})

	var buf bytes.Buffer

	// Header
	buf.WriteString("# Solobase Extensions Documentation\n\n")
	buf.WriteString(fmt.Sprintf("Generated: %s\n\n", time.Now().Format(time.RFC3339)))
	buf.WriteString("## Table of Contents\n\n")

	// TOC
	for _, ext := range extensions {
		buf.WriteString(fmt.Sprintf("- [%s](#%s)\n", ext.Name, strings.ReplaceAll(ext.Name, " ", "-")))
	}
	buf.WriteString("\n---\n\n")

	// Extension details
	for _, ext := range extensions {
		if err := dg.generateExtensionDocs(&buf, ext); err != nil {
			return "", err
		}
	}

	return buf.String(), nil
}

// generateExtensionDocs generates documentation for a single extension
func (dg *DocumentationGenerator) generateExtensionDocs(buf *bytes.Buffer, metadata ExtensionMetadata) error {
	// Get full extension details
	ext, exists := dg.registry.Get(metadata.Name)
	if !exists {
		return fmt.Errorf("extension not found: %s", metadata.Name)
	}

	// Header
	buf.WriteString(fmt.Sprintf("## %s\n\n", metadata.Name))

	// Metadata
	buf.WriteString("### Overview\n\n")
	buf.WriteString(fmt.Sprintf("- **Version:** %s\n", metadata.Version))
	buf.WriteString(fmt.Sprintf("- **Author:** %s\n", metadata.Author))
	buf.WriteString(fmt.Sprintf("- **License:** %s\n", metadata.License))
	buf.WriteString(fmt.Sprintf("- **Description:** %s\n", metadata.Description))

	if metadata.Homepage != "" {
		buf.WriteString(fmt.Sprintf("- **Homepage:** [%s](%s)\n", metadata.Homepage, metadata.Homepage))
	}

	if len(metadata.Tags) > 0 {
		buf.WriteString(fmt.Sprintf("- **Tags:** %s\n", strings.Join(metadata.Tags, ", ")))
	}

	buf.WriteString("\n")

	// Dependencies
	if len(metadata.Dependencies) > 0 {
		buf.WriteString("### Dependencies\n\n")
		for _, dep := range metadata.Dependencies {
			buf.WriteString(fmt.Sprintf("- %s\n", dep))
		}
		buf.WriteString("\n")
	}

	// Configuration
	configSchema := ext.ConfigSchema()
	if configSchema != nil && len(configSchema) > 0 {
		buf.WriteString("### Configuration\n\n")
		buf.WriteString("```json\n")
		var prettyJSON bytes.Buffer
		json.Indent(&prettyJSON, configSchema, "", "  ")
		buf.Write(prettyJSON.Bytes())
		buf.WriteString("\n```\n\n")
	}

	// Permissions
	permissions := ext.RequiredPermissions()
	if len(permissions) > 0 {
		buf.WriteString("### Required Permissions\n\n")
		buf.WriteString("| Permission | Resource | Actions | Description |\n")
		buf.WriteString("|------------|----------|---------|-------------|\n")
		for _, perm := range permissions {
			buf.WriteString(fmt.Sprintf("| %s | %s | %s | %s |\n",
				perm.Name,
				perm.Resource,
				strings.Join(perm.Actions, ", "),
				perm.Description,
			))
		}
		buf.WriteString("\n")
	}

	// Database schema
	dbSchema := ext.DatabaseSchema()
	if dbSchema != "" {
		buf.WriteString("### Database Schema\n\n")
		buf.WriteString(fmt.Sprintf("Schema name: `%s`\n\n", dbSchema))
	}

	// Status information
	status, err := dg.registry.GetStatus(metadata.Name)
	if err == nil && status != nil {
		buf.WriteString("### Current Status\n\n")
		buf.WriteString(fmt.Sprintf("- **State:** %s\n", status.State))
		buf.WriteString(fmt.Sprintf("- **Enabled:** %v\n", status.Enabled))
		buf.WriteString(fmt.Sprintf("- **Loaded:** %v\n", status.Loaded))

		if status.Health != nil {
			buf.WriteString(fmt.Sprintf("- **Health:** %s\n", status.Health.Status))
		}

		buf.WriteString("\n")

		// Resources
		if status.Resources.Routes > 0 || status.Resources.Hooks > 0 {
			buf.WriteString("#### Registered Resources\n\n")
			buf.WriteString(fmt.Sprintf("- Routes: %d\n", status.Resources.Routes))
			buf.WriteString(fmt.Sprintf("- Middleware: %d\n", status.Resources.Middleware))
			buf.WriteString(fmt.Sprintf("- Hooks: %d\n", status.Resources.Hooks))
			buf.WriteString(fmt.Sprintf("- Templates: %d\n", status.Resources.Templates))
			buf.WriteString(fmt.Sprintf("- Static Assets: %d\n", status.Resources.Assets))
			buf.WriteString("\n")
		}

		// Endpoints
		if len(status.Endpoints) > 0 {
			buf.WriteString("#### API Endpoints\n\n")
			buf.WriteString("| Path | Methods | Protected | Roles | Description |\n")
			buf.WriteString("|------|---------|-----------|-------|-------------|\n")
			for _, ep := range status.Endpoints {
				buf.WriteString(fmt.Sprintf("| %s | %s | %v | %s | %s |\n",
					ep.Path,
					strings.Join(ep.Methods, ", "),
					ep.Protected,
					strings.Join(ep.Roles, ", "),
					ep.Description,
				))
			}
			buf.WriteString("\n")
		}
	}

	buf.WriteString("---\n\n")

	return nil
}

// GenerateJSON generates JSON documentation for all extensions
func (dg *DocumentationGenerator) GenerateJSON() ([]byte, error) {
	extensions := dg.registry.List()

	type ExtensionDoc struct {
		Metadata    ExtensionMetadata `json:"metadata"`
		Status      *ExtensionStatus  `json:"status,omitempty"`
		Permissions []Permission      `json:"permissions,omitempty"`
		Schema      string            `json:"database_schema,omitempty"`
		Config      json.RawMessage   `json:"config_schema,omitempty"`
	}

	docs := make([]ExtensionDoc, 0, len(extensions))

	for _, metadata := range extensions {
		ext, exists := dg.registry.Get(metadata.Name)
		if !exists {
			continue
		}

		doc := ExtensionDoc{
			Metadata:    metadata,
			Permissions: ext.RequiredPermissions(),
			Schema:      ext.DatabaseSchema(),
			Config:      ext.ConfigSchema(),
		}

		if status, err := dg.registry.GetStatus(metadata.Name); err == nil {
			doc.Status = status
		}

		docs = append(docs, doc)
	}

	return json.MarshalIndent(docs, "", "  ")
}

// GenerateOpenAPI generates OpenAPI specification for extension endpoints
func (dg *DocumentationGenerator) GenerateOpenAPI() ([]byte, error) {
	spec := map[string]interface{}{
		"openapi": "3.0.0",
		"info": map[string]interface{}{
			"title":       "Solobase Extensions API",
			"version":     "1.0.0",
			"description": "Auto-generated API documentation for Solobase extensions",
		},
		"servers": []map[string]string{
			{"url": "/ext", "description": "Extension API base path"},
		},
		"paths": make(map[string]interface{}),
	}

	paths := spec["paths"].(map[string]interface{})

	// Generate paths for each extension
	extensions := dg.registry.List()
	for _, metadata := range extensions {
		status, err := dg.registry.GetStatus(metadata.Name)
		if err != nil || status == nil {
			continue
		}

		for _, endpoint := range status.Endpoints {
			path := paths[endpoint.Path]
			if path == nil {
				path = make(map[string]interface{})
				paths[endpoint.Path] = path
			}

			pathMap := path.(map[string]interface{})

			for _, method := range endpoint.Methods {
				methodLower := strings.ToLower(method)
				pathMap[methodLower] = map[string]interface{}{
					"summary":     endpoint.Description,
					"tags":        []string{metadata.Name},
					"operationId": fmt.Sprintf("%s_%s_%s", metadata.Name, methodLower, strings.ReplaceAll(endpoint.Path, "/", "_")),
					"responses": map[string]interface{}{
						"200": map[string]interface{}{
							"description": "Success",
						},
					},
				}

				if endpoint.Protected {
					security := []map[string][]string{
						{"bearerAuth": {}},
					}
					pathMap[methodLower].(map[string]interface{})["security"] = security
				}
			}
		}
	}

	// Add security schemes
	spec["components"] = map[string]interface{}{
		"securitySchemes": map[string]interface{}{
			"bearerAuth": map[string]interface{}{
				"type":         "http",
				"scheme":       "bearer",
				"bearerFormat": "JWT",
			},
		},
	}

	return json.MarshalIndent(spec, "", "  ")
}

// GenerateREADME generates a README file for an extension
func GenerateREADME(ext Extension) (string, error) {
	tmpl := `# {{.Name}}

{{.Description}}

## Installation

Add this extension to your Solobase configuration:

` + "```yaml" + `
extensions:
  enabled:
    - {{.Name}}
` + "```" + `

## Configuration

{{if .ConfigExample}}
` + "```yaml" + `
extensions:
  config:
    {{.Name}}:
{{.ConfigExample}}
` + "```" + `
{{else}}
This extension does not require configuration.
{{end}}

## Usage

{{.Usage}}

## API Endpoints

{{range .Endpoints}}
- ` + "`{{.Method}} {{.Path}}`" + ` - {{.Description}}
{{end}}

## Permissions

This extension requires the following permissions:
{{range .Permissions}}
- ` + "`{{.Name}}`" + ` - {{.Description}}
{{end}}

## Author

{{.Author}}

## License

{{.License}}
`

	metadata := ext.Metadata()

	data := struct {
		ExtensionMetadata
		ConfigExample string
		Usage         string
		Endpoints     []struct {
			Method      string
			Path        string
			Description string
		}
		Permissions []Permission
	}{
		ExtensionMetadata: metadata,
		Permissions:       ext.RequiredPermissions(),
	}

	// Generate config example
	if schema := ext.ConfigSchema(); schema != nil && len(schema) > 0 {
		// Parse schema and generate example
		data.ConfigExample = "      # Add your configuration here"
	}

	// Default usage
	data.Usage = fmt.Sprintf("Once enabled, the %s extension will be available at `/ext/%s`",
		metadata.Name, metadata.Name)

	t, err := template.New("readme").Parse(tmpl)
	if err != nil {
		return "", err
	}

	var buf bytes.Buffer
	if err := t.Execute(&buf, data); err != nil {
		return "", err
	}

	return buf.String(), nil
}
