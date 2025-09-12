package core

import (
	"context"
	"fmt"
	"sort"
	"sync"
)

// HookRegistry manages hooks registered by extensions
type HookRegistry struct {
	mu    sync.RWMutex
	hooks map[HookType][]HookRegistration
}

// NewHookRegistry creates a new hook registry
func NewHookRegistry() *HookRegistry {
	return &HookRegistry{
		hooks: make(map[HookType][]HookRegistration),
	}
}

// Register registers a new hook
func (r *HookRegistry) Register(hook HookRegistration) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	if r.hooks[hook.Type] == nil {
		r.hooks[hook.Type] = []HookRegistration{}
	}

	// Check for duplicate hooks
	for _, existing := range r.hooks[hook.Type] {
		if existing.Extension == hook.Extension && existing.Name == hook.Name {
			return fmt.Errorf("hook %s from extension %s already registered", hook.Name, hook.Extension)
		}
	}

	// Add hook
	r.hooks[hook.Type] = append(r.hooks[hook.Type], hook)

	// Sort hooks by priority (lower priority runs first)
	sort.Slice(r.hooks[hook.Type], func(i, j int) bool {
		return r.hooks[hook.Type][i].Priority < r.hooks[hook.Type][j].Priority
	})

	return nil
}

// Unregister removes all hooks from an extension
func (r *HookRegistry) Unregister(extension string) {
	r.mu.Lock()
	defer r.mu.Unlock()

	for hookType, hooks := range r.hooks {
		filtered := []HookRegistration{}
		for _, hook := range hooks {
			if hook.Extension != extension {
				filtered = append(filtered, hook)
			}
		}
		r.hooks[hookType] = filtered
	}
}

// Execute executes hooks of a specific type
func (r *HookRegistry) Execute(ctx context.Context, hookType HookType, hookCtx *HookContext) error {
	r.mu.RLock()
	hooks := r.hooks[hookType]
	r.mu.RUnlock()

	// Execute hooks in order
	for _, hook := range hooks {
		// Check if hook should apply to this path
		if !r.shouldExecute(hook, hookCtx) {
			continue
		}

		// Set extension context
		hookCtx.Extension = hook.Extension

		// Execute hook
		if err := hook.Handler(ctx, hookCtx); err != nil {
			// Log error but continue with other hooks
			// Extensions should not break the main application flow
			return &HookExecutionError{
				Extension: hook.Extension,
				Hook:      hook.Name,
				Type:      hook.Type,
				Err:       err,
			}
		}
	}

	return nil
}

// ExecuteWithResult executes hooks and collects results
func (r *HookRegistry) ExecuteWithResult(ctx context.Context, hookType HookType, hookCtx *HookContext) ([]HookResult, error) {
	r.mu.RLock()
	hooks := r.hooks[hookType]
	r.mu.RUnlock()

	results := []HookResult{}

	for _, hook := range hooks {
		if !r.shouldExecute(hook, hookCtx) {
			continue
		}

		hookCtx.Extension = hook.Extension

		result := HookResult{
			Extension: hook.Extension,
			Hook:      hook.Name,
			Type:      hook.Type,
		}

		if err := hook.Handler(ctx, hookCtx); err != nil {
			result.Error = err
		}

		// Copy any data modifications
		if hookCtx.Data != nil {
			result.Data = make(map[string]interface{})
			for k, v := range hookCtx.Data {
				result.Data[k] = v
			}
		}

		results = append(results, result)
	}

	return results, nil
}

// GetHooks returns all hooks of a specific type
func (r *HookRegistry) GetHooks(hookType HookType) []HookRegistration {
	r.mu.RLock()
	defer r.mu.RUnlock()

	hooks := r.hooks[hookType]
	result := make([]HookRegistration, len(hooks))
	copy(result, hooks)

	return result
}

// GetExtensionHooks returns all hooks from a specific extension
func (r *HookRegistry) GetExtensionHooks(extension string) []HookRegistration {
	r.mu.RLock()
	defer r.mu.RUnlock()

	result := []HookRegistration{}

	for _, hooks := range r.hooks {
		for _, hook := range hooks {
			if hook.Extension == extension {
				result = append(result, hook)
			}
		}
	}

	return result
}

// shouldExecute checks if a hook should execute for the current context
func (r *HookRegistry) shouldExecute(hook HookRegistration, ctx *HookContext) bool {
	// If no paths specified, execute for all paths
	if len(hook.Paths) == 0 {
		return true
	}

	// Check if current path matches any hook path
	if ctx.Request != nil {
		path := ctx.Request.URL.Path
		for _, hookPath := range hook.Paths {
			if matchPath(path, hookPath) {
				return true
			}
		}
	}

	return false
}

// matchPath checks if a path matches a pattern
func matchPath(path, pattern string) bool {
	// Simple prefix matching for now
	// TODO: Implement glob or regex matching
	if pattern == "*" {
		return true
	}

	// Exact match
	if path == pattern {
		return true
	}

	// Prefix match with trailing slash
	if len(pattern) > 0 && pattern[len(pattern)-1] == '/' {
		return len(path) >= len(pattern) && path[:len(pattern)] == pattern
	}

	// Prefix match for pattern without trailing slash
	if len(path) > len(pattern) && path[len(pattern)] == '/' {
		return path[:len(pattern)] == pattern
	}

	return false
}

// HookResult contains the result of a hook execution
type HookResult struct {
	Extension string
	Hook      string
	Type      HookType
	Data      map[string]interface{}
	Error     error
}

// HookExecutionError represents an error during hook execution
type HookExecutionError struct {
	Extension string
	Hook      string
	Type      HookType
	Err       error
}

func (e *HookExecutionError) Error() string {
	return fmt.Sprintf("hook %s from extension %s failed: %v", e.Hook, e.Extension, e.Err)
}

// HookChain allows building a chain of hooks with middleware-style execution
type HookChain struct {
	hooks []HookRegistration
	index int
}

// NewHookChain creates a new hook chain
func NewHookChain(hooks []HookRegistration) *HookChain {
	return &HookChain{
		hooks: hooks,
		index: 0,
	}
}

// Next executes the next hook in the chain
func (c *HookChain) Next(ctx context.Context, hookCtx *HookContext) error {
	if c.index >= len(c.hooks) {
		return nil
	}

	hook := c.hooks[c.index]
	c.index++

	// Execute hook
	if err := hook.Handler(ctx, hookCtx); err != nil {
		return err
	}

	// Continue chain
	return c.Next(ctx, hookCtx)
}

// Reset resets the chain for reuse
func (c *HookChain) Reset() {
	c.index = 0
}
