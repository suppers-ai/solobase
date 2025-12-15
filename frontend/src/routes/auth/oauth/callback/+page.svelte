<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import LoadingSpinner from '$lib/components/ui/LoadingSpinner.svelte';

	let status = 'processing';
	let message = 'Processing OAuth authentication...';

	onMount(async () => {
		try {
			// Extract the success and error parameters from the URL
			const urlParams = $page.url.searchParams;
			const success = urlParams.get('success');
			const error = urlParams.get('error');
			const redirectTo = urlParams.get('redirect');

			if (success === 'true') {
				status = 'success';
				message = 'Authentication successful! Redirecting...';

				// If this is a popup window (OAuth flow), notify parent and close
				if (window.opener) {
					// Notify parent window of successful auth
					// Send both message types so it works with SDK popup and login page popup
					window.opener.postMessage({
						type: 'oauth-success',
						redirect: redirectTo
					}, '*');
					window.opener.postMessage({
						type: 'auth-success',
						redirect: redirectTo
					}, '*');
					window.close();
					return;
				}

				// If not a popup, redirect normally
				const destination = redirectTo && isValidRedirectUrl(redirectTo) ? redirectTo : '/';
				setTimeout(() => goto(destination), 1000);
			} else {
				status = 'error';
				message = error || 'Authentication failed. Please try again.';

				// If this is a popup window, notify parent of error
				if (window.opener) {
					window.opener.postMessage({
						type: 'oauth-error',
						error: message
					}, window.location.origin);
					window.close();
					return;
				}

				// If not a popup, redirect to login page after delay
				setTimeout(() => goto('/auth/login'), 3000);
			}
		} catch (err) {
			console.error('OAuth callback error:', err);
			status = 'error';
			message = 'An unexpected error occurred.';

			if (window.opener) {
				window.opener.postMessage({
					type: 'oauth-error',
					error: message
				}, window.location.origin);
				window.close();
			} else {
				setTimeout(() => goto('/auth/login'), 3000);
			}
		}
	});

	// Validate redirect URL to prevent open redirect attacks
	function isValidRedirectUrl(url: string): boolean {
		if (!url) return false;

		try {
			// For relative URLs, they're generally safe
			if (url.startsWith('/') && !url.startsWith('//')) {
				return true;
			}

			// For absolute URLs, ensure they're on the same origin
			if (url.startsWith('http')) {
				const urlObj = new URL(url);
				return urlObj.origin === window.location.origin;
			}

			return false;
		} catch {
			return false;
		}
	}
</script>

<div class="callback-container">
	<div class="callback-content">
		{#if status === 'processing'}
			<LoadingSpinner size="lg" color="primary" centered />
			<h2>Processing Authentication</h2>
			<p>{message}</p>
		{:else if status === 'success'}
			<div class="success-icon">✓</div>
			<h2>Authentication Successful</h2>
			<p>{message}</p>
		{:else if status === 'error'}
			<div class="error-icon">✗</div>
			<h2>Authentication Failed</h2>
			<p>{message}</p>
		{/if}
	</div>
</div>

<style>
	.callback-container {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
		font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
	}

	.callback-content {
		text-align: center;
		background: white;
		padding: 2rem;
		border-radius: 12px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1);
		max-width: 400px;
		width: 100%;
		margin: 0 1rem;
	}

	h2 {
		margin: 1rem 0 0.5rem 0;
		color: #333;
		font-weight: 600;
	}

	p {
		color: #666;
		line-height: 1.5;
		margin-bottom: 0;
	}


	.success-icon {
		width: 60px;
		height: 60px;
		border-radius: 50%;
		background: #4caf50;
		color: white;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 28px;
		font-weight: bold;
		margin: 0 auto 1rem;
	}

	.error-icon {
		width: 60px;
		height: 60px;
		border-radius: 50%;
		background: #f44336;
		color: white;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 28px;
		font-weight: bold;
		margin: 0 auto 1rem;
	}

	@keyframes spin {
		0% { transform: rotate(0deg); }
		100% { transform: rotate(360deg); }
	}
</style>