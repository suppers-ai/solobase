<script lang="ts">
	import { LoginForm } from '@common/ui-components';
	import '@common/ui-components/css/variables.css';
	import { auth } from '$lib/stores/auth';
	import { goto } from '$app/navigation';
	import { get } from 'svelte/store';
	import { page } from '$app/stores';
	
	let email = '';
	let password = '';
	let loading = false;
	let error = '';
	
	// Get redirect parameter from URL
	$: redirectTo = $page.url.searchParams.get('redirect');

	/**
	 * Validates that a redirect URL is safe (same-origin only)
	 * Prevents open redirect vulnerabilities
	 */
	function isValidRedirectUrl(url: string): boolean {
		if (!url) return false;

		try {
			// For relative URLs, they're safe by default
			if (url.startsWith('/') && !url.startsWith('//')) {
				return true;
			}

			// For absolute URLs, check if they're same-origin
			const urlObj = new URL(url, window.location.origin);
			return urlObj.origin === window.location.origin;
		} catch {
			// If URL parsing fails, it's not valid
			return false;
		}
	}

	async function handleLogin(loginEmail: string, loginPassword: string) {
		loading = true;
		error = '';

		const success = await auth.login(loginEmail, loginPassword);

		if (success) {
			// Validate redirect parameter to prevent open redirect vulnerabilities
			if (redirectTo && isValidRedirectUrl(redirectTo)) {
				console.log('Redirecting to:', redirectTo);
				await goto(redirectTo);
			} else {
				// If redirect is invalid or not provided, go to home
				if (redirectTo) {
					console.warn('Invalid redirect URL blocked:', redirectTo);
				}
				await goto('/');
			}
		} else {
			const authState = get(auth);
			error = authState.error || 'Invalid email or password';
			loading = false;
		}
	}
</script>

<LoginForm
	bind:email
	bind:password
	{loading}
	{error}
	logoSrc="/logo_long.png"
	projectName="Solobase"
	subtitle="Welcome back! Please login to your account."
	showSignupLink={true}
	signupUrl="/auth/signup"
	showForgotPassword={true}
	forgotPasswordUrl="/auth/forgot-password"
	showRememberMe={true}
	onSubmit={handleLogin}
/>