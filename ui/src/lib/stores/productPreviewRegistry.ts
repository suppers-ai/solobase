import { writable } from 'svelte/store';

/**
 * Generic Product Preview Registry
 * Allows external applications to register preview URLs for specific product types
 */

interface PreviewUrlRegistry {
	[productTypeId: string]: string; // URL to the preview page
}

function createPreviewRegistry() {
	const { subscribe, update } = writable<PreviewUrlRegistry>({});

	return {
		subscribe,

		/**
		 * Register a preview URL for a specific product type
		 * @param productTypeId - The product type/template ID
		 * @param url - The URL to the preview page (can be iframe source)
		 */
		register(productTypeId: string, url: string) {
			update(registry => ({
				...registry,
				[productTypeId]: url
			}));
		},

		/**
		 * Unregister a preview URL
		 * @param productTypeId - The product type/template ID
		 */
		unregister(productTypeId: string) {
			update(registry => {
				const newRegistry = { ...registry };
				delete newRegistry[productTypeId];
				return newRegistry;
			});
		}
	};
}

export const productPreviewRegistry = createPreviewRegistry();

// Export for global access (so external apps can register)
if (typeof window !== 'undefined') {
	(window as any).__solobase_preview_registry = productPreviewRegistry;
}