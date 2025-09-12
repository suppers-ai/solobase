<script lang="ts">
	import '../app.css';
	import '@common/ui-components/css/variables.css';
	import { page } from '$app/stores';
	import { AppLayout } from '@common/ui-components';
	import { 
		Home, Users, Database, HardDrive, 
		FileText, Puzzle, Settings, Plus, X
	} from 'lucide-svelte';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { auth, currentUser } from '$lib/stores/auth';
	import { observeAndFixInputs } from '$lib/utils/fixTextSelection';
	import { api } from '$lib/api';
	import type { AppSettings } from '$lib/types';
	
	let user: any = null;
	let authChecked = false;
	let settings: AppSettings | null = null;
	let notificationDismissed = false;
	
	// Subscribe to the currentUser store
	$: user = $currentUser;
	
	// Define pages that don't require admin role
	const publicPages = ['/auth/login', '/auth/signup', '/auth/logout', '/'];
	const profilePages = ['/profile'];
	
	// Navigation configuration for admin
	const navigation = [
		{ 
			title: 'Dashboard', 
			href: '/admin', 
			icon: Home 
		},
		{ 
			title: 'Users', 
			href: '/admin/users', 
			icon: Users 
		},
		{ 
			title: 'Database', 
			href: '/admin/database', 
			icon: Database 
		},
		{ 
			title: 'Storage', 
			href: '/admin/storage', 
			icon: HardDrive 
		},
		{ 
			title: 'Logs', 
			href: '/admin/logs', 
			icon: FileText 
		},
		{
			title: 'Extensions',
			icon: Puzzle,
			expandable: true,
			children: [
				{ 
					title: 'Products & Pricing', 
					href: '/admin/extensions/products'
				},
				{ 
					title: 'Hugo Sites', 
					href: '/admin/extensions/hugo'
				},
				{ 
					title: 'Analytics', 
					href: '/admin/extensions/analytics'
				},
				{ 
					title: 'Cloud Storage', 
					href: '/admin/extensions/cloudstorage'
				},
				{ 
					title: 'Webhooks', 
					href: '/admin/extensions/webhooks'
				},
				{ 
					title: 'Manage Extensions', 
					href: '/admin/extensions/manage',
					icon: Plus
				}
			]
		},
		{ 
			title: 'Settings', 
			href: '/admin/settings', 
			icon: Settings 
		}
	];
	
	// Reactive role-based routing - handles navigation after auth state changes
	$: if (user && typeof window !== 'undefined') {
		const currentPath = $page.url.pathname;
		const isPublicPage = publicPages.some(p => currentPath === p || currentPath.startsWith('/auth/'));
		const isProfilePage = currentPath.startsWith('/profile');
		const isAdminPage = currentPath.startsWith('/admin');
		
		// Redirect non-admin users away from admin pages
		if (user.role !== 'admin' && isAdminPage) {
			goto('/profile');
		}
	}
	
	onMount(async () => {
		// Fix text selection in inputs
		const observer = observeAndFixInputs();
		
		// First check if we have a stored token before doing auth check
		const hasStoredToken = typeof window !== 'undefined' && localStorage.getItem('auth_token');
		
		// If we have a stored token, attempt to validate it
		if (hasStoredToken) {
			// Check if user is authenticated on mount
			const isAuth = await auth.checkAuth();
			authChecked = true;
			
			// Only redirect if auth check definitively failed (not just loading)
			if (!isAuth) {
				const currentPath = $page.url.pathname;
				const isPublicPage = publicPages.some(p => currentPath === p || currentPath.startsWith('/auth/'));
				const isProfilePage = currentPath.startsWith('/profile');
				
				if (!isPublicPage && !isProfilePage) {
					goto('/auth/login');
					return;
				}
			} else {
				// Load settings to check for notification
				loadSettings();
			}
		} else {
			// No stored token, check if we need to redirect
			const currentPath = $page.url.pathname;
			const isPublicPage = publicPages.some(p => currentPath === p || currentPath.startsWith('/auth/'));
			const isProfilePage = currentPath.startsWith('/profile');
			
			authChecked = true;
			
			if (!isPublicPage && !isProfilePage) {
				goto('/auth/login');
				return;
			}
		}
		
		// Cleanup observer on unmount
		return () => {
			if (observer) {
				observer.disconnect();
			}
		};
	});
	
	async function loadSettings() {
		const response = await api.getSettings();
		if (response.data) {
			settings = response.data;
		}
	}
	
	function dismissNotification() {
		notificationDismissed = true;
	}
	
	async function handleLogout() {
		// Navigate to logout page which handles the logout process
		window.location.href = '/auth/logout';
	}
	
	// Check if on auth pages or admin pages
	$: isAuthPage = $page.url.pathname.startsWith('/auth/');
	$: isProfilePage = $page.url.pathname.startsWith('/profile');
	$: isAdminPage = $page.url.pathname.startsWith('/admin');
	$: isRootPage = $page.url.pathname === '/';
	
	// Track if notification should be shown
	$: showNotification = settings?.notification && !notificationDismissed && !isAuthPage && !isRootPage;
