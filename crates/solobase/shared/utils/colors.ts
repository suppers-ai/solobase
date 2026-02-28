/**
 * Validates and formats a hex color value
 * @param color - The color value to validate/format
 * @returns A properly formatted hex color (#RRGGBB) or the default #000000
 */
export function formatHexColor(color: string | undefined | null): string {
	if (!color) return '#000000';

	// Convert to string if needed
	color = String(color);

	// Remove any whitespace
	color = color.trim();

	// If already valid, return as-is (uppercase)
	if (/^#[0-9A-Fa-f]{6}$/.test(color)) {
		return color.toUpperCase();
	}

	// If it doesn't start with #, add it
	if (!color.startsWith('#')) {
		color = '#' + color;
	}

	// Remove the # for processing
	let hex = color.substring(1);

	// Remove any non-hex characters
	hex = hex.replace(/[^0-9A-Fa-f]/g, '');

	// Handle 3-digit hex (e.g., #FFF -> #FFFFFF)
	if (hex.length === 3) {
		hex = hex.split('').map((c) => c + c).join('');
	}

	// Handle incomplete hex codes by padding or truncating
	if (hex.length < 6) {
		// Pad with zeros if too short
		hex = hex.padEnd(6, '0');
	} else if (hex.length > 6) {
		// Truncate if too long
		hex = hex.substring(0, 6);
	}

	// Validate that it's a valid hex string
	if (!/^[0-9A-Fa-f]{6}$/.test(hex)) {
		return '#000000'; // Return default if invalid
	}

	return '#' + hex.toUpperCase();
}

/**
 * Checks if a color value is a valid hex color
 * @param color - The color value to check
 * @returns True if valid hex color, false otherwise
 */
export function isValidHexColor(color: string): boolean {
	if (!color || typeof color !== 'string') return false;
	return /^#[0-9A-Fa-f]{6}$/.test(color);
}

/**
 * Convert hex color to RGB components
 */
export function hexToRgb(hex: string): { r: number; g: number; b: number } | null {
	const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
	return result
		? {
				r: parseInt(result[1], 16),
				g: parseInt(result[2], 16),
				b: parseInt(result[3], 16)
			}
		: null;
}

/**
 * Convert RGB to hex color
 */
export function rgbToHex(r: number, g: number, b: number): string {
	return (
		'#' +
		[r, g, b]
			.map((x) => {
				const hex = Math.max(0, Math.min(255, x)).toString(16);
				return hex.length === 1 ? '0' + hex : hex;
			})
			.join('')
			.toUpperCase()
	);
}

/**
 * Get contrasting text color (black or white) for a given background color
 */
export function getContrastColor(hexColor: string): string {
	const rgb = hexToRgb(hexColor);
	if (!rgb) return '#000000';

	// Calculate relative luminance
	const luminance = (0.299 * rgb.r + 0.587 * rgb.g + 0.114 * rgb.b) / 255;

	return luminance > 0.5 ? '#000000' : '#FFFFFF';
}
