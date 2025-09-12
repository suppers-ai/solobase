import { goto } from '$app/navigation';
import { get } from 'svelte/store';
import { currentUser } from '$lib/stores/auth';

/**
 * Check if the current user is an admin
 * Redirects to profile page if not
 */
export function requireAdmin() {
	const user = get(currentUser);
	
	if (!user) {
		// User not logged in, redirect to login
		goto('/login');
		return false;
	}
	
	if (user.role !== 'admin') {
		// User is not admin, redirect to profile
		goto('/profile');
		return false;
	}
	
	return true;
}

/**
 * Check if the current user has one of the allowed roles
 * @param allowedRoles Array of allowed roles
 * @param redirectTo Where to redirect if not authorized (default: '/profile')
 */
export function requireRole(allowedRoles: string[], redirectTo: string = '/profile') {
	const user = get(currentUser);
	
	if (!user) {
		goto('/login');
		return false;
	}
	
	if (!allowedRoles.includes(user.role)) {
		goto(redirectTo);
		return false;
	}
	
	return true;
}

/**
 * Check if user is authenticated
 * Redirects to login if not
 */
export function requireAuth() {
	const user = get(currentUser);
	
	if (!user) {
		goto('/login');
		return false;
	}
	
	return true;
}