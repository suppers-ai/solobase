package core

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestDependencyResolver(t *testing.T) {
	resolver := NewDependencyResolver()

	// Add extensions with dependencies
	resolver.AddExtension(ExtensionMetadata{
		Name:         "base",
		Version:      "1.0.0",
		Dependencies: []string{},
	})

	resolver.AddExtension(ExtensionMetadata{
		Name:         "auth",
		Version:      "1.0.0",
		Dependencies: []string{"base"},
	})

	resolver.AddExtension(ExtensionMetadata{
		Name:         "api",
		Version:      "1.0.0",
		Dependencies: []string{"base", "auth"},
	})

	resolver.AddExtension(ExtensionMetadata{
		Name:         "ui",
		Version:      "1.0.0",
		Dependencies: []string{"api"},
	})

	// Validate dependencies
	err := resolver.ValidateDependencies()
	assert.NoError(t, err)

	// Resolve load order
	order, err := resolver.Resolve()
	assert.NoError(t, err)
	assert.Equal(t, []string{"base", "auth", "api", "ui"}, order)

	// Test getting dependents
	authDependents := resolver.GetDependents("auth")
	assert.Contains(t, authDependents, "api")

	// Test can disable
	enabledExtensions := map[string]bool{
		"base": true,
		"auth": true,
		"api":  false,
		"ui":   false,
	}

	// Can't disable auth because api is enabled
	enabledExtensions["api"] = true
	canDisable := resolver.CanDisable("auth", enabledExtensions)
	assert.False(t, canDisable)

	// Can disable auth if api is disabled
	enabledExtensions["api"] = false
	canDisable = resolver.CanDisable("auth", enabledExtensions)
	assert.True(t, canDisable)
}

func TestCircularDependency(t *testing.T) {
	resolver := NewDependencyResolver()

	// Create circular dependency
	resolver.AddExtension(ExtensionMetadata{
		Name:         "ext1",
		Version:      "1.0.0",
		Dependencies: []string{"ext2"},
	})

	resolver.AddExtension(ExtensionMetadata{
		Name:         "ext2",
		Version:      "1.0.0",
		Dependencies: []string{"ext3"},
	})

	resolver.AddExtension(ExtensionMetadata{
		Name:         "ext3",
		Version:      "1.0.0",
		Dependencies: []string{"ext1"}, // Circular!
	})

	// Should detect circular dependency
	_, err := resolver.Resolve()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "circular dependency")
}

func TestMissingDependency(t *testing.T) {
	resolver := NewDependencyResolver()

	resolver.AddExtension(ExtensionMetadata{
		Name:         "ext1",
		Version:      "1.0.0",
		Dependencies: []string{"missing-ext"},
	})

	// Should detect missing dependency
	err := resolver.ValidateDependencies()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "missing extension")
}

func TestVersionConstraints(t *testing.T) {
	extensions := map[string]ExtensionMetadata{
		"base": {
			Name:    "base",
			Version: "1.0.0",
		},
		"auth": {
			Name:    "auth",
			Version: "2.0.0",
		},
	}

	// Test min version constraint
	constraints := []VersionConstraint{
		{
			Extension:  "base",
			MinVersion: "1.0.0",
		},
		{
			Extension:  "auth",
			MinVersion: "1.5.0",
		},
	}

	err := CheckVersionConstraints(extensions, constraints)
	assert.NoError(t, err)

	// Test failing min version
	constraints = []VersionConstraint{
		{
			Extension:  "base",
			MinVersion: "2.0.0",
		},
	}

	err = CheckVersionConstraints(extensions, constraints)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "below minimum required")

	// Test max version constraint
	constraints = []VersionConstraint{
		{
			Extension:  "auth",
			MaxVersion: "3.0.0",
		},
	}

	err = CheckVersionConstraints(extensions, constraints)
	assert.NoError(t, err)

	// Test failing max version
	constraints = []VersionConstraint{
		{
			Extension:  "auth",
			MaxVersion: "1.5.0",
		},
	}

	err = CheckVersionConstraints(extensions, constraints)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "above maximum allowed")
}

func TestComplexDependencyChain(t *testing.T) {
	resolver := NewDependencyResolver()

	// Create complex dependency chain
	extensions := []ExtensionMetadata{
		{Name: "core", Dependencies: []string{}},
		{Name: "logging", Dependencies: []string{"core"}},
		{Name: "database", Dependencies: []string{"core", "logging"}},
		{Name: "auth", Dependencies: []string{"database"}},
		{Name: "api", Dependencies: []string{"auth", "logging"}},
		{Name: "websocket", Dependencies: []string{"api"}},
		{Name: "ui", Dependencies: []string{"api", "websocket"}},
		{Name: "admin", Dependencies: []string{"ui", "auth"}},
	}

	for _, ext := range extensions {
		resolver.AddExtension(ext)
	}

	// Validate and resolve
	err := resolver.ValidateDependencies()
	assert.NoError(t, err)

	order, err := resolver.Resolve()
	assert.NoError(t, err)

	// Verify order satisfies dependencies
	resolved := make(map[string]bool)
	for _, name := range order {
		// Find extension
		var ext ExtensionMetadata
		for _, e := range extensions {
			if e.Name == name {
				ext = e
				break
			}
		}

		// Check all dependencies are resolved
		for _, dep := range ext.Dependencies {
			assert.True(t, resolved[dep],
				"Extension %s loaded before dependency %s", name, dep)
		}

		resolved[name] = true
	}

	// All extensions should be resolved
	assert.Equal(t, len(extensions), len(order))
}
