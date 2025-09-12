<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { 
		Package, Building2, Package2, Calculator,
		Database, ChevronRight,
		ShoppingBag
	} from 'lucide-svelte';
	import { api } from '$lib/api';
	import { requireAdmin } from '$lib/utils/auth';
	let loading = false;
	let userProductCreationEnabled = false; // Disabled by default
	
	// Stats
	let stats = {
		groupTypes: 0,
		productTypes: 0,
		pricingTemplates: 0
	};

	// Configuration sections
	const configSections = [
		{
			title: 'Group Types',
			description: 'Configure types of groups that can own products',
			icon: Building2,
			path: '/admin/extensions/products/group-types',
			color: 'purple',
			count: 0
		},
		{
			title: 'Product Types',
			description: 'Define different types of products with custom fields',
			icon: Package2,
			path: '/admin/extensions/products/product-types',
			color: 'green',
			count: 0
		},
		{
			title: 'Pricing & Formulas',
			description: 'Create pricing templates and formulas',
			icon: Calculator,
			path: '/admin/extensions/products/pricing',
			color: 'orange',
			count: 0
		}
	];

	onMount(async () => {
		if (!requireAdmin()) return;
		await loadStats();
	});

	async function loadStats() {
		try {
			loading = true;
			
			// Fetch counts from API
			const [groupTypesRes, productTypesRes, templatesRes] = await Promise.all([
				api.get('/products/group-types'),
				api.get('/products/product-types'),
				api.get('/products/pricing-templates')
			]);
			
			stats = {
				groupTypes: groupTypesRes?.length || 0,
				productTypes: productTypesRes?.length || 0,
				pricingTemplates: templatesRes?.length || 0
			};
			
			// Update counts in config sections
			configSections[0].count = stats.groupTypes;
			configSections[1].count = stats.productTypes;
			configSections[2].count = stats.pricingTemplates;
		} catch (error) {
			console.error('Failed to load stats:', error);
		} finally {
			loading = false;
		}
	}

	function navigateTo(path: string) {
		goto(path);
	}

	function getColorClasses(color: string) {
		switch (color) {
			case 'cyan': return 'bg-cyan-100 text-cyan-600';
			case 'purple': return 'bg-purple-100 text-purple-600';
			case 'green': return 'bg-green-100 text-green-600';
			case 'orange': return 'bg-orange-100 text-orange-600';
			default: return 'bg-gray-100 text-gray-600';
		}
	}
</script>

