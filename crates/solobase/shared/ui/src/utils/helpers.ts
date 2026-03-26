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

/**
 * Navigate to a URL only if not already there. Prevents redirect loops
 * by tracking redirect count in sessionStorage — stops after 3 redirects
 * within 5 seconds.
 */
export function safeRedirect(url: string): void {
	const current = window.location.pathname + window.location.hash;
	const target = url.startsWith('http') ? new URL(url).pathname : url;

	// Already on the target page
	if (current === target || window.location.href === url) {
		return;
	}

	// Track redirects to detect loops
	const key = 'sb_redirect_guard';
	const now = Date.now();
	const guard = JSON.parse(sessionStorage.getItem(key) || '{"count":0,"ts":0}');

	// Reset counter if more than 5 seconds since last redirect
	if (now - guard.ts > 5000) {
		guard.count = 0;
	}

	guard.count++;
	guard.ts = now;
	sessionStorage.setItem(key, JSON.stringify(guard));

	if (guard.count > 3) {
		console.error('Redirect loop detected — stopped after 3 redirects');
		sessionStorage.removeItem(key);
		return;
	}

	window.location.href = url;
}
