import type {
	AuthUser, UserResponse, LoginRequest, LoginResponse, SignupRequest,
	DatabaseTable, DatabaseColumn, QueryResult,
	StorageObject, StorageBucket,
	AppSettings, DashboardStats,
	ApiResponse, PaginatedResponse
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
			} catch (e) {
				console.error('Failed to parse response:', text);
				throw new Error(`Invalid JSON response: ${text.substring(0, 100)}`);
			}

			if (!response.ok) {
				// If we get a 401, the cookie may be expired or invalid
				if (response.status === 401) {
					console.log('Authentication failed - cookie may be expired');
				}
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
		console.log('API login request:', request);
		const response = await this.request<LoginResponse>('/auth/login', {
			method: 'POST',
			body: JSON.stringify(request)
		});
		console.log('API login response:', response);

		// Token is automatically set as httpOnly cookie by backend
		// No client-side token handling needed

		return response;
	}

	async logout(): Promise<ApiResponse<void>> {
		const response = await this.request<void>('/auth/logout', {
			method: 'POST'
		});

		// Token is automatically cleared via httpOnly cookie expiration by backend
		// No client-side token cleanup needed

		return response;
	}

	async signup(request: SignupRequest): Promise<ApiResponse<AuthUser>> {
		return this.request<AuthUser>('/auth/signup', {
			method: 'POST',
			body: JSON.stringify(request)
		});
	}

	async getCurrentUser(): Promise<ApiResponse<UserResponse>> {
		console.log('Getting current user via httpOnly cookie authentication');
		const response = await this.request<UserResponse>('/auth/me');
		console.log('Current user response:', response);
		return response;
	}

	// Users methods (admin)
	async getUsers(page = 1, pageSize = 20): Promise<ApiResponse<PaginatedResponse<AuthUser>>> {
		return this.request<PaginatedResponse<AuthUser>>(`/admin/users?page=${page}&page_size=${pageSize}`);
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
		
		// Add parent_folder_id as a separate form field if provided
		if (parentFolderId) {
			formData.append('parent_folder_id', parentFolderId);
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

	async createFolder(bucket: string, name: string, parentFolderId?: string | null): Promise<ApiResponse<any>> {
		return this.request<any>(`/storage/buckets/${bucket}/folders`, {
			method: 'POST',
			body: JSON.stringify({ 
				name, 
				parent_folder_id: parentFolderId 
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
	async getExtensions(): Promise<ApiResponse<any[]>> {
		return this.request<any[]>('/admin/extensions');
	}

	async toggleExtension(name: string, enabled: boolean): Promise<ApiResponse<any>> {
		return this.request<any>(`/admin/extensions/${name}/toggle`, {
			method: 'POST',
			body: JSON.stringify({ enabled })
		});
	}

	async getExtensionStatus(): Promise<ApiResponse<any[]>> {
		return this.request<any[]>('/admin/extensions/status');
	}

	// Analytics extension methods
	async getAnalyticsStats(): Promise<ApiResponse<any>> {
		return this.request<any>('/ext/analytics/stats');
	}

	async getAnalyticsPageviews(): Promise<ApiResponse<any>> {
		return this.request<any>('/ext/analytics/pageviews');
	}

	async getAnalyticsDailyStats(days: number = 7): Promise<ApiResponse<any>> {
		return this.request<any>(`/ext/analytics/daily?days=${days}`);
	}

	async trackAnalyticsEvent(event: any): Promise<ApiResponse<void>> {
		return this.request<void>('/ext/analytics/track', {
			method: 'POST',
			body: JSON.stringify(event)
		});
	}

	async exportAnalytics(): Promise<ApiResponse<any>> {
		return this.request<any>('/admin/analytics/export');
	}

	async clearAnalytics(): Promise<ApiResponse<void>> {
		return this.request<void>('/admin/analytics/clear', {
			method: 'DELETE'
		});
	}

	// Webhooks extension methods
	async getWebhooks(): Promise<ApiResponse<any>> {
		return this.request<any>('/ext/webhooks/webhooks');
	}

	async createWebhook(webhook: any): Promise<ApiResponse<any>> {
		return this.request<any>('/ext/webhooks/webhooks', {
			method: 'POST',
			body: JSON.stringify(webhook)
		});
	}

	async toggleWebhook(id: string, active: boolean): Promise<ApiResponse<any>> {
		return this.request<any>(`/ext/webhooks/webhooks/${id}/toggle`, {
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
	async get(path: string): Promise<any> {
		const response = await this.request<any>(path);
		// Return data directly if it exists, otherwise return the whole response
		return response.data !== undefined ? response.data : response;
	}

	async post(path: string, body?: any): Promise<any> {
		const response = await this.request<any>(path, {
			method: 'POST',
			body: body ? JSON.stringify(body) : undefined
		});
		return response.data !== undefined ? response.data : response;
	}

	async put(path: string, body?: any): Promise<any> {
		const response = await this.request<any>(path, {
			method: 'PUT',
			body: body ? JSON.stringify(body) : undefined
		});
		return response.data !== undefined ? response.data : response;
	}

	async patch(path: string, body?: any): Promise<any> {
		const response = await this.request<any>(path, {
			method: 'PATCH',
			body: body ? JSON.stringify(body) : undefined
		});
		return response.data !== undefined ? response.data : response;
	}

	async delete(path: string): Promise<any> {
		const response = await this.request<any>(path, {
			method: 'DELETE'
		});
		return response.data !== undefined ? response.data : response;
	}
}

export const api = new ApiClient();