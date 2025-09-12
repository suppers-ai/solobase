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
	export let onSubmit: (email: string, password: string) => Promise<void> | void = async () => {};
	
	let showPassword = false;
	let rememberMe = false;
	
	function togglePasswordVisibility() {
		showPassword = !showPassword;
	}
	
	async function handleSubmit() {
		await onSubmit(email, password);
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