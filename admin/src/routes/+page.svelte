<script lang="ts">
	import { authStore } from '$lib/stores/auth';
	import { Shield, Database, HardDrive, Puzzle, Rocket, LogIn, UserPlus, User, LayoutDashboard } from 'lucide-svelte';

	// Removed automatic redirect - let users decide where to go
	// This was causing issues with apps that embed Solobase
</script>

<svelte:head>
	<title>Welcome to Solobase</title>
</svelte:head>

<div class="welcome-page">
	<div class="welcome-container">
		<div class="logo-section">
			<img src="/logo_long.png" alt="Solobase" class="logo" />
		</div>
		
		<div class="welcome-content">
			<h1>Welcome to Solobase</h1>
			<p class="subtitle">Your modular backend platform</p>
			
			<div class="description">
				<p>
					Solobase provides a complete backend solution with authentication, 
					storage, database management, and extensible features to power your applications.
				</p>
			</div>
			
			<div class="actions">
				{#if $authStore.user}
					{#if $authStore.user.role === 'admin'}
						<a href="/admin" class="btn btn-primary">
							<LayoutDashboard size={18} />
							<span>Admin Dashboard</span>
						</a>
					{:else}
						<a href="/profile" class="btn btn-primary">
							<User size={18} />
							<span>Profile</span>
						</a>
					{/if}
				{:else}
					<a href="/auth/login" class="btn btn-primary">
						<LogIn size={18} />
						<span>Login</span>
					</a>
					<a href="/auth/signup" class="btn btn-secondary">
						<UserPlus size={18} />
						<span>Sign Up</span>
					</a>
				{/if}
			</div>
		</div>
		
		<div class="info-section">
			<div class="info-card">
				<div class="info-header">
					<Rocket size={20} />
					<h3>Getting Started</h3>
				</div>
				<p>This is the default Solobase landing page. When using Solobase as a package, you can override this with your own application.</p>
			</div>
		</div>
	</div>
</div>

<style>
	.welcome-page {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #f0f0f0;
		padding: 2rem;
	}
	
	.welcome-container {
		width: 100%;
		max-width: 900px;
		background: white;
		border-radius: 12px;
		border: 1px solid #e2e8f0;
		overflow: hidden;
	}
	
	.logo-section {
		padding: 2.5rem 2rem 1rem;
		text-align: center;
	}
	
	.logo {
		height: 60px;
		width: auto;
		margin: 0 auto;
		display: block;
	}
	
	.welcome-content {
		padding: 3rem 2rem;
		text-align: center;
	}
	
	.welcome-content h1 {
		margin: 0 0 0.5rem 0;
		font-size: 2.5rem;
		font-weight: 700;
		color: #1a202c;
	}
	
	.subtitle {
		margin: 0 0 2rem 0;
		font-size: 1.25rem;
		color: #64748b;
	}
	
	.description {
		max-width: 600px;
		margin: 0 auto 2rem;
	}
	
	.description p {
		font-size: 1rem;
		line-height: 1.6;
		color: #475569;
	}
	
	.actions {
		display: flex;
		gap: 1rem;
		justify-content: center;
		flex-wrap: wrap;
	}
	
	.btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1.5rem;
		border-radius: 8px;
		font-size: 0.9375rem;
		font-weight: 500;
		text-decoration: none;
		transition: all 0.2s;
		border: none;
		cursor: pointer;
	}
	
	.btn-primary {
		background: #189AB4;
		color: white;
	}
	
	.btn-primary:hover {
		background: #0D5E70;
		transform: translateY(-1px);
	}
	
	.btn-secondary {
		background: white;
		color: #475569;
		border: 1px solid #e2e8f0;
	}
	
	.btn-secondary:hover {
		background: #f8fafc;
		border-color: #cbd5e1;
	}
	
	.info-section {
		background: #f9fafb;
		padding: 2rem;
	}
	
	.info-card {
		background: white;
		padding: 1.5rem;
		border-radius: 8px;
		margin-bottom: 2rem;
		border: 1px solid #e2e8f0;
	}
	
	.info-header {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 0.75rem;
		color: #189AB4;
	}
	
	.info-card h3 {
		margin: 0;
		font-size: 1.125rem;
		color: #1a202c;
	}
	
	.info-card p {
		margin: 0;
		color: #64748b;
		line-height: 1.5;
		font-size: 0.875rem;
	}
	
	.features-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1rem;
	}
	
	.feature {
		background: white;
		padding: 1.5rem;
		border-radius: 8px;
		text-align: center;
		border: 1px solid #e2e8f0;
		transition: all 0.2s;
	}
	
	.feature:hover {
		transform: translateY(-2px);
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
		border-color: #189AB4;
	}
	
	.feature-icon {
		width: 48px;
		height: 48px;
		margin: 0 auto 1rem;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #f0f9ff;
		border-radius: 12px;
		color: #189AB4;
	}
	
	.feature:hover .feature-icon {
		background: #189AB4;
		color: white;
	}
	
	.feature h4 {
		margin: 0 0 0.5rem 0;
		font-size: 0.9375rem;
		font-weight: 600;
		color: #1a202c;
	}
	
	.feature p {
		margin: 0;
		font-size: 0.813rem;
		color: #64748b;
		line-height: 1.4;
	}
	
	@media (max-width: 640px) {
		.welcome-content h1 {
			font-size: 2rem;
		}
		
		.subtitle {
			font-size: 1rem;
		}
		
		.actions {
			flex-direction: column;
		}
		
		.btn {
			width: 100%;
			text-align: center;
		}
		
		.features-grid {
			grid-template-columns: 1fr;
		}
	}
</style>