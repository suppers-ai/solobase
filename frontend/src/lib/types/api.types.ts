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
	body?: unknown;
	params?: Record<string, string | number | boolean>;
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

export interface WebSocketMessage<T = unknown> {
	type: string;
	payload: T;
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

// Activity type for dashboard
export interface RecentActivity {
	id: string;
	type: string;
	description: string;
	userId?: string;
	userEmail?: string;
	createdAt: string;
}

// Dashboard types
export interface DashboardStats {
	totalUsers: number;
	totalFiles: number;
	totalStorage: string;
	activeUsers: number;
	recentActivity: RecentActivity[];
}

// App settings (frontend view)
export interface AppSettings {
	appName: string;
	appUrl: string;
	allowSignup: boolean;
	requireEmailConfirmation: boolean;
	mailerProvider: string; // none, mailgun
	mailgunDomain?: string;
	mailgunRegion?: string; // us, eu
	storageProvider: string;
	maxUploadSize: number;
	allowedFileTypes?: string;
	s3Bucket?: string;
	s3Region?: string;
	s3AccessKey?: string;
	s3SecretKey?: string;
	s3Endpoint?: string;
	maintenanceMode: boolean;
	maintenanceMessage?: string;
	notification?: string;
	sessionTimeout?: number;
	passwordMinLength?: number;
	enableApiLogs?: boolean;
	enableDebugMode?: boolean;
}

// Extension types
export interface Extension {
	name: string;
	displayName: string;
	description: string;
	version: string;
	enabled: boolean;
	category?: string;
	author?: string;
	hasUI?: boolean;
	uiPath?: string;
}

export interface ExtensionStatus {
	name: string;
	enabled: boolean;
	loaded: boolean;
	error?: string;
}
