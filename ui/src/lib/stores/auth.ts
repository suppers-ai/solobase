import { writable, derived } from 'svelte/store';
import type { User } from '$lib/types';
import { api } from '$lib/api';

interface AuthState {
	user: User | null;
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
				update(state => ({ ...state, loading: false, error: response.error! }));
				return false;
			}

			console.log('Login successful, user:', response.data!.user);
			
			// Extract roles from JWT token
			const roles = api.getRolesFromToken();
			console.log('User roles from token:', roles);
			
			// Update both user and roles atomically
			set({ 
				user: response.data!.user,
				roles: roles,
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

			console.log('Auth check successful, user:', response.data);
			
			// Extract roles from JWT token
			const roles = api.getRolesFromToken();
			console.log('User roles from token:', roles);
			
			// Update both user and roles atomically
			set({ 
				user: response.data!,
				roles: roles,
				loading: false, 
				error: null 
			});
			return true;
		},
		setUser(user: User | null) {
			update(state => ({ ...state, user }));
		},
		updateUser(user: User) {
			update(state => ({ ...state, user }));
		}
	};
}

export const authStore = createAuthStore();
export const auth = authStore; // Keep for backwards compatibility
export const isAuthenticated = derived(authStore, $auth => !!$auth.user);
export const currentUser = derived(authStore, $auth => $auth.user);
export const userRoles = derived(authStore, $auth => $auth.roles);