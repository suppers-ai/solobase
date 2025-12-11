// Shared API types

export interface ApiResponse<T = any> {
	data?: T;
	error?: ApiError | string;
	message?: string;
	status?: number;
	success?: boolean;
	metadata?: ResponseMetadata;
}

export interface ApiError {
	code: string;
	message: string;
	details?: any;
	field?: string;
	timestamp?: string;
	request_id?: string;
}

export interface ResponseMetadata {
	page?: number;
	per_page?: number;
	total?: number;
	total_pages?: number;
	has_next?: boolean;
	has_prev?: boolean;
}

export interface PaginatedResponse<T> {
	data: T[];
	page: number;
	per_page: number;
	total: number;
	total_pages?: number;
	has_next?: boolean;
	has_prev?: boolean;
}

export interface UploadProgress {
	loaded: number;
	total: number;
	percentage: number;
}
