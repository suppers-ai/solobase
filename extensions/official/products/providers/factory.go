package providers

import (
	"fmt"
	"sync"

	"github.com/suppers-ai/solobase/internal/env"
	stripeProvider "github.com/suppers-ai/solobase/extensions/official/products/providers/stripe"
)

// ProviderType represents the type of payment provider
type ProviderType string

const (
	// ProviderStripe represents Stripe payment provider
	ProviderStripe ProviderType = "stripe"
	// ProviderPayPal represents PayPal payment provider (future)
	ProviderPayPal ProviderType = "paypal"
	// ProviderSquare represents Square payment provider (future)
	ProviderSquare ProviderType = "square"
)

var (
	// providers holds singleton instances of payment providers
	providers map[ProviderType]PaymentProvider
	// mutex for thread-safe access to providers map
	providersMutex sync.RWMutex
)

func init() {
	providers = make(map[ProviderType]PaymentProvider)
}

// GetProvider returns a payment provider instance by type
// It creates and caches provider instances on first request
func GetProvider(providerType ProviderType) (PaymentProvider, error) {
	providersMutex.RLock()
	if provider, exists := providers[providerType]; exists {
		providersMutex.RUnlock()
		return provider, nil
	}
	providersMutex.RUnlock()

	// Provider doesn't exist, create it
	providersMutex.Lock()
	defer providersMutex.Unlock()

	// Double-check in case another goroutine created it
	if provider, exists := providers[providerType]; exists {
		return provider, nil
	}

	// Create the provider based on type
	var provider PaymentProvider
	switch providerType {
	case ProviderStripe:
		provider = stripeProvider.New()
	case ProviderPayPal:
		// TODO: Implement PayPal provider
		return nil, fmt.Errorf("PayPal provider not yet implemented")
	case ProviderSquare:
		// TODO: Implement Square provider
		return nil, fmt.Errorf("Square provider not yet implemented")
	default:
		return nil, fmt.Errorf("unknown provider type: %s", providerType)
	}

	// Cache the provider instance
	providers[providerType] = provider
	return provider, nil
}

// GetProviderByString returns a provider by string name
func GetProviderByString(providerName string) (PaymentProvider, error) {
	return GetProvider(ProviderType(providerName))
}

// GetConfiguredProviderType returns the provider type from environment variable
// Defaults to Stripe if not set or invalid
func GetConfiguredProviderType() ProviderType {
	providerEnv := env.GetEnv("PAYMENT_PROVIDER")
	if providerEnv == "" {
		// Default to Stripe if not set
		return ProviderStripe
	}

	// Validate the provider type
	switch ProviderType(providerEnv) {
	case ProviderStripe, ProviderPayPal, ProviderSquare:
		return ProviderType(providerEnv)
	default:
		// Default to Stripe for unknown types
		return ProviderStripe
	}
}

// GetDefaultProvider returns the default configured payment provider
// It uses the PAYMENT_PROVIDER environment variable, defaulting to Stripe
func GetDefaultProvider() (PaymentProvider, error) {
	// Get the configured provider type
	providerType := GetConfiguredProviderType()

	// Try to get the configured provider
	provider, err := GetProvider(providerType)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize %s provider: %w", providerType, err)
	}

	// Check if the provider is enabled
	if !provider.IsEnabled() {
		return nil, fmt.Errorf("%s provider is not configured properly", providerType)
	}

	return provider, nil
}

// ListAvailableProviders returns a list of configured and enabled providers
func ListAvailableProviders() []string {
	var available []string

	// Check Stripe
	if provider, err := GetProvider(ProviderStripe); err == nil && provider.IsEnabled() {
		available = append(available, string(ProviderStripe))
	}

	// Check PayPal when implemented
	// if provider, err := GetProvider(ProviderPayPal); err == nil && provider.IsEnabled() {
	//     available = append(available, string(ProviderPayPal))
	// }

	return available
}

// ResetProviders clears the provider cache (useful for testing)
func ResetProviders() {
	providersMutex.Lock()
	defer providersMutex.Unlock()
	providers = make(map[ProviderType]PaymentProvider)
}
