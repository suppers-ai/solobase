<script lang="ts">
	import { Eye, EyeOff, Mail, Lock, LogIn } from 'lucide-svelte';

	export let email: string = '';
	export let password: string = '';
	export let loading: boolean = false;
	export let error: string = '';
	export let logoSrc: string = '/logo_long.png';
	export let projectName: string = 'Project';
	export let subtitle: string = 'Welcome back! Please login to your account.';
	export let showSignupLink: boolean = true;
	export let signupUrl: string = '/signup';
	export let showForgotPassword: boolean = true;
	export let forgotPasswordUrl: string = '/forgot-password';
	export let showRememberMe: boolean = true;
	export let showOAuthButtons: boolean = true;
	export let onSubmit: (email: string, password: string) => Promise<void> | void = async () => {};
	export let onOAuthLogin: (provider: string) => void = () => {};

	let showPassword = false;
	let rememberMe = false;

	function togglePasswordVisibility() {
		showPassword = !showPassword;
	}

	async function handleSubmit() {
		await onSubmit(email, password);
	}

	function handleOAuthLogin(provider: string) {
		onOAuthLogin(provider);
	}
</script>

<div class="login-page">
	<div class="login-container">
		<!-- Logo Section -->
		<div class="login-logo">
			<img src={logoSrc} alt={projectName} class="logo-image" />
			<p class="login-subtitle">{subtitle}</p>
		</div>
		
		<!-- Error Message -->
		{#if error}
			<div class="login-error">
				<svg width="20" height="20" viewBox="0 0 20 20" fill="currentColor">
					<path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd"/>
				</svg>
				{error}
			</div>
		{/if}
		
		<!-- Login Form -->
		<form on:submit|preventDefault={handleSubmit} class="login-form">
			<div class="form-group">
				<label for="email" class="form-label">
					<Mail size={16} />
					Email Address
				</label>
				<input
					id="email"
					type="email"
					class="form-input"
					bind:value={email}
					placeholder="admin@example.com"
					required
					disabled={loading}
					autocomplete="email"
				/>
			</div>
			
			<div class="form-group">
				<label for="password" class="form-label">
					<Lock size={16} />
					Password
				</label>
				<div class="password-input-container">
					{#if showPassword}
						<input
							id="password"
							type="text"
							class="form-input with-icon"
							bind:value={password}
							placeholder="Enter your password"
							required
							disabled={loading}
							autocomplete="current-password"
						/>
					{:else}
						<input
							id="password"
							type="password"
							class="form-input with-icon"
							bind:value={password}
							placeholder="Enter your password"
							required
							disabled={loading}
							autocomplete="current-password"
						/>
					{/if}
					<button
						type="button"
						class="password-toggle"
						on:click={togglePasswordVisibility}
						tabindex="-1"
						aria-label={showPassword ? 'Hide password' : 'Show password'}
					>
						{#if showPassword}
							<EyeOff size={20} />
						{:else}
							<Eye size={20} />
						{/if}
					</button>
				</div>
			</div>
			
			{#if showRememberMe || showForgotPassword}
				<div class="form-actions">
					{#if showRememberMe}
						<label class="remember-me">
							<input type="checkbox" bind:checked={rememberMe} />
							<span>Remember me</span>
						</label>
					{/if}
					{#if showForgotPassword}
						<a href={forgotPasswordUrl} class="forgot-link">Forgot password?</a>
					{/if}
				</div>
			{/if}
			
			<button
				type="submit"
				class="login-button"
				disabled={loading}
			>
				{#if loading}
					<div class="spinner"></div>
					<span>Logging in...</span>
				{:else}
					<LogIn size={20} />
					<span>Login</span>
				{/if}
			</button>
		</form>

		<!-- OAuth Login Section -->
		{#if showOAuthButtons}
			<div class="oauth-divider">
				<span>Or continue with</span>
			</div>

			<div class="oauth-buttons">
				<button
					type="button"
					class="oauth-button google"
					on:click={() => handleOAuthLogin('google')}
					disabled={loading}
				>
					<svg viewBox="0 0 24 24" width="20" height="20">
						<path fill="#4285F4" d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"/>
						<path fill="#34A853" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"/>
						<path fill="#FBBC05" d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"/>
						<path fill="#EA4335" d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"/>
					</svg>
					<span>Google</span>
				</button>

				<button
					type="button"
					class="oauth-button microsoft"
					on:click={() => handleOAuthLogin('microsoft')}
					disabled={loading}
				>
					<svg viewBox="0 0 24 24" width="20" height="20">
						<path fill="#f25022" d="M1 1h10v10H1z"/>
						<path fill="#00a4ef" d="M13 1h10v10H13z"/>
						<path fill="#7fba00" d="M1 13h10v10H1z"/>
						<path fill="#ffb900" d="M13 13h10v10H13z"/>
					</svg>
					<span>Microsoft</span>
				</button>

				<button
					type="button"
					class="oauth-button facebook"
					on:click={() => handleOAuthLogin('facebook')}
					disabled={loading}
				>
					<svg viewBox="0 0 24 24" width="20" height="20">
						<path fill="#1877F2" d="M24 12.073c0-6.627-5.373-12-12-12s-12 5.373-12 12c0 5.99 4.388 10.954 10.125 11.854v-8.385H7.078v-3.47h3.047V9.43c0-3.007 1.792-4.669 4.533-4.669 1.312 0 2.686.235 2.686.235v2.953H15.83c-1.491 0-1.956.925-1.956 1.874v2.25h3.328l-.532 3.47h-2.796v8.385C19.612 23.027 24 18.062 24 12.073z"/>
					</svg>
					<span>Facebook</span>
				</button>
			</div>
		{/if}

		<!-- Sign Up Link -->
		{#if showSignupLink}
			<div class="signup-link">
				Don't have an account?
				<a href={signupUrl}>Sign up now</a>
			</div>
		{/if}
	</div>
</div>

<style>
	@import '../css/login.css';
</style>