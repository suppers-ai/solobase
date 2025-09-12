export function formatBytes(bytes, decimals = 2) {
	if (bytes === 0) return '0 Bytes';
	
	const k = 1024;
	const dm = decimals < 0 ? 0 : decimals;
	const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB'];
	
	const i = Math.floor(Math.log(bytes) / Math.log(k));
	
	return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
}

export function formatDate(date) {
	if (!date) return '';
	
	const d = new Date(date);
	const now = new Date();
	const diff = now - d;
	
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

export function formatDuration(ms) {
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

export function formatNumber(num) {
	if (num === null || num === undefined) return '0';
	
	if (num >= 1000000) {
		return (num / 1000000).toFixed(1) + 'M';
	}
	if (num >= 1000) {
		return (num / 1000).toFixed(1) + 'K';
	}
	
	return num.toString();
}

export function truncate(str, length = 50) {
	if (!str) return '';
	if (str.length <= length) return str;
	
	return str.substring(0, length) + '...';
}