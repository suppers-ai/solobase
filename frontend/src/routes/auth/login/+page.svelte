<script lang="ts">
	import LoginForm from '$lib/components/auth/LoginForm.svelte';
	import '$lib/css/variables.css';
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

	async function handleLogin(loginEmail: string, loginPassword: string) {
		loading = true;
		error = '';

		const success = await auth.login(loginEmail, loginPassword);

		if (success) {
			// Validate and use redirect parameter if present
			if (redirectTo && isValidRedirectUrl(redirectTo)) {
				console.log('Redirecting to:', redirectTo);
				await goto(redirectTo);
			} else {
				// Default redirect to home page
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

	function handleOAuthLogin(provider: string) {
		// Set loading state
		loading = true;
		error = '';

		// Construct OAuth login URL with callback
		const callbackUrl = `${window.location.origin}/auth/oauth/callback`;
		let oauthUrl = `/api/auth/oauth/login?provider=${provider}&callback=${encodeURIComponent(callbackUrl)}`;

		// Add redirect parameter if present
		if (redirectTo && isValidRedirectUrl(redirectTo)) {
			oauthUrl += `&redirect=${encodeURIComponent(redirectTo)}`;
		}

		// Open popup window for OAuth flow
		const popup = window.open(
			oauthUrl,
			'oauth-login',
			'width=600,height=700,scrollbars=yes,resizable=yes'
		);

		if (!popup) {
			loading = false;
			error = 'Popup blocked. Please allow popups for this site and try again.';
			return;
		}

		// Listen for messages from the popup
		const handleMessage = (event: MessageEvent) => {
			// Ensure message is from our domain
			if (event.origin !== window.location.origin) {
				return;
			}

			if (event.data?.type === 'oauth-success') {
				// OAuth successful - refresh auth state
				auth.checkAuth().then(async (success) => {
					if (success) {
						// Validate and use redirect parameter if present
						const redirectUrl = event.data.redirect;
						if (redirectUrl && isValidRedirectUrl(redirectUrl)) {
							console.log('Redirecting to:', redirectUrl);
							await goto(redirectUrl);
						} else {
							// Default redirect to home page
							await goto('/');
						}
					} else {
						error = 'Authentication verification failed. Please try again.';
						loading = false;
					}
				});
				window.removeEventListener('message', handleMessage);
			} else if (event.data?.type === 'oauth-error') {
				error = event.data.error || 'OAuth authentication failed';
				loading = false;
				window.removeEventListener('message', handleMessage);
			}
		};

		window.addEventListener('message', handleMessage);

		// Handle popup being closed without completion
		const checkClosed = setInterval(() => {
			if (popup.closed) {
				clearInterval(checkClosed);
				window.removeEventListener('message', handleMessage);
				// Only set error if we haven't already handled success/error
				if (loading) {
					error = 'Authentication was cancelled';
					loading = false;
				}
			}
		}, 1000);
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
	showOAuthButtons={true}
	onSubmit={handleLogin}
	onOAuthLogin={handleOAuthLogin}
/>