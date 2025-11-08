// Export utility functions for exporting data to CSV and JSON

export function exportToCSV(data: any[], filename: string = 'export.csv') {
	if (!data || data.length === 0) {
		console.warn('No data to export');
		return;
	}

	// Get headers from the first object
	const headers = Object.keys(data[0]);
	
	// Create CSV content
	const csvContent = [
		// Header row
		headers.join(','),
		// Data rows
		...data.map(row => 
			headers.map(header => {
				const value = row[header];
				// Handle different data types
				if (value === null || value === undefined) {
					return '';
				}
				if (typeof value === 'object') {
					return `"${JSON.stringify(value).replace(/"/g, '""')}"`;
				}
				// Escape quotes and wrap in quotes if contains comma, newline, or quotes
				const stringValue = String(value);
				if (stringValue.includes(',') || stringValue.includes('\n') || stringValue.includes('"')) {
					return `"${stringValue.replace(/"/g, '""')}"`;
				}
				return stringValue;
			}).join(',')
		)
	].join('\n');

	// Create blob and download
	const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
	downloadBlob(blob, filename);
}

export function exportToJSON(data: any[], filename: string = 'export.json') {
	if (!data || data.length === 0) {
		console.warn('No data to export');
		return;
	}

	// Create JSON content with pretty formatting
	const jsonContent = JSON.stringify(data, null, 2);
	
	// Create blob and download
	const blob = new Blob([jsonContent], { type: 'application/json;charset=utf-8;' });
	downloadBlob(blob, filename);
}

function downloadBlob(blob: Blob, filename: string) {
	// Create a temporary URL for the blob
	const url = window.URL.createObjectURL(blob);
	
	// Create a temporary anchor element and trigger download
	const link = document.createElement('a');
	link.href = url;
	link.download = filename;
	document.body.appendChild(link);
	link.click();
	
	// Clean up
	document.body.removeChild(link);
	window.URL.revokeObjectURL(url);
}

// Helper function to flatten nested objects for CSV export
export function flattenObjectsForCSV(data: any[]): any[] {
	return data.map(item => {
		const flattened: any = {};
		
		function flatten(obj: any, prefix: string = '') {
			for (const key in obj) {
				if (obj.hasOwnProperty(key)) {
					const value = obj[key];
					const newKey = prefix ? `${prefix}.${key}` : key;
					
					if (value === null || value === undefined) {
						flattened[newKey] = '';
					} else if (typeof value === 'object' && !Array.isArray(value) && !(value instanceof Date)) {
						flatten(value, newKey);
					} else if (Array.isArray(value)) {
						flattened[newKey] = JSON.stringify(value);
					} else if (value instanceof Date) {
						flattened[newKey] = value.toISOString();
					} else {
						flattened[newKey] = value;
					}
				}
			}
		}
		
		flatten(item);
		return flattened;
	});
}

// Export with format selection dialog
export function exportWithDialog(
	data: any[], 
	baseFilename: string = 'export',
	options: {
		flatten?: boolean;
		formats?: ('csv' | 'json')[];
	} = {}
) {
	const { flatten = false, formats = ['csv', 'json'] } = options;
	
	// Create a simple format selection
	const format = window.confirm('Export as CSV? (OK for CSV, Cancel for JSON)') ? 'csv' : 'json';
	
	const timestamp = new Date().toISOString().split('T')[0];
	const filename = `${baseFilename}_${timestamp}`;
	
	if (format === 'csv') {
		const dataToExport = flatten ? flattenObjectsForCSV(data) : data;
		exportToCSV(dataToExport, `${filename}.csv`);
	} else {
		exportToJSON(data, `${filename}.json`);
	}
}