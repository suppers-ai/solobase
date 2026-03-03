/**
 * Format a price in cents to a currency string.
 */
export function formatPrice(price?: number | null, currency?: string): string {
	if (price === undefined || price === null) return 'Free';
	return new Intl.NumberFormat('en-US', { style: 'currency', currency: currency || 'USD' }).format(price / 100);
}

/**
 * Validate that a redirect URL is safe (same-origin or relative path).
 */
export function isValidRedirectUrl(url: string): boolean {
	if (!url) return false;
	try {
		if (url.startsWith('/') && !url.startsWith('//')) return true;
		if (url.startsWith('http')) {
			const urlObj = new URL(url);
			return urlObj.origin === window.location.origin;
		}
		return false;
	} catch {
		return false;
	}
}