<div class="page-container">
	<!-- Header -->
	<div class="page-header">
		<div class="header-content">
			<div class="header-left">
				<div class="header-title">
					<Package size={24} />
					<h1>Products & Pricing System</h1>
				</div>
				<p class="header-subtitle">Configure variables, entities, products, and pricing formulas</p>
			</div>
			<div class="header-actions">
				<button class="btn btn-secondary" on:click={() => goto('/profile/groups')}>
					<Database size={16} />
					My Groups
				</button>
			</div>
		</div>
	</div>

	<!-- Configuration Sections -->
	<div class="config-grid">
		{#each configSections as section}
			<div class="config-card" on:click={() => navigateTo(section.path)}>
				<div class="config-card-body">
					<div class="config-card-header">
						<div class="config-icon {getColorClasses(section.color)}">
							<svelte:component this={section.icon} size={20} />
						</div>
						<ChevronRight size={20} class="chevron-icon" />
					</div>
					<h3 class="config-title">{section.title}</h3>
					<p class="config-description">{section.description}</p>
					<div class="config-count">
						<span class="count-badge">{section.count}</span>
						<span class="count-label">configured</span>
					</div>
				</div>
			</div>
		{/each}
	</div>

	<!-- User Section -->
	<div class="user-section">
		<div class="section-divider">
			<div class="divider-line"></div>
			<span class="divider-text">User Settings</span>
			<div class="divider-line"></div>
		</div>
		
		<div class="user-settings">
			<div class="user-settings-content">
				<label class="checkbox-label">
					<input type="checkbox" bind:checked={userProductCreationEnabled} />
					<span>Enable users to create their own groups and products</span>
				</label>
				<p class="settings-description">
					{#if userProductCreationEnabled}
						Users can create groups and manage their products. Currently enabled.
					{:else}
						Only administrators can create and manage products. User access is disabled.
					{/if}
				</p>
			</div>
			<div class="user-settings-actions">
				<button class="btn btn-primary" on:click={() => goto('/profile/products')}>
					<ShoppingBag size={16} />
					View Your Products
				</button>
			</div>
		</div>
	</div>
</div>

<style>
	.page-container {
		padding: 1.5rem;
		max-width: 1400px;
		margin: 0 auto;
	}

	.page-header {
		background: white;
		border-radius: 0.5rem;
		padding: 1.5rem;
		margin-bottom: 1.5rem;
		border: 1px solid #e5e7eb;
	}

	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
	}

	.header-title {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 0.5rem;
	}

	.header-title h1 {
		font-size: 1.5rem;
		font-weight: 600;
		color: #111827;
		margin: 0;
	}

	.header-subtitle {
		color: #6b7280;
		font-size: 0.875rem;
		margin: 0;
	}

	.header-actions {
		display: flex;
		gap: 0.75rem;
	}

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: 1rem;
		margin-bottom: 1.5rem;
	}

	.stat-card {
		background: white;
		border-radius: 0.5rem;
		padding: 1.25rem;
		border: 1px solid #e5e7eb;
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.stat-icon {
		width: 48px;
		height: 48px;
		border-radius: 0.5rem;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.stat-label {
		font-size: 0.875rem;
		color: #6b7280;
		margin: 0 0 0.25rem 0;
	}

	.stat-value {
		font-size: 1.5rem;
		font-weight: 600;
		color: #111827;
		margin: 0;
	}

	.config-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
		gap: 1.5rem;
		margin-bottom: 2rem;
	}

	.config-card {
		background: white;
		border-radius: 0.75rem;
		border: 1px solid #e5e7eb;
		cursor: pointer;
		transition: all 0.3s;
		overflow: hidden;
	}

	.config-card:hover {
		transform: translateY(-2px);
		box-shadow: 0 10px 25px rgba(0, 0, 0, 0.1);
		border-color: #06b6d4;
	}

	.config-card-body {
		padding: 1.5rem;
	}

	.config-card-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}

	.config-icon {
		width: 48px;
		height: 48px;
		border-radius: 0.5rem;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.chevron-icon {
		color: #9ca3af;
		transition: transform 0.3s;
	}

	.config-card:hover .chevron-icon {
		transform: translateX(4px);
		color: #06b6d4;
	}

	.config-title {
		font-size: 1.125rem;
		font-weight: 600;
		color: #111827;
		margin: 0 0 0.5rem 0;
	}

	.config-description {
		font-size: 0.875rem;
		color: #6b7280;
		margin: 0 0 1rem 0;
		line-height: 1.5;
	}

	.config-count {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.count-badge {
		display: inline-block;
		padding: 0.25rem 0.75rem;
		background: #f3f4f6;
		color: #111827;
		border-radius: 9999px;
		font-size: 0.875rem;
		font-weight: 600;
	}

	.count-label {
		font-size: 0.875rem;
		color: #6b7280;
	}

	.user-section {
		margin-top: 2rem;
	}

	.section-divider {
		display: flex;
		align-items: center;
		gap: 1rem;
		margin: 2rem 0;
	}

	.divider-line {
		flex: 1;
		height: 1px;
		background: #e5e7eb;
	}

	.divider-text {
		font-size: 0.875rem;
		font-weight: 600;
		color: #6b7280;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.user-settings {
		background: white;
		border-radius: 0.5rem;
		padding: 1.5rem;
		border: 1px solid #e5e7eb;
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 2rem;
	}

	.user-settings-content {
		flex: 1;
	}

	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		font-weight: 500;
		color: #111827;
		cursor: pointer;
		margin-bottom: 0.5rem;
	}

	.checkbox-label input[type="checkbox"] {
		width: 18px;
		height: 18px;
		cursor: pointer;
		accent-color: #06b6d4;
	}

	.settings-description {
		font-size: 0.875rem;
		color: #6b7280;
		margin: 0;
		margin-left: 1.875rem;
	}

	.user-settings-actions {
		display: flex;
		gap: 0.75rem;
		flex-shrink: 0;
	}



	.btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		border: none;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-primary {
		background: #06b6d4;
		color: white;
	}

	.btn-primary:hover {
		background: #0891b2;
	}

	.btn-secondary {
		background: white;
		color: #374151;
		border: 1px solid #e5e7eb;
	}

	.btn-secondary:hover {
		background: #f9fafb;
	}

	@media (max-width: 768px) {
		.config-grid {
			grid-template-columns: 1fr;
		}
		
		.user-settings {
			flex-direction: column;
			align-items: flex-start;
		}
		
		.user-settings-actions {
			width: 100%;
		}
		
		.user-settings-actions .btn {
			width: 100%;
			justify-content: center;
		}
	}
</style>

