import { signal, computed } from '@preact/signals';
import type { AuthUser, UserResponse, LoginResponse } from '@solobase/types';

interface AuthState {
	user: AuthUser | null;
	roles: string[];
	loading: boolean;
	error: string | null;
}

export const authState = signal<AuthState>({
	user: null,
	roles: [],
	loading: true,
	error: null
});

export const isAuthenticated = computed(() => !!authState.value.user);
export const currentUser = computed(() => authState.value.user);
export const userRoles = computed(() => authState.value.roles);
export const authLoading = computed(() => authState.value.loading);

// Import api lazily to avoid circular dependency
let _api: typeof import('../api').api | null = null;
async function getApi() {
	if (!_api) {
		const mod = await import('../api');
		_api = mod.api;
	}
	return _api;
}

export async function login(email: string, password: string): Promise<{ ok: boolean; error?: string; code?: string }> {
	authState.value = { ...authState.value, loading: true, error: null };

	const api = await getApi();
	const response = await api.login({ email, password });

	if (response.error) {
		const err = response.error;
		const errorMessage = typeof err === 'string' ? err : err.message;
		const errorCode = typeof err === 'string' ? '' : (err.code || '');
		authState.value = { ...authState.value, loading: false, error: errorMessage };
		return { ok: false, error: errorMessage, code: errorCode };
	}

	// API returns { access_token, user: { email, id, name, roles } }
	const data = response.data as any;
	const user = data?.user;

	authState.value = {
		user: user || null,
		roles: user?.roles || [],
		loading: false,
		error: null
	};
	return { ok: true };
}

export async function logout(): Promise<void> {
	const api = await getApi();
	await api.logout();
	authState.value = { user: null, roles: [], loading: false, error: null };
}

export async function checkAuth(): Promise<boolean> {
	authState.value = { ...authState.value, loading: true };

	const api = await getApi();
	const response = await api.getCurrentUser();

	if (response.error) {
		authState.value = { user: null, roles: [], loading: false, error: null };
		return false;
	}

	// API returns { user: { email, id, name, roles, ... } }
	const data = response.data as any;
	const user = data?.user;

	authState.value = {
		user: user || null,
		roles: user?.roles || [],
		loading: false,
		error: null
	};
	return { ok: true };
}

export function setUser(user: AuthUser | null): void {
	authState.value = { ...authState.value, user };
}
