<script lang="ts">
	import { api } from '$lib/api';
	import { goto } from '$app/navigation';
	import { Eye, EyeOff, Mail, Lock, UserPlus, User } from 'lucide-svelte';
	
	let email = '';
	let password = '';
	let confirmPassword = '';
	let fullName = '';
	let loading = false;
	let error = '';
	let showPassword = false;
	let showConfirmPassword = false;
	
	async function handleSignup() {
		loading = true;
		error = '';
		
		if (password !== confirmPassword) {
			error = 'Passwords do not match';
			loading = false;
			return;
		}
		
		if (password.length < 8) {
			error = 'Password must be at least 8 characters';
			loading = false;
			return;
		}
		
		const response = await api.signup({ email, password, fullName });
		
		if (response.error) {
			error = response.error;
		} else {
			goto('/auth/login?registered=true');
		}
		
		loading = false;
	}
	
	function togglePasswordVisibility() {
		showPassword = !showPassword;
	}
	
	function toggleConfirmPasswordVisibility() {
		showConfirmPassword = !showConfirmPassword;
	}
</script>

<div class="signup-page">
	<div class="signup-container">
		<!-- Logo Section -->
		<div class="signup-logo">
			<img src="/logo_long.png" alt="Solobase" class="logo-image" />
			<p class="signup-subtitle">Create your account to get started</p>
		</div>
		
		<!-- Error Message -->
		{#if error}
			<div class="signup-error">
				<svg width="20" height="20" viewBox="0 0 20 20" fill="currentColor">
					<path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd"/>
				</svg>
				{error}
			</div>
		{/if}
		
		<!-- Signup Form -->
		<form on:submit|preventDefault={handleSignup} class="signup-form">
			<div class="form-group">
				<label for="fullName" class="form-label">
					<User size={16} />
					Full Name
				</label>
				<input
					id="fullName"
					type="text"
					class="form-input"
					bind:value={fullName}
					placeholder="John Doe"
					disabled={loading}
					autocomplete="name"
				/>
			</div>
			
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
					placeholder="john@example.com"
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
							placeholder="Min. 8 characters"
							required
							disabled={loading}
							autocomplete="new-password"
						/>
					{:else}
						<input
							id="password"
							type="password"
							class="form-input with-icon"
							bind:value={password}
							placeholder="Min. 8 characters"
							required
							disabled={loading}
							autocomplete="new-password"
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
			
			<div class="form-group">
				<label for="confirmPassword" class="form-label">
					<Lock size={16} />
					Confirm Password
				</label>
				<div class="password-input-container">
					{#if showConfirmPassword}
						<input
							id="confirmPassword"
							type="text"
							class="form-input with-icon"
							bind:value={confirmPassword}
							placeholder="Re-enter your password"
							required
							disabled={loading}
							autocomplete="new-password"
						/>
					{:else}
						<input
							id="confirmPassword"
							type="password"
							class="form-input with-icon"
							bind:value={confirmPassword}
							placeholder="Re-enter your password"
							required
							disabled={loading}
							autocomplete="new-password"
						/>
					{/if}
					<button
						type="button"
						class="password-toggle"
						on:click={toggleConfirmPasswordVisibility}
						tabindex="-1"
						aria-label={showConfirmPassword ? 'Hide password' : 'Show password'}
					>
						{#if showConfirmPassword}
							<EyeOff size={20} />
						{:else}
							<Eye size={20} />
						{/if}
					</button>
				</div>
			</div>
			
			<div class="terms-section">
				<label class="terms-checkbox">
					<input type="checkbox" required />
					<span>I agree to the <a href="/terms" class="terms-link">Terms of Service</a> and <a href="/privacy" class="terms-link">Privacy Policy</a></span>
				</label>
			</div>
			
			<button
				type="submit"
				class="signup-button"
				disabled={loading}
			>
				{#if loading}
					<div class="spinner"></div>
					<span>Creating account...</span>
				{:else}
					<UserPlus size={20} />
					<span>Sign Up</span>
				{/if}
			</button>
		</form>
		
		<!-- Login Link -->
		<div class="login-link">
			Already have an account? 
			<a href="/auth/login">Login here</a>
		</div>
	</div>
</div>

<style>
	.signup-page {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #f0f0f0;
		padding: 1rem;
	}
	
	.signup-container {
		width: 100%;
		max-width: 420px;
		background: white;
		border: 1px solid #e2e8f0;
		border-radius: 12px;
		padding: 2.5rem;
		animation: slideUp 0.4s ease-out;
	}
	
	@keyframes slideUp {
		from {
			opacity: 0;
			transform: translateY(20px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
	
	.signup-logo {
		text-align: center;
		margin-bottom: 2rem;
	}
	
	.logo-image {
		height: 60px;
		width: auto;
		margin: 0 auto 1rem auto;
		display: block;
	}
	
	.signup-subtitle {
		color: #6b7280;
		font-size: 0.875rem;
		margin: 0;
	}
	
	.signup-error {
		background: #fee2e2;
		color: #dc2626;
		padding: 0.75rem 1rem;
		border-radius: 8px;
		margin-bottom: 1.5rem;
		font-size: 0.875rem;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		animation: shake 0.3s ease-in-out;
	}
	
	@keyframes shake {
		0%, 100% { transform: translateX(0); }
		25% { transform: translateX(-5px); }
		75% { transform: translateX(5px); }
	}
	
	.signup-form {
		margin-bottom: 1.5rem;
	}
	
	.form-group {
		margin-bottom: 1.25rem;
	}
	
	.form-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
		margin-bottom: 0.5rem;
	}
	
	.form-input {
		width: 100%;
		padding: 0.75rem 1rem;
		border: 1px solid #d1d5db;
		border-radius: 8px;
		font-size: 0.875rem;
		transition: all 0.2s;
		background: white;
		color: #1f2937;
	}
	
	.form-input:focus {
		outline: none;
		border-color: #3b82f6;
		box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
	}
	
	.form-input:disabled {
		background: #f9fafb;
		cursor: not-allowed;
		opacity: 0.7;
	}
	
	.form-input::placeholder {
		color: #9ca3af;
	}
	
	.password-input-container {
		position: relative;
		display: flex;
		align-items: center;
	}
	
	.form-input.with-icon {
		padding-right: 3rem;
	}
	
	.password-toggle {
		position: absolute;
		right: 0.75rem;
		background: none;
		border: none;
		color: #6b7280;
		cursor: pointer;
		padding: 0.5rem;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: color 0.2s;
	}
	
	.password-toggle:hover {
		color: #374151;
	}
	
	.password-toggle:focus {
		outline: none;
	}
	
	.terms-section {
		margin-bottom: 1.5rem;
	}
	
	.terms-checkbox {
		display: flex;
		align-items: flex-start;
		gap: 0.5rem;
		font-size: 0.875rem;
		color: #6b7280;
		cursor: pointer;
	}
	
	.terms-checkbox input[type="checkbox"] {
		width: 1rem;
		height: 1rem;
		margin-top: 0.125rem;
		cursor: pointer;
		flex-shrink: 0;
	}
	
	.terms-link {
		color: #3b82f6;
		text-decoration: none;
		transition: color 0.2s;
	}
	
	.terms-link:hover {
		color: #2563eb;
		text-decoration: underline;
	}
	
	.signup-button {
		width: 100%;
		padding: 0.875rem 1.5rem;
		background: #3b82f6;
		color: white;
		border: none;
		border-radius: 8px;
		font-size: 0.9375rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
	}
	
	.signup-button:hover:not(:disabled) {
		background: #2563eb;
		transform: translateY(-1px);
	}
	
	.signup-button:active:not(:disabled) {
		transform: translateY(0);
	}
	
	.signup-button:disabled {
		cursor: not-allowed;
		opacity: 0.7;
	}
	
	.spinner {
		width: 20px;
		height: 20px;
		border: 2px solid rgba(255, 255, 255, 0.3);
		border-top-color: white;
		border-radius: 50%;
		animation: spin 0.6s linear infinite;
	}
	
	@keyframes spin {
		to { transform: rotate(360deg); }
	}
	
	.login-link {
		text-align: center;
		font-size: 0.875rem;
		color: #6b7280;
	}
	
	.login-link a {
		color: #3b82f6;
		text-decoration: none;
		font-weight: 600;
		transition: color 0.2s;
	}
	
	.login-link a:hover {
		color: #2563eb;
		text-decoration: underline;
	}
	
	/* Responsive adjustments */
	@media (max-width: 480px) {
		.signup-container {
			padding: 2rem;
		}
	}
</style>