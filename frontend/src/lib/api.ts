import type {
	AuthUser, UserResponse, LoginRequest, LoginResponse, SignupRequest,
	DatabaseTable, DatabaseColumn, QueryResult,
	StorageObject, StorageBucket,
	AppSettings, DashboardStats,
	ApiResponse, PaginatedResponse,
	Extension, ExtensionStatus,
	Webhook, WebhookCreateRequest,
	AnalyticsStats, AnalyticsPageview, AnalyticsDailyStats, AnalyticsEvent
} from './types';
import { ErrorHandler } from './utils/error-handler';

const API_BASE = '/api';

// Re-export ErrorHandler for convenience
export { ErrorHandler };

/**
 * Authenticated fetch wrapper that automatically includes credentials (cookies)
 * Use this instead of raw fetch() for API calls that require authentication
 */
export async function authFetch(url: string, options: RequestInit = {}): Promise<Response> {
	return fetch(url, {
		...options,
		credentials: 'include',
		headers: {
			'Content-Type': 'application/json',
			...options.headers
		}
	});
}

class ApiClient {
	// No longer storing token in memory or localStorage
	// Authentication is handled via httpOnly cookies

	constructor() {
		// Cookies are automatically sent with requests
	}

	// Roles should now be fetched from the server via /api/auth/me
	// since we can't decode httpOnly cookies on client side
	async getCurrentUserRoles(): Promise<string[]> {
		const result = await this.getCurrentUser();
		if (result.error || !result.data) return [];
		// Roles would need to be added to user response from backend
		return result.data.roles || [];
	}

	private async request<T>(
		endpoint: string,
		options: RequestInit = {}
	): Promise<ApiResponse<T>> {
		const headers: HeadersInit = {
			'Content-Type': 'application/json',
			...options.headers
		};

		// No need to add Authorization header - cookies are sent automatically
		// The 'include' credentials option ensures cookies are sent

		try {
			const response = await fetch(`${API_BASE}${endpoint}`, {
				...options,
				headers,
				credentials: 'include'
			});

			// Check if we have a response body
			const text = await response.text();
			if (!text) {
				if (!response.ok) {
					throw new Error(`HTTP ${response.status}: Empty response`);
				}
				// Empty successful response
				return { data: {} as T };
			}

			// Try to parse JSON
			let data;
			try {
				data = JSON.parse(text);
			} catch {
				throw new Error(`Invalid JSON response: ${text.substring(0, 100)}`);
			}

			if (!response.ok) {
				throw new Error(data.error || `HTTP ${response.status}`);
			}

			return { data: data as T };
		} catch (error) {
			// Use ErrorHandler but don't show toast by default (let caller decide)
			const message = ErrorHandler.handle(error, false);
			return { error: message };
		}
	}

	// Auth methods
	async login(request: LoginRequest): Promise<ApiResponse<LoginResponse>> {
		return this.request<LoginResponse>('/auth/login', {
			method: 'POST',
			body: JSON.stringify(request)
		});
	}

	async logout(): Promise<ApiResponse<void>> {
		return this.request<void>('/auth/logout', {
			method: 'POST'
		});
	}

	async signup(request: SignupRequest): Promise<ApiResponse<AuthUser>> {
		return this.request<AuthUser>('/auth/signup', {
			method: 'POST',
			body: JSON.stringify(request)
		});
	}

	async getCurrentUser(): Promise<ApiResponse<UserResponse>> {
		return this.request<UserResponse>('/auth/me');
	}

	// Users methods (admin)
	async getUsers(page = 1, pageSize = 20): Promise<ApiResponse<PaginatedResponse<AuthUser>>> {
		return this.request<PaginatedResponse<AuthUser>>(`/admin/users?page=${page}&pageSize=${pageSize}`);
	}

	async getUser(id: string): Promise<ApiResponse<AuthUser>> {
		return this.request<AuthUser>(`/admin/users/${id}`);
	}

