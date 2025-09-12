package core

import (
	"encoding/json"
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/suppers-ai/logger"
)

func TestDocumentationGenerator(t *testing.T) {
	// Create test logger and services
	testLogger, _ := logger.New(logger.Config{
		Level:  logger.LevelDebug,
		Output: "console",
		Format: "text",
	})
	services := &ExtensionServices{}

	// Create registry and documentation generator
	registry := NewExtensionRegistry(testLogger, services)
	docGen := NewDocumentationGenerator(registry)

	// Register test extensions
	ext1 := &MockExtension{
		name:    "test-ext-1",
		version: "1.0.0",
	}
	ext2 := &MockExtension{
		name:    "test-ext-2",
		version: "2.0.0",
	}

	err := registry.Register(ext1)
	assert.NoError(t, err)
	err = registry.Register(ext2)
	assert.NoError(t, err)

	// Enable extensions to get full status
	err = registry.Enable("test-ext-1")
	assert.NoError(t, err)
	err = registry.Enable("test-ext-2")
	assert.NoError(t, err)

	// Test Markdown generation
	markdown, err := docGen.GenerateMarkdown()
	assert.NoError(t, err)
	assert.Contains(t, markdown, "# Solobase Extensions Documentation")
	assert.Contains(t, markdown, "test-ext-1")
	assert.Contains(t, markdown, "test-ext-2")
	assert.Contains(t, markdown, "## test-ext-1")
	assert.Contains(t, markdown, "## test-ext-2")
	assert.Contains(t, markdown, "Version:** 1.0.0")
	assert.Contains(t, markdown, "Version:** 2.0.0")
	assert.Contains(t, markdown, "State:** enabled")

	// Test JSON generation
	jsonDocs, err := docGen.GenerateJSON()
	assert.NoError(t, err)

	var docs []interface{}
	err = json.Unmarshal(jsonDocs, &docs)
	assert.NoError(t, err)
	assert.Len(t, docs, 2)

	// Test OpenAPI generation
	openapi, err := docGen.GenerateOpenAPI()
	assert.NoError(t, err)

	var spec map[string]interface{}
	err = json.Unmarshal(openapi, &spec)
	assert.NoError(t, err)
	assert.Equal(t, "3.0.0", spec["openapi"])
	assert.NotNil(t, spec["info"])
	assert.NotNil(t, spec["paths"])
	assert.NotNil(t, spec["components"])
}

func TestGenerateREADME(t *testing.T) {
	// Create test extension
	ext := &MockExtension{
		name:    "awesome-extension",
		version: "1.0.0",
	}

	// Generate README
	readme, err := GenerateREADME(ext)
	assert.NoError(t, err)
	assert.Contains(t, readme, "# awesome-extension")
	assert.Contains(t, readme, "## Installation")
	assert.Contains(t, readme, "## Configuration")
	assert.Contains(t, readme, "## Usage")
	assert.Contains(t, readme, "## API Endpoints")
	assert.Contains(t, readme, "## Permissions")
	assert.Contains(t, readme, "## Author")
	assert.Contains(t, readme, "## License")
	assert.Contains(t, readme, "Mock extension for testing")
}

func TestDocumentationWithComplexExtension(t *testing.T) {
	// Create test logger and services
	testLogger, _ := logger.New(logger.Config{
		Level:  logger.LevelDebug,
		Output: "console",
		Format: "text",
	})
	services := &ExtensionServices{}

	// Create registry and documentation generator
	registry := NewExtensionRegistry(testLogger, services)
	docGen := NewDocumentationGenerator(registry)

	// Create a complex extension with all features
	complexExt := &ComplexMockExtension{
		MockExtension: MockExtension{
			name:    "complex-extension",
			version: "3.0.0",
		},
	}

	err := registry.Register(complexExt)
	assert.NoError(t, err)
	err = registry.Enable("complex-extension")
	assert.NoError(t, err)

	// Generate documentation
	markdown, err := docGen.GenerateMarkdown()
	assert.NoError(t, err)

	// Verify all sections are present
	assert.Contains(t, markdown, "### Dependencies")
	assert.Contains(t, markdown, "### Configuration")
	assert.Contains(t, markdown, "### Required Permissions")
	assert.Contains(t, markdown, "### Database Schema")
	assert.Contains(t, markdown, "#### Migrations")
	assert.Contains(t, markdown, "#### Registered Resources")
}

// ComplexMockExtension is a mock extension with all features
type ComplexMockExtension struct {
	MockExtension
}

func (e *ComplexMockExtension) Metadata() ExtensionMetadata {
	return ExtensionMetadata{
		Name:         e.name,
		Version:      e.version,
		Description:  "Complex mock extension with all features",
		Author:       "Test",
		License:      "MIT",
		Homepage:     "https://example.com",
		Dependencies: []string{"base", "auth"},
		Tags:         []string{"test", "mock", "complex"},
	}
}

func (e *ComplexMockExtension) RequiredPermissions() []Permission {
	return []Permission{
		{
			Name:        "test.read",
			Description: "Read test data",
			Resource:    "test",
			Actions:     []string{"read"},
		},
		{
			Name:        "test.write",
			Description: "Write test data",
			Resource:    "test",
			Actions:     []string{"write", "delete"},
		},
	}
}

func (e *ComplexMockExtension) DatabaseSchema() string {
	return "complex_ext"
}

func (e *ComplexMockExtension) Migrations() []Migration {
	return []Migration{
		{
			Version:     "001",
			Description: "Initial schema",
			Extension:   e.name,
			Up:          "CREATE TABLE test (id INT);",
			Down:        "DROP TABLE test;",
		},
		{
			Version:     "002",
			Description: "Add name column",
			Extension:   e.name,
			Up:          "ALTER TABLE test ADD COLUMN name VARCHAR(255);",
			Down:        "ALTER TABLE test DROP COLUMN name;",
		},
	}
}

func (e *ComplexMockExtension) ConfigSchema() json.RawMessage {
	return json.RawMessage(`{
		"type": "object",
		"properties": {
			"enabled": {"type": "boolean"},
			"apiKey": {"type": "string"},
			"maxRequests": {"type": "number"}
		},
		"required": ["enabled"]
	}`)
}

func TestDocumentationFormatting(t *testing.T) {
	// Test that generated markdown is properly formatted
	testLogger, _ := logger.New(logger.Config{
		Level:  logger.LevelDebug,
		Output: "console",
		Format: "text",
	})
	services := &ExtensionServices{}
	registry := NewExtensionRegistry(testLogger, services)
	docGen := NewDocumentationGenerator(registry)

	ext := NewMockExtension("format-test", "1.0.0")
	registry.Register(ext)

	markdown, err := docGen.GenerateMarkdown()
	assert.NoError(t, err)

	// Check proper markdown formatting
	lines := strings.Split(markdown, "\n")

	// Headers should have proper spacing
	for i, line := range lines {
		if strings.HasPrefix(line, "#") {
			// Header should be followed by blank line or content
			if i < len(lines)-1 {
				nextLine := lines[i+1]
				if nextLine != "" && !strings.HasPrefix(nextLine, "-") && !strings.HasPrefix(nextLine, "|") {
					// If next line is content, it should not be another header
					assert.False(t, strings.HasPrefix(nextLine, "#"),
						"Headers should be properly spaced")
				}
			}
		}
	}

	// Tables should have proper format
	inTable := false
	for _, line := range lines {
		if strings.Contains(line, "|") {
			if !inTable {
				// Start of table - should have header separator next
				inTable = true
			}
		} else if inTable && line != "" && !strings.Contains(line, "---") {
			// End of table
			inTable = false
		}
	}
}
