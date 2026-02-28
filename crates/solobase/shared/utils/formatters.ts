/**
 * Format bytes to human-readable string
 */
export function formatBytes(bytes: number, decimals = 2): string {
	if (bytes === 0) return '0 Bytes';

	const k = 1024;
	const dm = decimals < 0 ? 0 : decimals;
	const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB'];

	const i = Math.floor(Math.log(bytes) / Math.log(k));

	return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
}

/**
 * Format date to relative time (e.g., "2 hours ago")
 */
export function formatDate(date: string | Date | null | undefined): string {
	if (!date) return '';

	const d = new Date(date);
	const now = new Date();
	const diff = now.getTime() - d.getTime();

	// Less than 1 minute
	if (diff < 60000) {
		return 'just now';
	}

	// Less than 1 hour
	if (diff < 3600000) {
		const minutes = Math.floor(diff / 60000);
		return `${minutes} minute${minutes > 1 ? 's' : ''} ago`;
	}

	// Less than 24 hours
	if (diff < 86400000) {
		const hours = Math.floor(diff / 3600000);
		return `${hours} hour${hours > 1 ? 's' : ''} ago`;
	}

	// Less than 7 days
	if (diff < 604800000) {
		const days = Math.floor(diff / 86400000);
		return `${days} day${days > 1 ? 's' : ''} ago`;
	}

	// Default to formatted date
	return d.toLocaleDateString('en-US', {
		year: 'numeric',
		month: 'short',
		day: 'numeric',
		hour: '2-digit',
		minute: '2-digit'
	});
}

/**
 * Format duration in milliseconds to human-readable string
 */
export function formatDuration(ms: number): string {
	if (ms < 1000) return `${ms}ms`;

	const seconds = Math.floor(ms / 1000);
	const minutes = Math.floor(seconds / 60);
	const hours = Math.floor(minutes / 60);
	const days = Math.floor(hours / 24);

	if (days > 0) {
		return `${days}d ${hours % 24}h`;
	}
	if (hours > 0) {
		return `${hours}h ${minutes % 60}m`;
	}
	if (minutes > 0) {
		return `${minutes}m ${seconds % 60}s`;
	}

	return `${seconds}s`;
}

/**
 * Format large numbers to compact form (e.g., 1.5K, 2.3M)
 */
export function formatNumber(num: number | null | undefined): string {
	if (num === null || num === undefined) return '0';

	if (num >= 1000000) {
		return (num / 1000000).toFixed(1) + 'M';
	}
	if (num >= 1000) {
		return (num / 1000).toFixed(1) + 'K';
	}

	return num.toString();
}

/**
 * Truncate string to specified length with ellipsis
 */
export function truncate(str: string | null | undefined, length = 50): string {
	if (!str) return '';
	if (str.length <= length) return str;

	return str.substring(0, length) + '...';
}

/**
 * Format date to long form (e.g., "January 1, 2024")
 */
export function formatDateLong(date: string | Date | null | undefined): string {
	if (!date) return '';
	return new Date(date).toLocaleDateString('en-US', {
		year: 'numeric',
		month: 'long',
		day: 'numeric'
	});
}

/**
 * Format date to short form (locale default)
 */
export function formatDateShort(date: string | Date | null | undefined): string {
	if (!date) return '';
	return new Date(date).toLocaleDateString();
}

/**
 * Format date and time
 */
export function formatDateTime(date: string | Date | null | undefined): string {
	if (!date) return 'N/A';
	return new Date(date).toLocaleString('en-US', {
		year: 'numeric',
		month: 'short',
		day: 'numeric',
		hour: '2-digit',
		minute: '2-digit'
	});
}

/**
 * Format file size (alias for formatBytes)
 */
export function formatFileSize(bytes: number): string {
	return formatBytes(bytes, 2);
}