	async updateUser(id: string, updates: Partial<AuthUser>): Promise<ApiResponse<AuthUser>> {
		return this.request<AuthUser>(`/admin/users/${id}`, {
			method: 'PATCH',
			body: JSON.stringify(updates)
		});
	}

	async deleteUser(id: string): Promise<ApiResponse<void>> {
		return this.request<void>(`/admin/users/${id}`, {
			method: 'DELETE'
		});
	}

	// Database methods (admin)
	async getDatabaseTables(): Promise<ApiResponse<DatabaseTable[]>> {
		return this.request<DatabaseTable[]>('/admin/database/tables');
	}

	async getTableColumns(table: string): Promise<ApiResponse<DatabaseColumn[]>> {
		return this.request<DatabaseColumn[]>(`/admin/database/tables/${table}/columns`);
	}

	async executeQuery(query: string): Promise<ApiResponse<QueryResult>> {
		return this.request<QueryResult>('/admin/database/query', {
			method: 'POST',
			body: JSON.stringify({ query })
		});
	}

	// Storage methods
	async getStorageBuckets(): Promise<ApiResponse<StorageBucket[]>> {
		return this.request<StorageBucket[]>('/storage/buckets');
	}

	async getBucketObjects(bucket: string): Promise<ApiResponse<StorageObject[]>> {
		return this.request<StorageObject[]>(`/storage/buckets/${bucket}/objects`);
	}

	async uploadFile(bucket: string, file: File, parentFolderId?: string | null): Promise<ApiResponse<StorageObject>> {
		const formData = new FormData();
		formData.append('file', file);
		
		// Add parentFolderId as a separate form field if provided
		if (parentFolderId) {
			formData.append('parentFolderId', parentFolderId);
		}

		const response = await fetch(`${API_BASE}/storage/buckets/${bucket}/upload`, {
			method: 'POST',
			// No Authorization header needed - httpOnly cookies sent automatically
			body: formData,
			credentials: 'include'
		});

		const data = await response.json();

		if (!response.ok) {
			return { error: data.error || `HTTP ${response.status}` };
		}

		return { data: data as StorageObject };
	}

	async deleteObject(bucket: string, objectId: string): Promise<ApiResponse<void>> {
		return this.request<void>(`/storage/buckets/${bucket}/objects/${objectId}`, {
			method: 'DELETE'
		});
	}

	async createFolder(bucket: string, name: string, parentFolderId?: string | null): Promise<ApiResponse<StorageObject>> {
		return this.request<StorageObject>(`/storage/buckets/${bucket}/folders`, {
			method: 'POST',
			body: JSON.stringify({
				name,
				parentFolderId: parentFolderId
			})
		});
	}


	// Settings methods
	async getSettings(): Promise<ApiResponse<AppSettings>> {
		return this.request<AppSettings>('/settings');
	}

	async updateSettings(settings: Partial<AppSettings>): Promise<ApiResponse<AppSettings>> {
		return this.request<AppSettings>('/admin/settings', {
			method: 'PATCH',
			body: JSON.stringify(settings)
		});
	}

	// Dashboard methods
	async getDashboardStats(): Promise<ApiResponse<DashboardStats>> {
		return this.request<DashboardStats>('/dashboard/stats');
	}

	// Extensions methods (admin)
	async getExtensions(): Promise<ApiResponse<Extension[]>> {
		return this.request<Extension[]>('/admin/extensions');
	}

	async toggleExtension(name: string, enabled: boolean): Promise<ApiResponse<Extension>> {
		return this.request<Extension>(`/admin/extensions/${name}/toggle`, {
			method: 'POST',
			body: JSON.stringify({ enabled })
		});
	}

	async getExtensionStatus(): Promise<ApiResponse<ExtensionStatus[]>> {
		return this.request<ExtensionStatus[]>('/admin/extensions/status');
	}

