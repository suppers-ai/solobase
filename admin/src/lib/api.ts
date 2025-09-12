import type { 
	User, LoginRequest, LoginResponse, SignupRequest,
	DatabaseTable, DatabaseColumn, QueryResult,
	StorageObject, StorageBucket,
	Collection, CollectionSchema,
	AppSettings, DashboardStats,
	ApiResponse, PaginatedResponse
} from './types';

const API_BASE = '/api';

class ApiClient {
	private token: string | null = null;

	constructor() {
		// Try to restore token from localStorage
		if (typeof window !== 'undefined') {
			this.token = localStorage.getItem('auth_token');
		}
	}

	setToken(token: string) {
		this.token = token;
		if (typeof window !== 'undefined') {
			localStorage.setItem('auth_token', token);
		}
	}

	private async request<T>(
		endpoint: string,
		options: RequestInit = {}
	): Promise<ApiResponse<T>> {
		const headers: HeadersInit = {
			'Content-Type': 'application/json',
			...options.headers
		};

		// Only add Authorization header if we have a valid token
		if (this.token) {
			headers['Authorization'] = `Bearer ${this.token}`;
		}

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
				// If we get a 401, clear the invalid token
				if (response.status === 401 && this.token) {
					console.log('Token invalid, clearing from storage');
					this.token = null;
					if (typeof window !== 'undefined') {
						localStorage.removeItem('auth_token');
					}
				}
				throw new Error(data.error || `HTTP ${response.status}`);
			}

			return { data: data as T };
		} catch (error) {
			console.error('API request failed:', error);
			return { 
				error: error instanceof Error ? error.message : 'An error occurred' 
			};
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

		if (response.data?.token) {
			this.token = response.data.token;
			if (typeof window !== 'undefined') {
				localStorage.setItem('auth_token', this.token);
				console.log('Token stored in localStorage');
			}
		}

		return response;
	}

	async logout(): Promise<ApiResponse<void>> {
		const response = await this.request<void>('/auth/logout', {
			method: 'POST'
		});

		this.token = null;
		if (typeof window !== 'undefined') {
			localStorage.removeItem('auth_token');
		}

		return response;
	}

	async signup(request: SignupRequest): Promise<ApiResponse<User>> {
		return this.request<User>('/auth/signup', {
			method: 'POST',
			body: JSON.stringify(request)
		});
	}

	async getCurrentUser(): Promise<ApiResponse<User>> {
		console.log('Getting current user, token:', this.token ? 'present' : 'missing');
		const response = await this.request<User>('/auth/me');
		console.log('Current user response:', response);
		return response;
	}

	// Users methods
	async getUsers(page = 1, pageSize = 20): Promise<ApiResponse<PaginatedResponse<User>>> {
		return this.request<PaginatedResponse<User>>(`/users?page=${page}&page_size=${pageSize}`);
	}

	async getUser(id: string): Promise<ApiResponse<User>> {
		return this.request<User>(`/users/${id}`);
	}

	async updateUser(id: string, updates: Partial<User>): Promise<ApiResponse<User>> {
		return this.request<User>(`/users/${id}`, {
			method: 'PATCH',
			body: JSON.stringify(updates)
		});
	}

	async deleteUser(id: string): Promise<ApiResponse<void>> {
		return this.request<void>(`/users/${id}`, {
			method: 'DELETE'
		});
	}

	// Database methods
	async getDatabaseTables(): Promise<ApiResponse<DatabaseTable[]>> {
		return this.request<DatabaseTable[]>('/database/tables');
	}

	async getTableColumns(table: string): Promise<ApiResponse<DatabaseColumn[]>> {
		return this.request<DatabaseColumn[]>(`/database/tables/${table}/columns`);
	}

	async executeQuery(query: string): Promise<ApiResponse<QueryResult>> {
		return this.request<QueryResult>('/database/query', {
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
			headers: {
				'Authorization': this.token ? `Bearer ${this.token}` : ''
			},
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

	// Collections methods
	async getCollections(): Promise<ApiResponse<Collection[]>> {
		return this.request<Collection[]>('/collections');
	}

	async getCollection(id: string): Promise<ApiResponse<Collection>> {
		return this.request<Collection>(`/collections/${id}`);
	}

	async createCollection(name: string, schema: CollectionSchema): Promise<ApiResponse<Collection>> {
		return this.request<Collection>('/collections', {
			method: 'POST',
			body: JSON.stringify({ name, schema })
		});
	}

	async updateCollection(id: string, updates: Partial<Collection>): Promise<ApiResponse<Collection>> {
		return this.request<Collection>(`/collections/${id}`, {
			method: 'PATCH',
			body: JSON.stringify(updates)
		});
	}

	async deleteCollection(id: string): Promise<ApiResponse<void>> {
		return this.request<void>(`/collections/${id}`, {
			method: 'DELETE'
		});
	}

	// Settings methods
	async getSettings(): Promise<ApiResponse<AppSettings>> {
		return this.request<AppSettings>('/settings');
	}

	async updateSettings(settings: Partial<AppSettings>): Promise<ApiResponse<AppSettings>> {
		return this.request<AppSettings>('/settings', {
			method: 'PATCH',
			body: JSON.stringify(settings)
		});
	}

	// Dashboard methods
	async getDashboardStats(): Promise<ApiResponse<DashboardStats>> {
		return this.request<DashboardStats>('/dashboard/stats');
	}

	// Extensions methods
	async getExtensions(): Promise<ApiResponse<any[]>> {
		return this.request<any[]>('/extensions');
	}

	async toggleExtension(name: string, enabled: boolean): Promise<ApiResponse<any>> {
		return this.request<any>(`/extensions/${name}/toggle`, {
			method: 'POST',
			body: JSON.stringify({ enabled })
		});
	}

	async getExtensionStatus(): Promise<ApiResponse<any[]>> {
		return this.request<any[]>('/extensions/status');
	}

	// Analytics extension methods
	async getAnalyticsStats(): Promise<ApiResponse<any>> {
		return this.request<any>('/ext/analytics/api/stats');
	}

	async getAnalyticsPageviews(): Promise<ApiResponse<any>> {
		return this.request<any>('/ext/analytics/api/pageviews');
	}

	async getAnalyticsDailyStats(days: number = 7): Promise<ApiResponse<any>> {
		return this.request<any>(`/ext/analytics/api/daily?days=${days}`);
	}

	async trackAnalyticsEvent(event: any): Promise<ApiResponse<void>> {
		return this.request<void>('/ext/analytics/api/track', {
			method: 'POST',
			body: JSON.stringify(event)
		});
	}

	// Webhooks extension methods
	async getWebhooks(): Promise<ApiResponse<any>> {
		return this.request<any>('/ext/webhooks/api/webhooks');
	}

	async createWebhook(webhook: any): Promise<ApiResponse<any>> {
		return this.request<any>('/ext/webhooks/api/webhooks/create', {
			method: 'POST',
			body: JSON.stringify(webhook)
		});
	}

	async toggleWebhook(id: string, active: boolean): Promise<ApiResponse<any>> {
		return this.request<any>(`/ext/webhooks/api/webhooks/${id}/toggle`, {
			method: 'POST',
			body: JSON.stringify({ active })
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