import { writable, derived } from 'svelte/store';
import type { User } from '$lib/types';
import { api } from '$lib/api';

interface AuthState {
	user: User | null;
	loading: boolean;
	error: string | null;
}

function createAuthStore() {
	const { subscribe, set, update } = writable<AuthState>({
		user: null,
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
			update(state => ({ 
				...state, 
				user: response.data!.user, 
				loading: false, 
				error: null 
			}));
			return true;
		},
		async logout() {
			console.log('Logging out...');
			await api.logout();
			set({ user: null, loading: false, error: null });
			console.log('Logout complete, auth store cleared');
		},
		async checkAuth() {
			update(state => ({ ...state, loading: true }));
			
			console.log('Checking auth status...');
			const response = await api.getCurrentUser();
			console.log('Current user response:', response);
			
			if (response.error) {
				console.log('Auth check failed:', response.error);
				set({ user: null, loading: false, error: null });
				return false;
			}

			console.log('Auth check successful, user:', response.data);
			update(state => ({ 
				...state, 
				user: response.data!, 
				loading: false, 
				error: null 
			}));
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