	// Analytics extension methods
	async getAnalyticsStats(): Promise<ApiResponse<AnalyticsStats>> {
		return this.request<AnalyticsStats>('/ext/analytics/stats');
	}

	async getAnalyticsPageviews(): Promise<ApiResponse<AnalyticsPageview[]>> {
		return this.request<AnalyticsPageview[]>('/ext/analytics/pageviews');
	}

	async getAnalyticsDailyStats(days: number = 7): Promise<ApiResponse<AnalyticsDailyStats[]>> {
		return this.request<AnalyticsDailyStats[]>(`/ext/analytics/daily?days=${days}`);
	}

	async trackAnalyticsEvent(event: AnalyticsEvent): Promise<ApiResponse<void>> {
		return this.request<void>('/ext/analytics/track', {
			method: 'POST',
			body: JSON.stringify(event)
		});
	}

	async exportAnalytics(): Promise<ApiResponse<AnalyticsStats>> {
		return this.request<AnalyticsStats>('/admin/analytics/export');
	}

	async clearAnalytics(): Promise<ApiResponse<void>> {
		return this.request<void>('/admin/analytics/clear', {
			method: 'DELETE'
		});
	}

	// Webhooks extension methods
	async getWebhooks(): Promise<ApiResponse<Webhook[]>> {
		return this.request<Webhook[]>('/ext/webhooks/webhooks');
	}

	async createWebhook(webhook: WebhookCreateRequest): Promise<ApiResponse<Webhook>> {
		return this.request<Webhook>('/ext/webhooks/webhooks', {
			method: 'POST',
			body: JSON.stringify(webhook)
		});
	}

	async toggleWebhook(id: string, active: boolean): Promise<ApiResponse<Webhook>> {
		return this.request<Webhook>(`/ext/webhooks/webhooks/${id}/toggle`, {
			method: 'POST',
			body: JSON.stringify({ active })
		});
	}

	async deleteWebhooks(ids: string[]): Promise<ApiResponse<void>> {
		return this.request<void>('/admin/webhooks/delete', {
			method: 'DELETE',
			body: JSON.stringify({ ids })
		});
	}

	// Generic HTTP methods for direct API calls
	// These throw on error for use with try/catch
	async get<T = unknown>(path: string): Promise<T> {
		const response = await this.request<T>(path);
		if (response.error) {
			const errorMsg = typeof response.error === 'string' ? response.error : response.error.message;
			throw new Error(errorMsg);
		}
		return response.data as T;
	}

	async post<T = unknown>(path: string, body?: unknown): Promise<T> {
		const response = await this.request<T>(path, {
			method: 'POST',
			body: body ? JSON.stringify(body) : undefined
		});
		if (response.error) {
			const errorMsg = typeof response.error === 'string' ? response.error : response.error.message;
			throw new Error(errorMsg);
		}
		return response.data as T;
	}

	async put<T = unknown>(path: string, body?: unknown): Promise<T> {
		const response = await this.request<T>(path, {
			method: 'PUT',
			body: body ? JSON.stringify(body) : undefined
		});
		if (response.error) {
			const errorMsg = typeof response.error === 'string' ? response.error : response.error.message;
			throw new Error(errorMsg);
		}
		return response.data as T;
	}

	async patch<T = unknown>(path: string, body?: unknown): Promise<T> {
		const response = await this.request<T>(path, {
			method: 'PATCH',
			body: body ? JSON.stringify(body) : undefined
		});
		if (response.error) {
			const errorMsg = typeof response.error === 'string' ? response.error : response.error.message;
			throw new Error(errorMsg);
		}
		return response.data as T;
	}

	async delete<T = unknown>(path: string): Promise<T> {
		const response = await this.request<T>(path, {
			method: 'DELETE'
		});
		if (response.error) {
			const errorMsg = typeof response.error === 'string' ? response.error : response.error.message;
			throw new Error(errorMsg);
		}
		return response.data as T;
	}
}

export const api = new ApiClient();