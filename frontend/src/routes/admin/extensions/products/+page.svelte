<script lang="ts">
	import { onMount } from 'svelte';
	import { api, ErrorHandler } from '$lib/api';
	import { formatDateTime } from '$lib/utils/formatters';
	import { toasts } from '$lib/stores/toast';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import { Package, Building2, ShoppingCart } from 'lucide-svelte';

	// Tab management
	let activeTab = 'overview';

	// Stats
	let stats = {
		totalProducts: 0,
		totalPurchases: 0,
		totalRevenue: 0,
		activeProducts: 0
	};

	// Products data
	let products: any[] = [];
	let productsLoading = false;
	let productsError: string | null = null;

	// Groups data
	let groups: any[] = [];
	let groupsLoading = false;
	let groupsError: string | null = null;

	// Purchases data for the Purchases tab
	interface Purchase {
		id: number;
		userId: string;
		status: string;
		totalCents: number;
		currency: string;
		customerName?: string;
		customerEmail?: string;
		createdAt: string;
		providerSessionId: string;
		lineItems: Array<{
			productName: string;
			quantity: number;
			totalPrice: number;
		}>;
	}

	let purchases: Purchase[] = [];
	let purchasesTotal = 0;
	let purchasesLoading = true;
	let purchasesError: string | null = null;
	let purchasesCurrentPage = 0;
	const purchasesLimit = 20;

	// Confirmation dialogs state
	let showRefundConfirm = false;
	let showApproveConfirm = false;
	let purchaseToRefund: number | null = null;
	let purchaseToApprove: number | null = null;
	let refundReason = '';

	// Configuration counts
	let configCounts = {
		productTypes: 0,
		groupTypes: 0,
		pricingTemplates: 0,
		variables: 0
	};

	// Payment provider status
	let providerStatus: ProviderStatus = {
		configured: false,
		provider: 'none',
		mode: 'none',
		configuredProvider: 'stripe',
		availableProviders: []
	};


	// Product stats response
	interface ProductStats {
		totalProducts: number;
		activeProducts: number;
	}

	// Purchase stats response
	interface PurchaseStats {
		totalPurchases?: number;
		totalSpent?: number;
	}

	// Provider status response
	interface ProviderStatus {
		configured: boolean;
		provider: string;
		mode: string;
		configuredProvider?: string;
		availableProviders?: string[];
	}

	// Purchases list response
	interface PurchasesResponse {
		purchases: Purchase[];
		total: number;
	}

	type ConfigCountKey = 'productTypes' | 'groupTypes' | 'pricingTemplates' | 'variables';

	// Configuration sections with colors
	const configSections: Array<{
		title: string;
		description: string;
		path: string;
		color: string;
		count: number;
		countKey: ConfigCountKey;
	}> = [
		{
			title: 'Product Types',
			description: 'Define templates for different types of products',
			path: '/admin/extensions/products/product-types',
			color: 'blue',
			count: 0,
			countKey: 'productTypes'
		},
		{
			title: 'Group Types',
			description: 'Configure business entity types (restaurants, stores, etc)',
			path: '/admin/extensions/products/group-types',
			color: 'green',
			count: 0,
			countKey: 'groupTypes'
		},
		{
			title: 'Pricing Templates',
			description: 'Create reusable pricing formulas and rules',
			path: '/admin/extensions/products/pricing',
			color: 'purple',
			count: 0,
			countKey: 'pricingTemplates'
		},
		{
			title: 'Variables',
			description: 'Manage pricing variables used in formulas',
			path: '/admin/extensions/products/variables',
			color: 'yellow',
			count: 0,
			countKey: 'variables'
		}
	];

	async function loadStats() {
		try {
			const data = await api.get<ProductStats>('/admin/ext/products/stats');
			stats = {
				totalProducts: data.totalProducts || 0,
				totalPurchases: 0,
				totalRevenue: 0,
				activeProducts: data.activeProducts || 0
			};

			// Load purchase stats
			const purchaseStats = await api.get<PurchaseStats>('/ext/products/purchases/stats');
			stats.totalPurchases = purchaseStats.totalPurchases || 0;
			stats.totalRevenue = purchaseStats.totalSpent || 0;
		} catch (err) {
			console.error('Error loading stats:', err);
		}
	}

	async function loadProviderStatus() {
		try {
			const status = await api.get<ProviderStatus>('/admin/ext/products/provider/status');
			providerStatus = status;
		} catch (err) {
			console.error('Error loading provider status:', err);
		}
	}

	async function loadConfigCounts() {
		try {
			// Load counts for each configuration type
			const [productTypes, groupTypes, pricingTemplates, variables] = await Promise.all([
				api.get('/admin/ext/products/product-types'),
				api.get('/admin/ext/products/group-types'),
				api.get('/admin/ext/products/pricing-templates'),
				api.get('/admin/ext/products/variables')
			]);

			configCounts = {
				productTypes: Array.isArray(productTypes) ? productTypes.length : 0,
				groupTypes: Array.isArray(groupTypes) ? groupTypes.length : 0,
				pricingTemplates: Array.isArray(pricingTemplates) ? pricingTemplates.length : 0,
				variables: Array.isArray(variables) ? variables.length : 0
			};

			// Update counts in config sections
			configSections.forEach(section => {
				section.count = configCounts[section.countKey];
			});
		} catch (err) {
			console.error('Error loading config counts:', err);
		}
	}

	async function loadProducts() {
		productsLoading = true;
		productsError = null;
		try {
			const data = await api.get('/ext/products/products');
			products = Array.isArray(data) ? data : [];
		} catch (err) {
			productsError = 'Failed to load products';
			console.error('Error loading products:', err);
		} finally {
			productsLoading = false;
		}
	}

	async function loadGroups() {
		groupsLoading = true;
		groupsError = null;
		try {
			const data = await api.get('/admin/ext/products/groups');
			groups = Array.isArray(data) ? data : [];
		} catch (err) {
			groupsError = 'Failed to load groups';
			console.error('Error loading groups:', err);
		} finally {
			groupsLoading = false;
		}
	}

	async function loadPurchases() {
		purchasesLoading = true;
		purchasesError = null;
		try {
			const data = await api.get<PurchasesResponse>(`/admin/ext/products/purchases?limit=${purchasesLimit}&offset=${purchasesCurrentPage * purchasesLimit}`);
			purchases = data.purchases || [];
			purchasesTotal = data.total || 0;
		} catch (err) {
			purchasesError = err instanceof Error ? err.message : 'Failed to load purchases';
			console.error('Error loading purchases:', err);
		} finally {
			purchasesLoading = false;
		}
	}

	function handleRefund(purchaseId: number) {
		purchaseToRefund = purchaseId;
		refundReason = '';
		showRefundConfirm = true;
	}

	async function confirmRefund() {
		if (!purchaseToRefund) return;
		showRefundConfirm = false;

		if (!refundReason.trim()) {
			toasts.error('Please enter a refund reason');
			return;
		}

		try {
			await api.post(`/admin/ext/products/purchases/${purchaseToRefund}/refund`, { amount: 0, reason: refundReason });
			await loadPurchases();
			toasts.success('Refund processed successfully');
		} catch (err) {
			ErrorHandler.handle(err);
		}
		purchaseToRefund = null;
		refundReason = '';
	}

	function handleApprove(purchaseId: number) {
		purchaseToApprove = purchaseId;
		showApproveConfirm = true;
	}

	async function confirmApprove() {
		if (!purchaseToApprove) return;
		showApproveConfirm = false;

		try {
			await api.post(`/admin/ext/products/purchases/${purchaseToApprove}/approve`);
			await loadPurchases();
			toasts.success('Purchase approved');
		} catch (err) {
			ErrorHandler.handle(err);
		}
		purchaseToApprove = null;
	}


	function formatCurrency(cents: number, currency: string = 'USD'): string {
		return new Intl.NumberFormat('en-US', {
			style: 'currency',
			currency: currency
		}).format(cents / 100);
	}

	function getStatusColor(status: string): string {
		switch(status) {
			case 'paid': return 'text-green-600 bg-green-100';
			case 'pending': return 'text-yellow-600 bg-yellow-100';
			case 'refunded': return 'text-red-600 bg-red-100';
			case 'cancelled': return 'text-gray-600 bg-gray-100';
			case 'requires_approval': return 'text-blue-600 bg-blue-100';
			default: return 'text-gray-600 bg-gray-100';
		}
	}

	function getColorClasses(color: string) {
		switch(color) {
			case 'blue': return 'bg-blue-100 text-blue-600';
			case 'green': return 'bg-green-100 text-green-600';
			case 'purple': return 'bg-purple-100 text-purple-600';
			case 'yellow': return 'bg-yellow-100 text-yellow-600';
			default: return 'bg-gray-100 text-gray-600';
		}
	}

	// Load data based on active tab
	$: if (activeTab === 'products') loadProducts();
	$: if (activeTab === 'groups') loadGroups();
	$: if (activeTab === 'purchases') loadPurchases();
	$: if (activeTab === 'setup') loadConfigCounts();

	onMount(() => {
		loadStats();
		loadConfigCounts();
		loadProviderStatus();
	});
