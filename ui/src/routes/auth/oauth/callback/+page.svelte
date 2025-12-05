<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { auth } from '$lib/stores/auth';
	import api from '$lib/api';

	let loading = true;
	let error = '';

	onMount(async () => {
		// Get token from URL parameters
		const token = $page.url.searchParams.get('token');
		const errorParam = $page.url.searchParams.get('error');

		if (errorParam) {
			error = decodeURIComponent(errorParam);
			loading = false;
			setTimeout(() => {
				goto('/auth/login');
			}, 3000);
			return;
		}

		if (!token) {
			error = 'No authentication token received';
			loading = false;
			setTimeout(() => {
				goto('/auth/login');
			}, 3000);
			return;
		}

		try {
			// Store the token
			api.setToken(token);

			// Fetch user info
			const user = await api.getCurrentUser();

			if (user) {
				// Update auth store
				auth.setUser(user);

				// Redirect to home page
				await goto('/');
			} else {
				error = 'Failed to fetch user information';
				loading = false;
			}
		} catch (err) {
			console.error('OAuth callback error:', err);
			error = 'Authentication failed. Please try again.';
			loading = false;
			setTimeout(() => {
				goto('/auth/login');
			}, 3000);
		}
	});
</script>

<div class="callback-page">
	<div class="callback-container">
		{#if loading}
			<div class="loading-spinner"></div>
			<h2>Completing sign in...</h2>
			<p>Please wait while we finish setting up your account.</p>
		{:else if error}
			<div class="error-icon">⚠️</div>
			<h2>Authentication Error</h2>
			<p>{error}</p>
			<p class="redirect-message">Redirecting to login page...</p>
		{/if}
	</div>
</div>

<style>
	.callback-page {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #f0f0f0;
		padding: 1rem;
	}

	.callback-container {
		text-align: center;
		background: white;
		padding: 3rem 2rem;
		border-radius: 12px;
		box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
		max-width: 400px;
		width: 100%;
	}

	.loading-spinner {
		width: 60px;
		height: 60px;
		border: 4px solid #e5e7eb;
		border-top-color: #189AB4;
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
		margin: 0 auto 1.5rem;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.error-icon {
		font-size: 3rem;
		margin-bottom: 1rem;
	}

	h2 {
		color: #374151;
		font-size: 1.5rem;
		margin-bottom: 0.75rem;
	}

	p {
		color: #6b7280;
		font-size: 0.9375rem;
		margin-bottom: 0.5rem;
	}

	.redirect-message {
		font-size: 0.875rem;
		font-style: italic;
		margin-top: 1rem;
	}
</style>
