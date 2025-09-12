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
	
	async function handleLogin(loginEmail: string, loginPassword: string) {
		loading = true;
		error = '';
		
		const success = await auth.login(loginEmail, loginPassword);
		
		if (success) {
			// Always check for redirect parameter first
			if (redirectTo) {
				console.log('Redirecting to:', redirectTo);
				// Handle both absolute and relative URLs
				if (redirectTo.startsWith('http')) {
					// Absolute URL - navigate directly
					window.location.href = redirectTo;
					return; // Ensure we don't continue
				} else {
					// Relative URL - use goto
					await goto(redirectTo);
					return; // Ensure we don't continue
				}
			} else {
				// Default redirect to home page
				console.log('No redirect param, going to /');
				await goto('/');
				return;
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