</script>

<div class="p-6">
	<div class="mb-6">
		<h1 class="text-2xl font-bold text-gray-900">Products Extension</h1>
		<p class="text-gray-600">Manage products, pricing, and purchases</p>
	</div>

	<!-- Tabs -->
	<div class="border-b border-gray-200 mb-6">
		<nav class="-mb-px flex space-x-8">
			<button
				on:click={() => activeTab = 'overview'}
				class="py-2 px-1 border-b-2 font-medium text-sm {activeTab === 'overview' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
			>
				Overview
			</button>
			<button
				on:click={() => activeTab = 'setup'}
				class="py-2 px-1 border-b-2 font-medium text-sm {activeTab === 'setup' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
			>
				Setup
			</button>
			<button
				on:click={() => activeTab = 'products'}
				class="py-2 px-1 border-b-2 font-medium text-sm {activeTab === 'products' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
			>
				Products
			</button>
			<button
				on:click={() => activeTab = 'groups'}
				class="py-2 px-1 border-b-2 font-medium text-sm {activeTab === 'groups' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
			>
				Groups
			</button>
			<button
				on:click={() => activeTab = 'purchases'}
				class="py-2 px-1 border-b-2 font-medium text-sm {activeTab === 'purchases' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
			>
				Purchases
			</button>
			<button
				on:click={() => activeTab = 'demo'}
				class="py-2 px-1 border-b-2 font-medium text-sm {activeTab === 'demo' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
			>
				Demo Flow
			</button>
		</nav>
	</div>

	<!-- Tab Content -->
	{#if activeTab === 'overview'}
		<!-- Overview Tab -->
		<div>
			<!-- Stats Grid -->
			<div class="grid grid-cols-1 md:grid-cols-4 gap-4">
				<div class="bg-white p-6 rounded-lg shadow">
					<div class="text-2xl font-bold text-gray-900">{stats.totalProducts}</div>
					<div class="text-sm text-gray-600">Total Products</div>
				</div>
				<div class="bg-white p-6 rounded-lg shadow">
					<div class="text-2xl font-bold text-gray-900">{stats.activeProducts}</div>
					<div class="text-sm text-gray-600">Active Products</div>
				</div>
				<div class="bg-white p-6 rounded-lg shadow">
					<div class="text-2xl font-bold text-gray-900">{stats.totalPurchases}</div>
					<div class="text-sm text-gray-600">Total Purchases</div>
				</div>
				<div class="bg-white p-6 rounded-lg shadow">
					<div class="text-2xl font-bold text-green-600">{formatCurrency(stats.totalRevenue)}</div>
					<div class="text-sm text-gray-600">Total Revenue</div>
				</div>
			</div>
		</div>

	{:else if activeTab === 'setup'}
		<!-- Setup Tab -->
		<div>
			<!-- Payment Provider Configuration -->
			<div class="bg-white rounded-lg shadow mb-6">
				<div class="p-6 border-b border-gray-200">
					<h2 class="text-lg font-semibold text-gray-900">Payment Provider Configuration</h2>
					<p class="text-sm text-gray-600 mt-1">Current payment provider status and configuration</p>
				</div>
				<div class="p-6">
					{#if providerStatus.configured}
						<div class="rounded-md bg-green-50 border border-green-200 p-4">
							<div class="flex">
								<svg class="h-5 w-5 text-green-400 mt-0.5" viewBox="0 0 20 20" fill="currentColor">
									<path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd" />
								</svg>
								<div class="ml-3">
									<h3 class="text-sm font-medium text-green-800">
										Payment Provider Configured
									</h3>
									<div class="mt-2 text-sm text-green-700">
										<p><strong>Provider:</strong> {providerStatus.provider.charAt(0).toUpperCase() + providerStatus.provider.slice(1)}</p>
										<p><strong>Mode:</strong> {providerStatus.mode === 'test' ? 'Test Mode' : 'Production Mode'}</p>
										<p class="mt-2"><strong>Environment Variable:</strong> PAYMENT_PROVIDER={providerStatus.configuredProvider}</p>
									</div>
								</div>
							</div>
						</div>
					{:else}
						<div class="rounded-md bg-amber-50 border border-amber-200 p-4">
							<div class="flex">
								<svg class="h-5 w-5 text-amber-400 mt-0.5" viewBox="0 0 20 20" fill="currentColor">
									<path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd" />
								</svg>
								<div class="ml-3">
									<h3 class="text-sm font-medium text-amber-800">
										Payment Provider Not Configured
									</h3>
									<div class="mt-2 text-sm text-amber-700">
										<p>Please set the following environment variables when starting your server:</p>
										<ul class="mt-2 space-y-1 list-disc list-inside">
											<li><code class="font-mono bg-amber-100 px-1 py-0.5 rounded">PAYMENT_PROVIDER</code> - Payment provider type (defaults to 'stripe')</li>
											<li><code class="font-mono bg-amber-100 px-1 py-0.5 rounded">STRIPE_SECRET_KEY</code> - Your Stripe secret key</li>
											<li><code class="font-mono bg-amber-100 px-1 py-0.5 rounded">STRIPE_WEBHOOK_SECRET</code> - Your webhook endpoint secret</li>
											<li><code class="font-mono bg-amber-100 px-1 py-0.5 rounded">STRIPE_PUBLISHABLE_KEY</code> - Your Stripe publishable key (optional)</li>
										</ul>
										<div class="mt-3">
											<p>Example:</p>
											<pre class="mt-1 font-mono text-xs bg-amber-100 p-2 rounded overflow-x-auto">PAYMENT_PROVIDER=stripe \
STRIPE_SECRET_KEY=sk_test_... \
STRIPE_WEBHOOK_SECRET=whsec_... \
STRIPE_PUBLISHABLE_KEY=pk_test_... \
./solobase</pre>
										</div>
										<p class="mt-3">Current configured provider: <strong>{providerStatus.configuredProvider}</strong></p>
									</div>
								</div>
							</div>
						</div>
					{/if}
				</div>
			</div>

			<!-- Configuration Grid -->
			<div class="grid grid-cols-1 md:grid-cols-2 gap-6">
				{#each configSections as section}
					<a
						href={section.path}
						class="block bg-white rounded-lg shadow hover:shadow-md transition-shadow overflow-hidden"
					>
						<div class="p-6">
							<div class="flex items-start">
								<div class="flex-shrink-0">
									<div class="w-12 h-12 rounded-lg flex items-center justify-center text-2xl font-bold {getColorClasses(section.color)}">
										{section.count}
									</div>
								</div>
								<div class="ml-4 flex-1">
									<h3 class="text-lg font-semibold text-gray-900 mb-1">{section.title}</h3>
									<p class="text-gray-600 text-sm">{section.description}</p>
								</div>
							</div>
						</div>
					</a>
				{/each}
			</div>
		</div>

	{:else if activeTab === 'products'}
		<!-- Products Tab -->
		<div class="bg-white rounded-lg shadow">
			{#if productsLoading}
				<div class="p-8 text-center">
					<div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500 mx-auto"></div>
				</div>
			{:else if productsError}
				<div class="p-4 text-red-600">{productsError}</div>
			{:else if products.length === 0}
				<EmptyState
					icon={Package}
					title="No products found"
					message="Create product types first, then add products to your groups"
				>
					<a href="/admin/extensions/products/product-types" class="btn btn-primary">
						Create your first product type →
					</a>
				</EmptyState>
			{:else}
				<table class="min-w-full divide-y divide-gray-200">
					<thead class="bg-gray-50">
						<tr>
							<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Name</th>
							<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Group</th>
							<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Base Price</th>
							<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Status</th>
							<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Created</th>
						</tr>
					</thead>
					<tbody class="bg-white divide-y divide-gray-200">
						{#each products as product}
							<tr>
								<td class="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
									{product.name}
								</td>
								<td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
									{product.group?.name || 'N/A'}
								</td>
								<td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
									${product.basePrice}
								</td>
								<td class="px-6 py-4 whitespace-nowrap">
									<span class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full {product.active ? 'bg-green-100 text-green-800' : 'bg-gray-100 text-gray-800'}">
										{product.active ? 'Active' : 'Inactive'}
									</span>
								</td>
								<td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
									{new Date(product.createdAt).toLocaleDateString()}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}
		</div>

	{:else if activeTab === 'groups'}
		<!-- Groups Tab -->
		<div class="bg-white rounded-lg shadow">
			{#if groupsLoading}
				<div class="p-8 text-center">
					<div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500 mx-auto"></div>
				</div>
			{:else if groupsError}
				<div class="p-4 text-red-600">{groupsError}</div>
			{:else if groups.length === 0}
				<EmptyState
					icon={Building2}
					title="No groups found"
					message="Create group types first, then add groups"
				>
					<a href="/admin/extensions/products/group-types" class="btn btn-primary">
						Create your first group type →
					</a>
				</EmptyState>
			{:else}
				<table class="min-w-full divide-y divide-gray-200">
					<thead class="bg-gray-50">
						<tr>
							<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Name</th>
							<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Type</th>
							<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Owner</th>
							<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Products</th>
							<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Status</th>
							<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">Created</th>
						</tr>
					</thead>
					<tbody class="bg-white divide-y divide-gray-200">
						{#each groups as group}
							<tr>
								<td class="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
									{group.name}
								</td>
								<td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
									{group.typeName || 'N/A'}
								</td>
								<td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
									{group.ownerId || 'N/A'}
								</td>
								<td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
									{group.products?.length || 0}
								</td>
								<td class="px-6 py-4 whitespace-nowrap">
									<span class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full {group.active ? 'bg-green-100 text-green-800' : 'bg-gray-100 text-gray-800'}">
										{group.active ? 'Active' : 'Inactive'}
									</span>
								</td>
								<td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
									{new Date(group.createdAt).toLocaleDateString()}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}
		</div>

	{:else if activeTab === 'purchases'}
		<!-- Purchases Tab - Full Purchase Management -->
		<div>
			{#if purchasesLoading}
				<div class="flex justify-center items-center h-64">
					<div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500"></div>
				</div>
			{:else if purchasesError}
				<div class="bg-red-50 border border-red-200 rounded-md p-4 text-red-700">
					{purchasesError}
				</div>
			{:else}
				<div class="bg-white shadow-sm rounded-lg overflow-hidden">
					<table class="min-w-full divide-y divide-gray-200">
						<thead class="bg-gray-50">
							<tr>
								<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
									ID
								</th>
								<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
									Customer
								</th>
								<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
									Products
								</th>
								<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
									Total
								</th>
								<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
									Status
								</th>
								<th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
									Date
								</th>
								<th class="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
									Actions
								</th>
							</tr>
						</thead>
						<tbody class="bg-white divide-y divide-gray-200">
							{#each purchases as purchase}
								<tr class="hover:bg-gray-50">
									<td class="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
										#{purchase.id}
									</td>
									<td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
										<div>
											{purchase.customerName || 'N/A'}
										</div>
										<div class="text-xs text-gray-400">
											{purchase.customerEmail || 'N/A'}
										</div>
									</td>
									<td class="px-6 py-4 text-sm text-gray-500">
										{#if purchase.lineItems && purchase.lineItems.length > 0}
											<div class="max-w-xs">
												{#each purchase.lineItems.slice(0, 2) as item}
													<div class="text-xs">
														{item.productName} (x{item.quantity})
													</div>
												{/each}
												{#if purchase.lineItems.length > 2}
													<div class="text-xs text-gray-400">
														+{purchase.lineItems.length - 2} more
													</div>
												{/if}
											</div>
										{:else}
											<span class="text-gray-400">No items</span>
										{/if}
									</td>
									<td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
										{formatCurrency(purchase.totalCents, purchase.currency)}
									</td>
									<td class="px-6 py-4 whitespace-nowrap">
										<span class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full {getStatusColor(purchase.status)}">
											{purchase.status}
										</span>
									</td>
									<td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
										{formatDateTime(purchase.createdAt)}
									</td>
									<td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
										<div class="flex justify-end gap-2">
											{#if purchase.status === 'requires_approval'}
												<button
													on:click={() => handleApprove(purchase.id)}
													class="text-blue-600 hover:text-blue-900"
												>
													Approve
												</button>
											{/if}
											{#if purchase.status === 'paid'}
												<button
													on:click={() => handleRefund(purchase.id)}
													class="text-red-600 hover:text-red-900"
												>
													Refund
												</button>
											{/if}
											{#if purchase.providerSessionId}
												<a
													href="https://dashboard.stripe.com/test/payments/{purchase.providerSessionId}"
													target="_blank"
													rel="noopener noreferrer"
													class="text-gray-600 hover:text-gray-900"
												>
													View in Stripe
												</a>
											{/if}
										</div>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>

					{#if purchases.length === 0}
						<EmptyState
							icon={ShoppingCart}
							title="No purchases found"
							compact
						/>
					{/if}

					{#if purchasesTotal > purchasesLimit}
						<div class="bg-gray-50 px-6 py-3 flex items-center justify-between border-t border-gray-200">
							<div class="text-sm text-gray-700">
								Showing {purchasesCurrentPage * purchasesLimit + 1} to {Math.min((purchasesCurrentPage + 1) * purchasesLimit, purchasesTotal)} of {purchasesTotal} results
							</div>
							<div class="flex gap-2">
								<button
									on:click={() => { purchasesCurrentPage--; loadPurchases(); }}
									disabled={purchasesCurrentPage === 0}
									class="px-3 py-1 border rounded text-sm {purchasesCurrentPage === 0 ? 'bg-gray-100 text-gray-400' : 'bg-white hover:bg-gray-50'}"
								>
									Previous
								</button>
								<button
									on:click={() => { purchasesCurrentPage++; loadPurchases(); }}
									disabled={(purchasesCurrentPage + 1) * purchasesLimit >= purchasesTotal}
									class="px-3 py-1 border rounded text-sm {(purchasesCurrentPage + 1) * purchasesLimit >= purchasesTotal ? 'bg-gray-100 text-gray-400' : 'bg-white hover:bg-gray-50'}"
								>
									Next
								</button>
							</div>
						</div>
					{/if}
				</div>
			{/if}
		</div>

	{:else if activeTab === 'demo'}
		<!-- Demo Flow Tab -->
		<div class="space-y-6">
			<div class="bg-white rounded-lg shadow p-6">
				<h2 class="text-lg font-semibold mb-4">Complete Purchase Flow Demo</h2>
				<p class="text-gray-600 mb-6">
					Follow this step-by-step guide to test the entire purchase system with Stripe integration.
				</p>

				<!-- Step by Step Guide -->
				<div class="space-y-4">
					<div class="border-l-4 border-blue-500 pl-4">
						<h3 class="font-semibold">Step 1: Configure Stripe</h3>
						<p class="text-sm text-gray-600 mt-1">Set these environment variables before starting:</p>
						<pre class="mt-2 p-3 bg-gray-100 rounded text-xs overflow-x-auto">
export STRIPE_SECRET_KEY=sk_test_...
export STRIPE_WEBHOOK_SECRET=whsec_...
export STRIPE_PUBLISHABLE_KEY=pk_test_...</pre>
					</div>

					<div class="border-l-4 border-green-500 pl-4">
						<h3 class="font-semibold">Step 2: Create Product Setup</h3>
						<p class="text-sm text-gray-600 mt-1">Configure your products infrastructure:</p>
						<ol class="mt-2 text-sm text-gray-600 list-decimal list-inside space-y-1">
							<li>Create a <a href="/admin/extensions/products/group-types" class="text-blue-600 hover:underline">Group Type</a> (e.g., Restaurant)</li>
							<li>Create a <a href="/admin/extensions/products/product-types" class="text-blue-600 hover:underline">Product Type</a> (e.g., Menu Item)</li>
							<li>Add some test products</li>
						</ol>
					</div>

					<div class="border-l-4 border-purple-500 pl-4">
						<h3 class="font-semibold">Step 3: Test Customer Flow</h3>
						<p class="text-sm text-gray-600 mt-1">Experience the customer journey:</p>
						<div class="mt-2 space-y-2">
							<a href="/products/checkout" target="_blank" class="inline-block px-4 py-2 bg-purple-600 text-white rounded hover:bg-purple-700">
								Go to Checkout Page
							</a>
							<p class="text-sm text-gray-600">
								→ Add items to cart<br/>
								→ Click "Proceed to Stripe Checkout"<br/>
								→ Use test card: <code class="bg-gray-100 px-1">4242 4242 4242 4242</code><br/>
								→ Complete payment<br/>
								→ Return to success page
							</p>
						</div>
					</div>

					<div class="border-l-4 border-yellow-500 pl-4">
						<h3 class="font-semibold">Step 4: Webhook Testing</h3>
						<p class="text-sm text-gray-600 mt-1">Test webhook handling with Stripe CLI:</p>
						<pre class="mt-2 p-3 bg-gray-100 rounded text-xs overflow-x-auto">
# Install Stripe CLI
brew install stripe/stripe-cli/stripe

# Login to Stripe
stripe login

# Forward webhooks to local server
stripe listen --forward-to localhost:8080/api/ext/products/webhooks/stripe

# Trigger test events
stripe trigger checkout.session.completed</pre>
					</div>

					<div class="border-l-4 border-red-500 pl-4">
						<h3 class="font-semibold">Step 5: Admin Management</h3>
						<p class="text-sm text-gray-600 mt-1">Manage purchases in the Purchases tab:</p>
						<p class="text-sm text-gray-600 mt-2">
							→ View purchase details<br/>
							→ Process refunds<br/>
							→ Approve pending purchases<br/>
							→ View in Stripe Dashboard
						</p>
					</div>
				</div>
			</div>

			<!-- Test Cards Reference -->
			<div class="bg-white rounded-lg shadow p-6">
				<h2 class="text-lg font-semibold mb-4">Stripe Test Cards</h2>
				<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
					<div>
						<h3 class="font-medium text-sm text-gray-700 mb-2">Successful Payment</h3>
						<code class="block p-2 bg-gray-100 rounded text-sm">4242 4242 4242 4242</code>
					</div>
					<div>
						<h3 class="font-medium text-sm text-gray-700 mb-2">Card Declined</h3>
						<code class="block p-2 bg-gray-100 rounded text-sm">4000 0000 0000 0002</code>
					</div>
					<div>
						<h3 class="font-medium text-sm text-gray-700 mb-2">Requires Authentication</h3>
						<code class="block p-2 bg-gray-100 rounded text-sm">4000 0025 0000 3155</code>
					</div>
					<div>
						<h3 class="font-medium text-sm text-gray-700 mb-2">Insufficient Funds</h3>
						<code class="block p-2 bg-gray-100 rounded text-sm">4000 0000 0000 9995</code>
					</div>
				</div>
				<p class="mt-4 text-xs text-gray-500">
					Use any future date for expiry, any 3 digits for CVC, and any 5 digits for ZIP.
				</p>
			</div>

			<!-- API Endpoints Reference -->
			<div class="bg-white rounded-lg shadow p-6">
				<h2 class="text-lg font-semibold mb-4">API Endpoints</h2>
				<div class="space-y-3">
					<div class="border-l-4 border-gray-300 pl-4">
						<code class="text-sm font-mono">POST /ext/products/purchase</code>
						<p class="text-xs text-gray-600 mt-1">Create checkout session</p>
					</div>
					<div class="border-l-4 border-gray-300 pl-4">
						<code class="text-sm font-mono">GET /ext/products/purchases</code>
						<p class="text-xs text-gray-600 mt-1">List user purchases</p>
					</div>
					<div class="border-l-4 border-gray-300 pl-4">
						<code class="text-sm font-mono">POST /ext/products/webhooks/stripe</code>
						<p class="text-xs text-gray-600 mt-1">Stripe webhook handler</p>
					</div>
					<div class="border-l-4 border-gray-300 pl-4">
						<code class="text-sm font-mono">POST /admin/ext/products/purchases/&#123;id&#125;/refund</code>
						<p class="text-xs text-gray-600 mt-1">Process refund (admin only)</p>
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>

<!-- Refund Confirmation Dialog -->
<ConfirmDialog
	bind:show={showRefundConfirm}
	title="Refund Purchase"
	message="Are you sure you want to refund this purchase?"
	confirmText="Refund"
	variant="danger"
	on:confirm={confirmRefund}
	on:close={() => { showRefundConfirm = false; purchaseToRefund = null; refundReason = ''; }}
>
	<div class="mt-4">
		<label for="refund-reason" class="block text-sm font-medium text-gray-700 mb-1">Refund Reason</label>
		<input
			id="refund-reason"
			type="text"
			bind:value={refundReason}
			placeholder="Enter refund reason..."
			class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-red-500"
		/>
	</div>
</ConfirmDialog>

<!-- Approve Confirmation Dialog -->
<ConfirmDialog
	bind:show={showApproveConfirm}
	title="Approve Purchase"
	message="Are you sure you want to approve this purchase?"
	confirmText="Approve"
	variant="success"
	on:confirm={confirmApprove}
/>

<style>
	pre {
		font-family: 'Courier New', Courier, monospace;
		white-space: pre-wrap;
		word-wrap: break-word;
	}

	code {
		font-family: 'Courier New', Courier, monospace;
	}
</style>