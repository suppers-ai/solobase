// Re-export shared API types
export type {
	ApiResponse,
	ApiError,
	ResponseMetadata,
	PaginatedResponse,
	UploadProgress
} from '$shared/types';

// Frontend-specific API types
import type { ApiResponse, ApiError } from '$shared/types';

export interface ApiRequest {
	method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
	url: string;
	headers?: Record<string, string>;
	body?: any;
	params?: Record<string, any>;
	timeout?: number;
	withCredentials?: boolean;
}

export interface BatchRequest {
	requests: ApiRequest[];
	parallel?: boolean;
}

export interface BatchResponse {
	responses: ApiResponse[];
	errors?: ApiError[];
}

export interface WebSocketMessage {
	type: string;
	payload: any;
	timestamp: string;
	id?: string;
}

export interface ApiClientConfig {
	baseUrl: string;
	timeout?: number;
	headers?: Record<string, string>;
	withCredentials?: boolean;
	retryAttempts?: number;
	retryDelay?: number;
	onRequest?: (config: ApiRequest) => ApiRequest | Promise<ApiRequest>;
	onResponse?: (response: ApiResponse) => ApiResponse | Promise<ApiResponse>;
	onError?: (error: ApiError) => void;
}

// Dashboard types
export interface DashboardStats {
	totalUsers: number;
	totalFiles: number;
	totalStorage: string;
	activeUsers: number;
	recentActivity: any[];
}

// App settings (frontend view)
export interface AppSettings {
	app_name: string;
	app_url: string;
	allow_signup: boolean;
	require_email_confirmation: boolean;
	smtp_enabled: boolean;
	storage_provider: string;
	max_upload_size: number;
	maintenance_mode: boolean;
	maintenance_message?: string;
}
