/**
 * API response normalization utilities
 * Handles various response formats from the backend
 */

/**
 * Normalizes an API response to an array
 * Handles: direct array, { data: [] }, { items: [] }, { objects: [] }, { rows: [] }
 */
export function normalizeArray<T>(response: unknown): T[] {
	if (Array.isArray(response)) {
		return response;
	}

	if (response && typeof response === 'object') {
		const obj = response as Record<string, unknown>;

		// Check common array wrapper properties
		if (Array.isArray(obj.data)) return obj.data;
		if (Array.isArray(obj.items)) return obj.items;
		if (Array.isArray(obj.objects)) return obj.objects;
		if (Array.isArray(obj.rows)) return obj.rows;
		if (Array.isArray(obj.results)) return obj.results;
		if (Array.isArray(obj.records)) return obj.records;
		if (Array.isArray(obj.list)) return obj.list;
		if (Array.isArray(obj.logs)) return obj.logs;
		if (Array.isArray(obj.users)) return obj.users;
		if (Array.isArray(obj.tables)) return obj.tables;
	}

	return [];
}

/**
 * Extracts total count from paginated API response
 */
export function extractTotal(response: unknown): number {
	if (response && typeof response === 'object') {
		const obj = response as Record<string, unknown>;

		if (typeof obj.total === 'number') return obj.total;
		if (typeof obj.totalCount === 'number') return obj.totalCount;
		if (typeof obj.count === 'number') return obj.count;
		if (typeof obj.totalItems === 'number') return obj.totalItems;
		if (typeof obj.totalRecords === 'number') return obj.totalRecords;

		// If response is an array, return its length
		if (Array.isArray(response)) return response.length;

		// Check nested data
		const data = normalizeArray(response);
		return data.length;
	}

	return 0;
}

/**
 * Extracts pagination info from API response
 */
export interface PaginationInfo {
	page: number;
	pageSize: number;
	totalPages: number;
	totalItems: number;
}

export function extractPagination(response: unknown, defaultPageSize = 25): PaginationInfo {
	const totalItems = extractTotal(response);

	if (response && typeof response === 'object') {
		const obj = response as Record<string, unknown>;

		const page = typeof obj.page === 'number' ? obj.page : 1;
		const pageSize = typeof obj.pageSize === 'number' ? obj.pageSize :
			typeof obj.limit === 'number' ? obj.limit :
				typeof obj.perPage === 'number' ? obj.perPage : defaultPageSize;
		const totalPages = typeof obj.totalPages === 'number' ? obj.totalPages :
			typeof obj.pages === 'number' ? obj.pages :
				Math.ceil(totalItems / pageSize) || 1;

		return { page, pageSize, totalPages, totalItems };
	}

	return {
		page: 1,
		pageSize: defaultPageSize,
		totalPages: Math.ceil(totalItems / defaultPageSize) || 1,
		totalItems
	};
}

/**
 * Converts rows array with columns to objects
 * Useful for SQL query results: { columns: ['id', 'name'], rows: [[1, 'foo'], [2, 'bar']] }
 */
export function rowsToObjects<T = Record<string, unknown>>(
	rows: unknown[][],
	columns: string[]
): T[] {
	return rows.map(row =>
		Object.fromEntries(columns.map((col, i) => [col, row[i]])) as T
	);
}

/**
 * Safe JSON parse with fallback
 */
export function safeJsonParse<T>(value: string, fallback: T): T {
	try {
		return JSON.parse(value) as T;
	} catch {
		return fallback;
	}
}

/**
 * Check if response indicates an error
 */
export function isErrorResponse(response: unknown): boolean {
	if (response && typeof response === 'object') {
		const obj = response as Record<string, unknown>;
		return obj.error === true ||
			typeof obj.error === 'string' ||
			typeof obj.message === 'string' && obj.success === false;
	}
	return false;
}

/**
 * Extract error message from response
 */
export function extractErrorMessage(response: unknown): string | null {
	if (response && typeof response === 'object') {
		const obj = response as Record<string, unknown>;
		if (typeof obj.message === 'string') return obj.message;
		if (typeof obj.error === 'string') return obj.error;
		if (typeof obj.errorMessage === 'string') return obj.errorMessage;
	}
	return null;
}
