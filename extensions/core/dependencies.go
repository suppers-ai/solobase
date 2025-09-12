package core

import (
	"fmt"
	"sort"
)

// DependencyResolver resolves extension dependencies
type DependencyResolver struct {
	extensions map[string]ExtensionMetadata
	resolved   []string
	unresolved []string
}

// NewDependencyResolver creates a new dependency resolver
func NewDependencyResolver() *DependencyResolver {
	return &DependencyResolver{
		extensions: make(map[string]ExtensionMetadata),
		resolved:   []string{},
		unresolved: []string{},
	}
}

// AddExtension adds an extension to the resolver
func (dr *DependencyResolver) AddExtension(metadata ExtensionMetadata) {
	dr.extensions[metadata.Name] = metadata
}

// Resolve resolves dependencies and returns the load order
func (dr *DependencyResolver) Resolve() ([]string, error) {
	dr.resolved = []string{}
	dr.unresolved = []string{}

	// Get all extension names
	names := make([]string, 0, len(dr.extensions))
	for name := range dr.extensions {
		names = append(names, name)
	}

	// Sort for consistent ordering
	sort.Strings(names)

	// Resolve each extension
	for _, name := range names {
		if err := dr.resolveDependencies(name); err != nil {
			return nil, err
		}
	}

	return dr.resolved, nil
}

// resolveDependencies recursively resolves dependencies for an extension
func (dr *DependencyResolver) resolveDependencies(name string) error {
	// Check if already resolved
	if dr.isResolved(name) {
		return nil
	}

	// Check for circular dependency
	if dr.isUnresolved(name) {
		return fmt.Errorf("circular dependency detected for extension: %s", name)
	}

	// Mark as unresolved (being processed)
	dr.unresolved = append(dr.unresolved, name)

	// Get extension metadata
	metadata, exists := dr.extensions[name]
	if !exists {
		return fmt.Errorf("extension not found: %s", name)
	}

	// Resolve dependencies first
	for _, dep := range metadata.Dependencies {
		if err := dr.resolveDependencies(dep); err != nil {
			return fmt.Errorf("failed to resolve dependency %s for %s: %w", dep, name, err)
		}
	}

	// Remove from unresolved
	dr.removeUnresolved(name)

	// Add to resolved
	dr.resolved = append(dr.resolved, name)

	return nil
}

// isResolved checks if an extension is already resolved
func (dr *DependencyResolver) isResolved(name string) bool {
	for _, resolved := range dr.resolved {
		if resolved == name {
			return true
		}
	}
	return false
}

// isUnresolved checks if an extension is currently being resolved
func (dr *DependencyResolver) isUnresolved(name string) bool {
	for _, unresolved := range dr.unresolved {
		if unresolved == name {
			return true
		}
	}
	return false
}

// removeUnresolved removes an extension from the unresolved list
func (dr *DependencyResolver) removeUnresolved(name string) {
	for i, unresolved := range dr.unresolved {
		if unresolved == name {
			dr.unresolved = append(dr.unresolved[:i], dr.unresolved[i+1:]...)
			return
		}
	}
}

// ValidateDependencies validates that all dependencies can be satisfied
func (dr *DependencyResolver) ValidateDependencies() error {
	for name, metadata := range dr.extensions {
		for _, dep := range metadata.Dependencies {
			if _, exists := dr.extensions[dep]; !exists {
				return fmt.Errorf("extension %s depends on missing extension: %s", name, dep)
			}
		}
	}
	return nil
}

// GetDependencyGraph returns a visual representation of dependencies
func (dr *DependencyResolver) GetDependencyGraph() map[string][]string {
	graph := make(map[string][]string)
	for name, metadata := range dr.extensions {
		graph[name] = metadata.Dependencies
	}
	return graph
}

// GetDependents returns extensions that depend on a given extension
func (dr *DependencyResolver) GetDependents(extension string) []string {
	var dependents []string
	for name, metadata := range dr.extensions {
		for _, dep := range metadata.Dependencies {
			if dep == extension {
				dependents = append(dependents, name)
				break
			}
		}
	}
	return dependents
}

// CanDisable checks if an extension can be disabled (no active dependents)
func (dr *DependencyResolver) CanDisable(extension string, enabledExtensions map[string]bool) bool {
	dependents := dr.GetDependents(extension)
	for _, dep := range dependents {
		if enabled, exists := enabledExtensions[dep]; exists && enabled {
			return false
		}
	}
	return true
}

// GetLoadOrder returns the optimal load order considering dependencies
func (dr *DependencyResolver) GetLoadOrder(extensions []ExtensionMetadata) ([]string, error) {
	// Clear and add all extensions
	dr.extensions = make(map[string]ExtensionMetadata)
	for _, ext := range extensions {
		dr.AddExtension(ext)
	}

	// Validate dependencies
	if err := dr.ValidateDependencies(); err != nil {
		return nil, err
	}

	// Resolve and return order
	return dr.Resolve()
}

// VersionConstraint represents a version constraint
type VersionConstraint struct {
	Extension  string
	MinVersion string
	MaxVersion string
}

// CheckVersionConstraints checks if version constraints are satisfied
func CheckVersionConstraints(extensions map[string]ExtensionMetadata, constraints []VersionConstraint) error {
	for _, constraint := range constraints {
		ext, exists := extensions[constraint.Extension]
		if !exists {
			return fmt.Errorf("required extension not found: %s", constraint.Extension)
		}

		// Simple version comparison (can be enhanced with semver)
		if constraint.MinVersion != "" && ext.Version < constraint.MinVersion {
			return fmt.Errorf("extension %s version %s is below minimum required %s",
				constraint.Extension, ext.Version, constraint.MinVersion)
		}

		if constraint.MaxVersion != "" && ext.Version > constraint.MaxVersion {
			return fmt.Errorf("extension %s version %s is above maximum allowed %s",
				constraint.Extension, ext.Version, constraint.MaxVersion)
		}
	}
	return nil
}
