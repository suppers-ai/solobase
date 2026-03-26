import type {
	AuthUser, UserResponse, LoginRequest, LoginResponse, SignupRequest,
	ApiResponse, PaginatedResponse
} from '@solobase/types';
import { ErrorHandler } from './utils/error-handler';

export const API_BASE = import.meta.env.DEV ? '/api' : '';

export { ErrorHandler };

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
	async getCurrentUserRoles(): Promise<string[]> {
		const result = await this.getCurrentUser();
		if (result.error || !result.data) return [];
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

		try {
			const response = await fetch(`${API_BASE}${endpoint}`, {
				...options,
				headers,
				credentials: 'include'
			});

			const text = await response.text();
			if (!text) {
				if (!response.ok) {
					throw new Error(`HTTP ${response.status}: Empty response`);
				}
				return { data: {} as T };
			}

			let data;
			try {
				data = JSON.parse(text);
			} catch {
				throw new Error(`Invalid JSON response: ${text.substring(0, 100)}`);
			}

			if (!response.ok) {
				// Auto-logout on 401 — token is invalid (expired, secret rotated, etc.)
				if (response.status === 401 && !url.includes('/auth/login')) {
					const { authState } = await import('./stores/auth');
					if (authState.value.user) {
						authState.value = { user: null, roles: [], loading: false, error: null };
						window.location.href = '/auth/login';
						return { error: 'Session expired. Please log in again.' };
					}
				}

				const err = data.error;
				if (typeof err === 'object' && err !== null) {
					return { error: err };
				}
				const errMsg = typeof err === 'string' ? err : (data.message || `HTTP ${response.status}`);
				return { error: errMsg };
			}

			return { data: data as T };
		} catch (error) {
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
		return this.request<void>('/auth/logout', { method: 'POST' });
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
		return this.request<void>(`/admin/users/${id}`, { method: 'DELETE' });
	}

	// Database methods (admin)
	async getDatabaseTables(): Promise<ApiResponse<any[]>> {
		return this.request<any[]>('/admin/database/tables');
	}

	async getTableColumns(table: string): Promise<ApiResponse<any[]>> {
		return this.request<any[]>(`/admin/database/tables/${table}/columns`);
	}

	async executeQuery(query: string): Promise<ApiResponse<any>> {
		return this.request<any>('/admin/database/query', {
			method: 'POST',
			body: JSON.stringify({ query })
		});
	}

	// Storage methods
	async getStorageBuckets(): Promise<ApiResponse<any[]>> {
		return this.request<any[]>('/storage/buckets');
	}

	async getBucketObjects(bucket: string): Promise<ApiResponse<any[]>> {
		return this.request<any[]>(`/storage/buckets/${bucket}/objects`);
	}

	async uploadFile(bucket: string, file: File, parentFolderId?: string | null): Promise<ApiResponse<any>> {
		const formData = new FormData();
		formData.append('file', file);
		if (parentFolderId) {
			formData.append('parentFolderId', parentFolderId);
		}

		const response = await fetch(`${API_BASE}/storage/buckets/${bucket}/upload`, {
			method: 'POST',
			body: formData,
			credentials: 'include'
		});

		const data = await response.json();
		if (!response.ok) {
			return { error: data.error || `HTTP ${response.status}` };
		}
		return { data };
	}

	async deleteObject(bucket: string, objectId: string): Promise<ApiResponse<void>> {
		return this.request<void>(`/storage/buckets/${bucket}/objects/${objectId}`, { method: 'DELETE' });
	}

	async createFolder(bucket: string, name: string, parentFolderId?: string | null): Promise<ApiResponse<any>> {
		return this.request<any>(`/storage/buckets/${bucket}/folders`, {
			method: 'POST',
			body: JSON.stringify({ name, parentFolderId })
		});
	}

	// Settings methods
	async getSettings(): Promise<ApiResponse<any>> {
		return this.request<any>('/settings');
	}

	async updateSettings(settings: Record<string, any>): Promise<ApiResponse<any>> {
		return this.request<any>('/admin/settings', {
			method: 'PATCH',
			body: JSON.stringify(settings)
		});
	}

	// Dashboard methods
	async getDashboardStats(): Promise<ApiResponse<any>> {
		return this.request<any>('/dashboard/stats');
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

	// Generic HTTP methods (throw on error)
	private async makeRequest<T>(path: string, method: string, body?: unknown): Promise<T> {
		const response = await this.request<T>(path, {
			method,
			body: body ? JSON.stringify(body) : undefined
		});
		if (response.error) {
			const errorMsg = typeof response.error === 'string' ? response.error : response.error.message;
			throw new Error(errorMsg);
		}
		return response.data as T;
	}

	async get<T = unknown>(path: string): Promise<T> {
		return this.makeRequest<T>(path, 'GET');
	}

	async post<T = unknown>(path: string, body?: unknown): Promise<T> {
		return this.makeRequest<T>(path, 'POST', body);
	}

	async put<T = unknown>(path: string, body?: unknown): Promise<T> {
		return this.makeRequest<T>(path, 'PUT', body);
	}

	async patch<T = unknown>(path: string, body?: unknown): Promise<T> {
		return this.makeRequest<T>(path, 'PATCH', body);
	}

	async delete<T = unknown>(path: string): Promise<T> {
		return this.makeRequest<T>(path, 'DELETE');
	}
}

export const api = new ApiClient();