</script>

<div class="app-container" class:with-notification={showNotification}>
	{#if showNotification}
		<div class="notification-banner">
			<div class="notification-content">
				<span>{settings.notification}</span>
				<button class="notification-close" on:click={dismissNotification}>
					<X size={18} />
				</button>
			</div>
		</div>
	{/if}

	<div class="main-content">
		{#if isAuthPage || isRootPage}
			<!-- Auth pages and root page without layout -->
			<slot />
		{:else if !authChecked && !publicPages.some(p => $page.url.pathname === p || $page.url.pathname.startsWith('/auth/'))}
			<!-- Show loading state while checking auth for protected pages -->
			<div class="auth-loading">
				<div class="spinner"></div>
				<p>Loading...</p>
			</div>
		{:else if isProfilePage}
			<!-- Profile pages have their own layout -->
			<slot />
		{:else if isAdminPage}
			<!-- Main admin layout - only for admin pages -->
			<AppLayout
				currentUser={user}
				{navigation}
				currentPath={$page.url.pathname}
				logoSrc="/logo_long.png"
				logoCollapsedSrc="/logo.png"
				projectName="Solobase"
				mobileTitle="Solobase Admin"
				onLogout={handleLogout}
			>
				<slot />
			</AppLayout>
		{:else}
			<!-- Other pages without layout -->
			<slot />
		{/if}
	</div>
</div>

<style>
	.app-container {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
	}
	
	.main-content {
		flex: 1;
		display: flex;
		flex-direction: column;
	}
	
	.app-container.with-notification .main-content {
		padding-top: 52px;
	}
	
	.notification-banner {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
		color: white;
		padding: 0.75rem 1rem;
		z-index: 9999;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
		height: 52px;
		display: flex;
		align-items: center;
	}
	
	.notification-content {
		max-width: 1200px;
		width: 100%;
		margin: 0 auto;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
	}
	
	.notification-content span {
		flex: 1;
		font-size: 0.95rem;
		line-height: 1.5;
	}
	
	.notification-close {
		background: rgba(255, 255, 255, 0.2);
		border: none;
		color: white;
		width: 28px;
		height: 28px;
		border-radius: 6px;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition: background 0.2s;
		flex-shrink: 0;
	}
	
	.notification-close:hover {
		background: rgba(255, 255, 255, 0.3);
	}
	
	.auth-loading {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100vh;
		background: #f0f0f0;
	}
	
	.spinner {
		width: 40px;
		height: 40px;
		border: 4px solid #e2e8f0;
		border-top-color: #3b82f6;
		border-radius: 50%;
		animation: spin 1s linear infinite;
	}
	
	@keyframes spin {
		0% { transform: rotate(0deg); }
		100% { transform: rotate(360deg); }
	}
	
	.auth-loading p {
		margin-top: 1rem;
		color: #666;
		font-size: 14px;
	}
</style>