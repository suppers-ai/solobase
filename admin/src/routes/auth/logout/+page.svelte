<script lang="ts">
	import { onMount } from 'svelte';
	import { auth } from '$lib/stores/auth';
	import { goto } from '$app/navigation';
	import { LogOut } from 'lucide-svelte';
	
	let message = 'Logging out...';
	let isLoading = true;
	
	onMount(async () => {
		// Perform logout
		try {
			await auth.logout();
			
			// Clear any remaining localStorage items
			if (typeof window !== 'undefined') {
				localStorage.removeItem('auth_token');
			}
			
			message = 'You have been logged out successfully.';
			isLoading = false;
			
			// Redirect to login after a short delay
			setTimeout(() => {
				goto('/auth/login');
			}, 1500);
		} catch (error) {
			console.error('Logout error:', error);
			message = 'Logout completed.';
			isLoading = false;
			
			// Even if there's an error, clear session and redirect
			if (typeof window !== 'undefined') {
				localStorage.removeItem('auth_token');
			}
			setTimeout(() => {
				goto('/auth/login');
			}, 1500);
		}
	});
</script>

<div class="logout-page">
	<div class="logout-container">
		<!-- Logo Section -->
		<div class="logout-logo">
			<img src="/logo_long.png" alt="Solobase" class="logo-image" />
		</div>
		
		<div class="logout-content">
			<div class="logout-icon">
				<LogOut size={32} />
			</div>
			
			<h1 class="logout-title">
				{isLoading ? 'Signing Out' : 'Signed Out'}
			</h1>
			
			<p class="logout-message">{message}</p>
			
			{#if isLoading}
				<div class="spinner-container">
					<div class="spinner"></div>
				</div>
			{:else}
				<p class="redirect-message">Redirecting to login...</p>
			{/if}
		</div>
		
		<div class="logout-footer">
			<a href="/auth/login" class="login-link">
				Click here if you're not redirected
			</a>
		</div>
	</div>
</div>

<style>
	.logout-page {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #f0f0f0;
		padding: 1rem;
	}
	
	.logout-container {
		width: 100%;
		max-width: 420px;
		background: white;
		border: 1px solid #e2e8f0;
		border-radius: 12px;
		padding: 2.5rem;
		text-align: center;
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
	
	.logout-logo {
		margin-bottom: 2rem;
	}
	
	.logo-image {
		height: 60px;
		width: auto;
		margin: 0 auto;
	}
	
	.logout-content {
		margin-bottom: 1.5rem;
	}
	
	.logout-icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 64px;
		height: 64px;
		background: #f3f4f6;
		border-radius: 50%;
		margin-bottom: 1rem;
		color: #6b7280;
	}
	
	.logout-title {
		font-size: 1.5rem;
		font-weight: 700;
		color: #1e293b;
		margin: 0 0 0.5rem 0;
	}
	
	.logout-message {
		color: #64748b;
		font-size: 0.875rem;
		margin: 0 0 1.5rem 0;
	}
	
	.spinner-container {
		display: flex;
		justify-content: center;
	}
	
	.spinner {
		width: 32px;
		height: 32px;
		border: 3px solid #e2e8f0;
		border-top-color: #189AB4;
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}
	
	@keyframes spin {
		to { transform: rotate(360deg); }
	}
	
	.redirect-message {
		font-size: 0.875rem;
		color: #94a3b8;
		margin: 0;
	}
	
	.logout-footer {
		margin-top: 1rem;
	}
	
	.login-link {
		font-size: 0.875rem;
		color: #189AB4;
		text-decoration: none;
		transition: color 0.2s;
	}
	
	.login-link:hover {
		color: #0284c7;
		text-decoration: underline;
	}
	
	/* Responsive adjustments */
	@media (max-width: 480px) {
		.logout-container {
			padding: 2rem;
		}
		
		.logout-title {
			font-size: 1.25rem;
		}
	}
</style>