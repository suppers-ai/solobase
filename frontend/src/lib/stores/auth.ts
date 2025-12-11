import { writable, derived } from 'svelte/store';
import type { AuthUser, UserResponse, LoginResponse } from '$lib/types';
import { api } from '$lib/api';

interface AuthState {
	user: AuthUser | null;
	roles: string[];
	loading: boolean;
	error: string | null;
}

function createAuthStore() {
	const { subscribe, set, update } = writable<AuthState>({
		user: null,
		roles: [],
		loading: true,
		error: null
	});

	return {
		subscribe,
		async login(email: string, password: string) {
			update(state => ({ ...state, loading: true, error: null }));

			console.log('Attempting login for:', email);
			const response = await api.login({ email, password });
			console.log('Login response:', response);

			if (response.error) {
				console.error('Login failed:', response.error);
				const errorMessage = typeof response.error === 'string'
					? response.error
					: response.error.message;
				update(state => ({ ...state, loading: false, error: errorMessage }));
				return false;
			}

			// Login response wraps UserResponse in data: { data: UserResponse, message: string }
			const loginResponse = response.data as LoginResponse;
			const userResponse = loginResponse.data;
			console.log('Login successful, user:', userResponse.user);
			console.log('User roles:', userResponse.roles);

			set({
				user: userResponse.user,
				roles: userResponse.roles || [],
				loading: false,
				error: null
			});
			return true;
		},
		async logout() {
			console.log('Logging out...');
			await api.logout();
			set({ user: null, roles: [], loading: false, error: null });
			console.log('Logout complete, auth store cleared');
		},
		async checkAuth() {
			update(state => ({ ...state, loading: true }));

			console.log('Checking auth status...');
			const response = await api.getCurrentUser();
			console.log('Current user response:', response);

			if (response.error) {
				console.log('Auth check failed:', response.error);
				set({ user: null, roles: [], loading: false, error: null });
				return false;
			}

			// /api/auth/me returns UserResponse directly (not wrapped in data)
			const userResponse = response.data as UserResponse;
			console.log('Auth check successful, user:', userResponse.user);
			console.log('User roles:', userResponse.roles);

			set({
				user: userResponse.user,
				roles: userResponse.roles || [],
				loading: false,
				error: null
			});
			return true;
		},
		setUser(user: AuthUser | null) {
			update(state => ({ ...state, user }));
		},
		updateUser(user: AuthUser) {
			update(state => ({ ...state, user }));
		}
	};
}

export const authStore = createAuthStore();
export const auth = authStore; // Keep for backwards compatibility
export const isAuthenticated = derived(authStore, $auth => !!$auth.user);
export const currentUser = derived(authStore, $auth => $auth.user);
export const userRoles = derived(authStore, $auth => $auth.roles);
