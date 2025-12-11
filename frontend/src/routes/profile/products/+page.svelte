<script lang="ts">
	import '../../../app.css';
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import {
		TrendingUp, Package, ShoppingCart, Users,
		Eye, Heart, DollarSign, Search, Filter,
		Download, Plus, MoreVertical, ChevronDown,
		Activity, Box, CreditCard, Clock, Check,
		X, AlertCircle, ArrowLeft, Edit2, Trash2
	} from 'lucide-svelte';
	import { api, ErrorHandler } from '$lib/api';
	import { authStore } from '$lib/stores/auth';
	import { productPreviewRegistry } from '$lib/stores/productPreviewRegistry';
	import ProductModal from '$lib/components/products/ProductModal.svelte';
	import GroupModal from '$lib/components/products/GroupModal.svelte';

	let loading = true;

	// Get tab from URL
	function getTabFromURL() {
		console.log('window', window)
		if (typeof window !== 'undefined') {
			const params = new URLSearchParams(window.location.search);
			const tab = params.get('tab');
			console.log('[Products Page] Getting tab from URL:', tab);
			return tab || 'dashboard';
		}
		return 'dashboard';
	}

	// Initialize currentTab from URL IMMEDIATELY
	let currentTab = getTabFromURL();
	console.log('[Products Page] Initial currentTab value:', currentTab);

	// Navigate to tab
	function navigateToTab(tab: string) {
		console.log('[Products Page] Navigating to tab:', tab);
		currentTab = tab;
		if (typeof window !== 'undefined') {
			window.history.replaceState({}, '', `?tab=${tab}`);
		}
	}

	// Listen for browser back/forward
	onMount(() => {
		const handlePopState = () => {
			currentTab = getTabFromURL();
			console.log('[Products Page] PopState - tab is now:', currentTab);
		};

		window.addEventListener('popstate', handlePopState);

		return () => {
			window.removeEventListener('popstate', handlePopState);
		};
	});

	// Subscribe to the preview component registry
	let previewComponents: Record<string, any> = {};
	$: previewComponents = $productPreviewRegistry;

	// Real data from API
	let products: any[] = [];
	let orders: any[] = [];
	let groups: any[] = [];
	let groupTypes: any[] = [];
	
	// Stats (calculated from real data)
	let stats = {
		totalProducts: 0,
		totalSales: 0,
		totalGroups: 0,
		totalViews: 0,
		productLikes: 0,
		totalRevenue: 0
	};
	
	// Filters
	let searchQuery = '';
	let filterStatus = 'all';
	let sortBy = 'date';
	
	// Group and Product creation
	let showCreateGroupModal = false;
	let showEditGroupModal = false;
	let showCreateProductModal = false;
	let showEditProductModal = false;
	let selectedGroupForProduct: any = null;
	let productTypes: any[] = [];
	let productTemplates: any[] = [];
	let editingProduct: any = null;
	let editingGroup: any = null;
	
	$: filteredOrders = orders.filter(order => {
		const matchesSearch =
			(order.id && order.id.toLowerCase().includes(searchQuery.toLowerCase())) ||
			(order.customerEmail && order.customerEmail.toLowerCase().includes(searchQuery.toLowerCase())) ||
			(order.productName && order.productName.toLowerCase().includes(searchQuery.toLowerCase()));
		const matchesStatus = filterStatus === 'all' || order.status === filterStatus;
		return matchesSearch && matchesStatus;
	});
	
	$: orderStats = {
		completed: orders.filter(o => o.status === 'completed').length,
		pending: orders.filter(o => o.status === 'pending').length,
		cancelled: orders.filter(o => o.status === 'cancelled').length,
		totalRevenue: orders
			.filter(o => o.status === 'completed')
			.reduce((sum, o) => sum + (o.amount || 0), 0)
	};
	
	// Get top products by calculating from orders
	$: topProducts = products.slice(0, 3).map(product => ({
		...product,
		sales: orders.filter(o => o.productId === product.id && o.status === 'completed').length,
		views: 0, // Will be implemented when view tracking is added
		likes: 0 // Will be implemented when likes feature is added
	}));
	
	// Recent sales from orders
	$: recentSales = orders
		.filter(o => o.status === 'completed')
		.slice(0, 3)
		.map(order => ({
			id: order.id,
			name: order.productName || 'Unknown Product',
			date: new Date(order.createdAt).toLocaleDateString(),
			price: order.amount || 0
		}));
	
	onMount(async () => {
		// Get tab from URL first - CRITICAL FOR INITIAL LOAD
		const tabFromURL = getTabFromURL();
		console.log('[Products Page] onMount - setting tab to:', tabFromURL);
		currentTab = tabFromURL;

		if (!$authStore.user) {
			goto('/login');
			return;
		}

		// Check for URL parameters for navigation
		const urlParams = new URLSearchParams(window.location.search);
		const mode = urlParams.get('mode');
		const template = urlParams.get('template');
		const productId = urlParams.get('id');
		const data = urlParams.get('data');

		// Generic preview URL discovery
		if (typeof window !== 'undefined') {
			// Applications can register preview URLs using the pattern: solobase_preview_[template_id]
			// This allows any application to register preview URLs without Solobase knowing about them
			console.log('Scanning localStorage for preview URLs...');
			for (let i = 0; i < localStorage.length; i++) {
				const key = localStorage.key(i);
				if (key && key.startsWith('solobase_preview_')) {
					const templateId = key.replace('solobase_preview_', '');
					const previewUrl = localStorage.getItem(key);
					console.log('Found preview registration:', { key, templateId, previewUrl });
					if (previewUrl) {
						productPreviewRegistry.register(templateId, previewUrl);
						console.log('Registered preview URL for template:', templateId);
					}
				}
			}
		}

		await loadData();

		// Handle navigation based on URL parameters
		if (mode === 'create') {
			// Pre-select the template if specified
			if (template) {
				editingProduct = {
					productTemplateId: template
				};
				// If there's data (for duplicate), parse and apply it
				if (data) {
					try {
						const parsedData = JSON.parse(decodeURIComponent(data));
						editingProduct = {
							...parsedData,
							productTemplateId: template
						};
					} catch (e) {
						console.error('Failed to parse product data:', e);
					}
				}
			}
			showCreateProductModal = true;
		} else if (mode === 'edit' && productId) {
			// Find the product by ID
			const product = products.find(p => p.id === parseInt(productId));
			if (product) {
				editingProduct = product;
				showEditProductModal = true;
			}
		}

		// Clear navigation params from URL after processing (but keep tab)
		if (mode || template || productId || data) {
			const tab = urlParams.get('tab') || 'dashboard';
			goto(`?tab=${tab}`, { replaceState: true, noScroll: true });
		}
	});
	
	async function loadData() {
		try {
			loading = true;
			
			// Load group types
			try {
				const typesRes = await api.get('/ext/products/group-types');
				groupTypes = Array.isArray(typesRes) ? typesRes : [];
			} catch (err) {
				console.error('Failed to load group types:', err);
				groupTypes = [];
			}
			
			// Load product types
			try {
				const prodTypesRes = await api.get('/ext/products/product-types');
				productTypes = Array.isArray(prodTypesRes) ? prodTypesRes : [];
			} catch (err) {
				console.error('Failed to load product types:', err);
				productTypes = [];
			}

			// Load product templates (product types are actually ProductTemplate models)
			// No mapping needed - they already have the correct structure
			productTemplates = productTypes;
			
			// Load groups
			try {
				const groupsRes = await api.get('/ext/products/groups');
				groups = Array.isArray(groupsRes) ? groupsRes : [];
			} catch (err) {
				console.error('Failed to load groups:', err);
				groups = [];
			}
			
			// Load products
			try {
				const productsRes = await api.get('/ext/products/products');
				// Filter out any null/undefined products and ensure all have IDs
				products = Array.isArray(productsRes)
					? productsRes.filter(p => p && p.id)
					: [];
			} catch (err) {
				console.error('Failed to load products:', err);
				products = [];
			}
			
			// Orders are not available yet, using empty array
			orders = [];
			
			// Calculate stats
			stats = {
				totalProducts: products.length,
				totalSales: orders.filter(o => o.status === 'completed').length,
				totalGroups: groups.length,
				totalViews: products.reduce((sum, p) => sum + (p.views || 0), 0),
				productLikes: products.reduce((sum, p) => sum + (p.likes || 0), 0),
				totalRevenue: orders
					.filter(o => o.status === 'completed')
					.reduce((sum, o) => sum + (o.amount || 0), 0)
			};
			
		} catch (error) {
			console.error('Failed to load data:', error);
		} finally {
			loading = false;
		}
	}
	
	interface GroupResponse {
		id?: string;
		error?: string;
	}

	async function handleGroupSubmit(event: CustomEvent) {
		const groupData = event.detail;
		try {
			if (editingGroup && editingGroup.id) {
				// Update existing group
				const response = await api.put<GroupResponse>(`/ext/products/groups/${editingGroup.id}`, groupData);

				// Check if response is an error
				if (response?.error) {
					throw new Error(response.error);
				}

				if (response && response.id) {
					groups = groups.map(g => g.id === editingGroup.id ? response : g);
				} else {
					throw new Error('Invalid response from server');
				}
			} else {
				// Create new group
				const response = await api.post<GroupResponse>('/ext/products/groups', groupData);

				// Check if response is an error
				if (response?.error) {
					throw new Error(response.error);
				}

				if (response && response.id) {
					groups = [...groups, response];
				} else {
					throw new Error('Invalid response from server');
				}
			}
			await loadData();
			showCreateGroupModal = false;
			showEditGroupModal = false;
			editingGroup = null;
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}

	async function editGroup(group: any) {
		editingGroup = group;
		showEditGroupModal = true;
	}
	
	interface ProductResponse {
		id?: string;
		error?: string;
	}

	async function handleProductSubmit(event: CustomEvent) {
		const productData = event.detail;
		try {
			// Ensure proper data types
			if (productData.groupId) productData.groupId = parseInt(productData.groupId);
			if (productData.productTemplateId) productData.productTemplateId = parseInt(productData.productTemplateId);
			if (productData.basePrice) productData.basePrice = parseFloat(productData.basePrice);

			let success = false;

			if (editingProduct && editingProduct.id) {
				// Update existing product
				const productId = editingProduct.id; // Store ID before any async operations
				const response = await api.put<ProductResponse>(`/ext/products/products/${productId}`, productData);

				// Check if response is an error
				if (response?.error) {
					throw new Error(response.error);
				}

				if (response && response.id) {
					// Use stored productId instead of editingProduct.id in map
					products = products.filter(p => p && p.id).map(p => {
						return p.id === productId ? response : p;
					});
					success = true;
				} else {
					throw new Error('Invalid response from server - missing product ID');
				}
			} else {
				// Create new product
				if (selectedGroupForProduct) {
					productData.groupId = parseInt(selectedGroupForProduct.id);
				}
				const response = await api.post<ProductResponse>('/ext/products/products', productData);

				// Check if response is an error
				if (response?.error) {
					throw new Error(response.error);
				}

				if (response && response.id) {
					products = [...products, response];
					success = true;
				} else {
					throw new Error('Invalid response from server');
				}
			}

			if (success) {
				await loadData();
				showCreateProductModal = false;
				showEditProductModal = false;
				selectedGroupForProduct = null;
				editingProduct = null;
			}
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}

	async function editProduct(product: any) {
		editingProduct = product;
		showEditProductModal = true;
	}
	
	async function deleteProduct(id: string) {
		if (!confirm('Are you sure you want to delete this product?')) {
			return;
		}
		
		try {
			await api.delete(`/ext/products/products/${id}`);
			products = products.filter(p => p.id !== id);
			await loadData();
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}
	
	async function deleteGroup(id: string) {
		if (!confirm('Are you sure you want to delete this group? All associated products will also be deleted.')) {
			return;
		}

		try {
			await api.delete(`/ext/products/groups/${id}`);
			groups = groups.filter(e => e.id !== id);
			await loadData();
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}
	
	function openProductCreationForGroup(group: any) {
		selectedGroupForProduct = group;
		editingProduct = null;
		showCreateProductModal = true;
	}
	
	function getGroupTypeName(typeId: string): string {
		const type = groupTypes.find(t => t.id === typeId);
		return type?.name || 'Unknown';
	}
	
	// Get products for a specific group
	function getGroupProducts(groupId: string) {
		return products.filter(p => p.groupId === groupId);
	}
	
	function getStatusColor(status: string) {
		switch(status) {
			case 'completed': return 'status-completed';
			case 'pending': return 'status-pending';
			case 'cancelled': return 'status-cancelled';
			case 'refunded': return 'status-refunded';
			default: return '';
		}
	}
	
	function getStatusIcon(status: string) {
		switch(status) {
			case 'completed': return Check;
			case 'pending': return Clock;
			case 'cancelled': return X;
			case 'refunded': return AlertCircle;
			default: return null;
		}
	}
	
	function exportOrders() {
		const csv = [
			['Order ID', 'Date', 'Customer', 'Product', 'Amount', 'Status'],
			...filteredOrders.map(o => [
				o.id,
				new Date(o.createdAt).toLocaleDateString(),
				o.customerEmail,
				o.productName,
				o.amount,
				o.status
			])
		].map(row => row.join(',')).join('\n');
		
		const blob = new Blob([csv], { type: 'text/csv' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `orders-${new Date().toISOString().split('T')[0]}.csv`;
		a.click();
	}
</script>

<svelte:head>
	<title>Products Dashboard - Solobase</title>
</svelte:head>

<div class="products-page">
	<div class="products-container">
		<!-- Back Button -->
		<a href="/profile" class="back-button">
			<ArrowLeft size={18} />
			<span>Back to Profile</span>
		</a>
		
		<div class="products-card">
			<!-- Header with Logo -->
			<div class="logo-header">
				<img src="/logo_long.png" alt="Solobase" class="logo" />
			</div>
			
			<!-- Navigation Tabs -->
			<div class="nav-tabs">
				<button
					class="nav-tab {currentTab === 'dashboard' ? 'active' : ''}"
					on:click={() => navigateToTab('dashboard')}
				>
					<TrendingUp size={14} />
					Dashboard
				</button>
				<button
					class="nav-tab {currentTab === 'orders' ? 'active' : ''}"
					on:click={() => navigateToTab('orders')}
				>
					<ShoppingCart size={14} />
					Orders
				</button>
				<button
					class="nav-tab {currentTab === 'products' ? 'active' : ''}"
					on:click={() => navigateToTab('products')}
				>
					<Package size={14} />
					Products
				</button>
			</div>
			
			{#if loading}
				<div class="loading-container">
					<div class="spinner"></div>
					<p>Loading dashboard...</p>
				</div>
			{:else if currentTab === 'dashboard'}
				<!-- Dashboard Tab -->
				<div class="dashboard-content">
					<!-- Compact Stats Grid -->
					<div class="stats-grid">
						<div class="stat-card">
							<div class="stat-icon purple"><Package size={16} /></div>
							<div class="stat-info">
								<span class="stat-value">{stats.totalProducts}</span>
								<span class="stat-label">Products</span>
							</div>
						</div>
						
						<div class="stat-card">
							<div class="stat-icon cyan"><ShoppingCart size={16} /></div>
							<div class="stat-info">
								<span class="stat-value">{stats.totalSales}</span>
								<span class="stat-label">Sales</span>
							</div>
						</div>
						
						<div class="stat-card">
							<div class="stat-icon green"><DollarSign size={16} /></div>
							<div class="stat-info">
								<span class="stat-value">${stats.totalRevenue.toFixed(0)}</span>
								<span class="stat-label">Revenue</span>
							</div>
						</div>
						
						<div class="stat-card">
							<div class="stat-icon blue"><Users size={16} /></div>
							<div class="stat-info">
								<span class="stat-value">{stats.totalGroups}</span>
								<span class="stat-label">Groups</span>
							</div>
						</div>
					</div>
					
					<!-- Recent Activity -->
					<div class="activity-section">
						<!-- Recent Sales -->
						<div class="activity-card">
							<h3 class="section-title">
								<DollarSign size={14} />
								Recent Sales
							</h3>
							{#if recentSales.length > 0}
								<div class="sales-list">
									{#each recentSales as sale}
										<div class="sale-item">
											<div class="sale-info">
												<span class="sale-name">{sale.name}</span>
												<span class="sale-date">{sale.date}</span>
											</div>
											<span class="sale-price">${sale.price.toFixed(2)}</span>
										</div>
									{/each}
								</div>
							{:else}
								<p class="no-data">No recent sales</p>
							{/if}
						</div>
						
						<!-- Top Products -->
						<div class="activity-card">
							<h3 class="section-title">
								<Activity size={14} />
								Top Products
							</h3>
							{#if topProducts.length > 0}
								<div class="products-list">
									{#each topProducts as product, index}
										<div class="product-item">
											<span class="product-rank">{index + 1}</span>
											<div class="product-info">
												<span class="product-name">{product.name}</span>
												<div class="product-stats">
													<span><Eye size={10} /> {product.views}</span>
													<span><Heart size={10} /> {product.likes}</span>
													<span><ShoppingCart size={10} /> {product.sales}</span>
												</div>
											</div>
										</div>
									{/each}
								</div>
							{:else}
								<p class="no-data">No products yet</p>
							{/if}
						</div>
					</div>
				</div>
			{:else if currentTab === 'orders'}
				<!-- Orders Tab -->
				<div class="orders-content">
					<!-- Search Bar -->
					<div class="orders-header">
						<div class="search-box">
							<Search size={14} />
							<input 
								type="text" 
								placeholder="Search orders..."
								bind:value={searchQuery}
							/>
						</div>
						<button class="btn-export" on:click={exportOrders}>
							<Download size={14} />
							Export
						</button>
					</div>
					
					<!-- Orders Table -->
					{#if filteredOrders.length > 0}
						<div class="orders-table-container">
							<table class="orders-table">
								<thead>
									<tr>
										<th>Order</th>
										<th>Customer</th>
										<th>Product</th>
										<th>Amount</th>
										<th>Status</th>
									</tr>
								</thead>
								<tbody>
									{#each filteredOrders as order}
										<tr>
											<td>
												<div class="order-info">
													<span class="order-id">#{order.id?.slice(0, 8)}</span>
													<span class="order-date">{new Date(order.createdAt).toLocaleDateString()}</span>
												</div>
											</td>
											<td class="customer">{order.customerEmail || 'Unknown'}</td>
											<td>{order.productName || 'Unknown'}</td>
											<td class="amount">${(order.amount || 0).toFixed(2)}</td>
											<td>
												<span class="status-badge {getStatusColor(order.status)}">
													{order.status || 'pending'}
												</span>
											</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
						
						<!-- Order Stats -->
						<div class="order-stats">
							<span class="stat">Total: {orders.length}</span>
							<span class="stat completed">Completed: {orderStats.completed}</span>
							<span class="stat pending">Pending: {orderStats.pending}</span>
							<span class="stat revenue">Revenue: ${orderStats.totalRevenue.toFixed(2)}</span>
						</div>
					{:else}
						<div class="empty-state">
							<ShoppingCart size={32} />
							<p>No orders found</p>
						</div>
					{/if}
				</div>
			{:else if currentTab === 'products'}
				<!-- Products Tab -->
				<div class="products-content">
					<div class="products-header">
						<h3 class="section-title">Manage Products</h3>
						{#if groups.length === 0}
							<button class="btn-primary" on:click={() => showCreateGroupModal = true}>
								<Plus size={14} />
								Create Group
							</button>
						{/if}
					</div>
					
					{#if groups.length > 0}
						<!-- Groups with their products -->
						<div class="groups-section">
							{#each groups as group}
								{@const groupProducts = getGroupProducts(group.id)}
								<div class="group-container">
									<div class="group-header">
										<div class="group-info">
											<h4>{group.name}</h4>
											{#if group.groupTemplateId && getGroupTypeName(group.groupTemplateId) !== 'Unknown'}
												<span class="group-type">{getGroupTypeName(group.groupTemplateId)}</span>
											{/if}
										</div>
										<div class="group-actions">
											<button class="btn-sm btn-primary" on:click={() => openProductCreationForGroup(group)}>
												<Plus size={12} />
												Add Product
											</button>
											<button class="btn-icon" on:click={() => editGroup(group)} title="Edit Group">
												<Edit2 size={14} />
											</button>
											<button class="btn-icon" on:click={() => deleteGroup(group.id)} title="Delete Group">
												<Trash2 size={14} />
											</button>
										</div>
									</div>
									
									{#if group.description}
										<p class="group-description">{group.description}</p>
									{/if}
									
									{#if groupProducts.length > 0}
										<div class="group-products">
											{#each groupProducts as product}
												<div class="product-card">
													<div class="product-card-header">
														<h5>{product.name}</h5>
														<div class="product-actions">
															<button class="btn-icon btn-icon-edit" on:click={() => editProduct(product)} title="Edit Product">
																<Edit2 size={12} />
															</button>
															<button class="btn-icon" on:click={() => deleteProduct(product.id)} title="Delete Product">
																<Trash2 size={12} />
															</button>
														</div>
													</div>
													<p class="product-description">{product.description || 'No description'}</p>
													<div class="product-price">${(product.basePrice || product.price || 0).toFixed(2)}</div>
												</div>
											{/each}
										</div>
									{:else}
										<div class="no-products">
											<Package size={20} />
											<p>No products for this group</p>
											<button class="btn-sm" on:click={() => openProductCreationForGroup(group)}>
												Add First Product
											</button>
										</div>
									{/if}
								</div>
							{/each}
							
							<!-- Add New Group Card -->
							<div class="group-container add-group" on:click={() => showCreateGroupModal = true}>
								<Plus size={24} />
								<span>Add New Group</span>
							</div>
						</div>
					{:else}
						<!-- No groups yet -->
						<div class="empty-state">
							<Box size={32} />
							<h4>Start by Creating an Group</h4>
							<p>Groups represent your business units (stores, restaurants, services)</p>
							<button class="btn-primary" on:click={() => showCreateGroupModal = true}>
								<Plus size={14} />
								Create Your First Group
							</button>
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</div>

<!-- Group Modals -->
<GroupModal
	show={showCreateGroupModal}
	mode="create"
	on:submit={handleGroupSubmit}
	on:close={() => showCreateGroupModal = false}
/>

<GroupModal
	show={showEditGroupModal}
	mode="edit"
	group={editingGroup}
	title="Edit Group"
	submitButtonText="Save Changes"
	on:submit={handleGroupSubmit}
	on:close={() => showEditGroupModal = false}
/>

<!-- Product Modals -->
<ProductModal
	show={showCreateProductModal}
	mode="create"
	product={editingProduct}
	{productTemplates}
	{groups}
	initialGroupId={selectedGroupForProduct?.id}
	title={selectedGroupForProduct ? `Add Product to ${selectedGroupForProduct.name}` : 'Create New Product'}
	on:submit={handleProductSubmit}
	on:close={() => {
		showCreateProductModal = false;
		selectedGroupForProduct = null;
		editingProduct = null;
	}}
/>

<ProductModal
	show={showEditProductModal}
	mode="edit"
	product={editingProduct}
	{productTemplates}
	{groups}
	title={'Edit Product'}
	on:submit={handleProductSubmit}
	on:close={() => {
		showEditProductModal = false;
		editingProduct = null;
	}}
/>

<style>
	/* Page Layout - matching profile page */
	.products-page {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #f0f0f0;
		padding: 1rem;
	}
	
	.products-container {
		width: 100%;
		max-width: 900px;
		position: relative;
		padding-top: 3.5rem; /* Add padding to make room for back button */
	}
	
	.products-card {
		background: white;
		border: 1px solid #e2e8f0;
		border-radius: 12px;
		padding: 1.5rem;
		animation: slideUp 0.4s ease-out;
		min-height: 500px;
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
	
	/* Back Button */
	.back-button {
		position: absolute;
		top: 0;
		left: 0;
		display: flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.375rem 0.625rem;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		color: #374151;
		font-size: 0.813rem;
		font-weight: 500;
		text-decoration: none;
		transition: all 0.2s;
	}
	
	.back-button:hover {
		background: #f9fafb;
		border-color: #189AB4;
		transform: translateX(-2px);
	}
	
	/* Logo Header */
	.logo-header {
		text-align: center;
		margin-bottom: 1.5rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid #e5e7eb;
	}
	
	.logo {
		height: 35px;
		width: auto;
		max-width: 200px;
	}
	
	/* Navigation Tabs */
	.nav-tabs {
		display: flex;
		gap: 0.25rem;
		margin-bottom: 1.5rem;
		border-bottom: 1px solid #e5e7eb;
		padding-bottom: 0;
	}
	
	.nav-tab {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.625rem 1rem;
		background: transparent;
		border: none;
		border-bottom: 2px solid transparent;
		color: #6b7280;
		font-size: 0.813rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		margin-bottom: -1px;
	}
	
	.nav-tab:hover {
		color: #374151;
	}
	
	.nav-tab.active {
		color: #189AB4;
		border-bottom-color: #189AB4;
	}
	
	/* Loading */
	.loading-container {
		padding: 3rem;
		text-align: center;
	}
	
	.spinner {
		width: 32px;
		height: 32px;
		border: 3px solid #e5e7eb;
		border-top-color: #189AB4;
		border-radius: 50%;
		margin: 0 auto 1rem;
		animation: spin 1s linear infinite;
	}
	
	@keyframes spin {
		to { transform: rotate(360deg); }
	}
	
	/* Dashboard Content */
	.dashboard-content {
		animation: fadeIn 0.3s ease-out;
	}
	
	@keyframes fadeIn {
		from { opacity: 0; }
		to { opacity: 1; }
	}
	
	/* Compact Stats Grid */
	.stats-grid {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 0.75rem;
		margin-bottom: 1.5rem;
	}
	
	.stat-card {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.875rem;
		background: #f9fafb;
		border-radius: 0.5rem;
		border: 1px solid #e5e7eb;
	}
	
	.stat-icon {
		width: 32px;
		height: 32px;
		border-radius: 0.375rem;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	
	.stat-icon.purple {
		background: #f3e8ff;
		color: #9333ea;
	}
	
	.stat-icon.cyan {
		background: #e0f2fe;
		color: #06b6d4;
	}
	
	.stat-icon.green {
		background: #d1fae5;
		color: #10b981;
	}
	
	.stat-icon.blue {
		background: #dbeafe;
		color: #3b82f6;
	}
	
	.stat-info {
		display: flex;
		flex-direction: column;
	}
	
	.stat-value {
		font-size: 1.125rem;
		font-weight: 700;
		color: #111827;
		line-height: 1;
	}
	
	.stat-label {
		font-size: 0.75rem;
		color: #6b7280;
		margin-top: 0.125rem;
	}
	
	/* Activity Section */
	.activity-section {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
	}
	
	.activity-card {
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		padding: 1rem;
	}
	
	.section-title {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		margin: 0 0 0.75rem 0;
		font-size: 0.875rem;
		font-weight: 600;
		color: #374151;
	}
	
	/* Sales List */
	.sales-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	
	.sale-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.5rem;
		background: white;
		border-radius: 0.375rem;
		font-size: 0.813rem;
	}
	
	.sale-info {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}
	
	.sale-name {
		font-weight: 500;
		color: #111827;
	}
	
	.sale-date {
		font-size: 0.75rem;
		color: #6b7280;
	}
	
	.sale-price {
		font-weight: 600;
		color: #10b981;
	}
	
	/* Products List */
	.products-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	
	.product-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem;
		background: white;
		border-radius: 0.375rem;
		font-size: 0.813rem;
	}
	
	.product-rank {
		width: 24px;
		height: 24px;
		background: #189AB4;
		color: white;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 600;
		font-size: 0.75rem;
	}
	
	.product-info {
		flex: 1;
	}
	
	.product-name {
		font-weight: 500;
		color: #111827;
		display: block;
	}
	
	.product-stats {
		display: flex;
		gap: 0.625rem;
		margin-top: 0.125rem;
		font-size: 0.688rem;
		color: #6b7280;
	}
	
	.product-stats span {
		display: flex;
		align-items: center;
		gap: 0.125rem;
	}
	
	/* Orders Content */
	.orders-content {
		animation: fadeIn 0.3s ease-out;
	}
	
	.orders-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
		gap: 0.75rem;
	}
	
	.search-box {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.5rem 0.75rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		background: white;
		flex: 1;
		max-width: 300px;
	}
	
	.search-box input {
		border: none;
		outline: none;
		flex: 1;
		font-size: 0.813rem;
	}
	
	.btn-export {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.5rem 0.75rem;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		font-size: 0.813rem;
		font-weight: 500;
		color: #374151;
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.btn-export:hover {
		background: #f9fafb;
	}
	
	/* Orders Table */
	.orders-table-container {
		overflow-x: auto;
		margin-bottom: 1rem;
	}
	
	.orders-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.813rem;
	}
	
	.orders-table thead {
		background: #f9fafb;
	}
	
	.orders-table th {
		padding: 0.625rem;
		text-align: left;
		font-weight: 600;
		color: #374151;
		border-bottom: 1px solid #e5e7eb;
	}
	
	.orders-table td {
		padding: 0.625rem;
		border-bottom: 1px solid #f3f4f6;
		color: #374151;
	}
	
	.order-info {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}
	
	.order-id {
		font-weight: 500;
		color: #111827;
	}
	
	.order-date {
		font-size: 0.688rem;
		color: #6b7280;
	}
	
	.customer {
		color: #374151;
	}
	
	.amount {
		font-weight: 600;
		color: #111827;
	}
	
	.status-badge {
		display: inline-block;
		padding: 0.25rem 0.5rem;
		border-radius: 999px;
		font-size: 0.688rem;
		font-weight: 500;
		text-transform: capitalize;
	}
	
	.status-completed {
		background: #d1fae5;
		color: #065f46;
	}
	
	.status-pending {
		background: #fed7aa;
		color: #92400e;
	}
	
	.status-cancelled {
		background: #fee2e2;
		color: #991b1b;
	}
	
	.status-refunded {
		background: #cffafe;
		color: #155e75;
	}
	
	/* Order Stats */
	.order-stats {
		display: flex;
		gap: 1rem;
		padding: 0.75rem;
		background: #f9fafb;
		border-radius: 0.375rem;
		font-size: 0.813rem;
	}
	
	.order-stats .stat {
		font-weight: 500;
	}
	
	.stat.completed {
		color: #10b981;
	}
	
	.stat.pending {
		color: #f59e0b;
	}
	
	.stat.revenue {
		color: #189AB4;
		margin-left: auto;
	}
	
	/* Products Content */
	.products-content {
		animation: fadeIn 0.3s ease-out;
	}
	
	.products-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}
	
	/* Groups Section */
	.groups-section {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	
	.group-container {
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		padding: 1rem;
	}
	
	.group-container.add-group {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 2rem;
		border: 2px dashed #d1d5db;
		cursor: pointer;
		transition: all 0.2s;
		gap: 0.5rem;
		color: #6b7280;
	}
	
	.group-container.add-group:hover {
		border-color: #189AB4;
		background: #f0f9ff;
		color: #189AB4;
	}
	
	.group-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 0.75rem;
	}
	
	.group-info h4 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
		color: #111827;
	}
	
	.group-type {
		font-size: 0.75rem;
		color: #189AB4;
		font-weight: 500;
	}
	
	.group-description {
		font-size: 0.813rem;
		color: #6b7280;
		margin: 0 0 0.75rem 0;
		line-height: 1.4;
	}
	
	.group-actions {
		display: flex;
		gap: 0.5rem;
		align-items: center;
	}
	
	.group-products {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
		gap: 0.75rem;
		margin-top: 0.75rem;
	}
	
	.no-products {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 1.5rem;
		background: white;
		border-radius: 0.375rem;
		border: 1px dashed #e5e7eb;
		color: #9ca3af;
		text-align: center;
		gap: 0.5rem;
	}
	
	.no-products p {
		margin: 0;
		font-size: 0.813rem;
	}
	
	.product-card {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		padding: 0.75rem;
	}
	
	.product-card-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 0.375rem;
	}
	
	.product-card h5 {
		margin: 0;
		font-size: 0.813rem;
		font-weight: 600;
		color: #111827;
	}
	
	.product-actions {
		display: flex;
		gap: 0.25rem;
	}
	
	.btn-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 24px;
		height: 24px;
		border: 1px solid #e5e7eb;
		border-radius: 0.25rem;
		background: white;
		color: #6b7280;
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.btn-icon:hover {
		background: #fee2e2;
		border-color: #ef4444;
		color: #ef4444;
	}

	.btn-icon-edit:hover {
		background: #e0f2fe;
		border-color: #189AB4;
		color: #189AB4;
	}
	
	.product-description {
		font-size: 0.75rem;
		color: #6b7280;
		margin: 0 0 0.5rem 0;
		line-height: 1.4;
	}
	
	.product-price {
		font-size: 0.875rem;
		font-weight: 600;
		color: #189AB4;
	}
	
	/* Empty State */
	.empty-state {
		padding: 3rem 2rem;
		text-align: center;
		color: #6b7280;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		min-height: 200px;
	}
	
	.empty-state p {
		margin: 0.5rem 0 1rem;
		font-size: 0.875rem;
	}
	
	.no-data {
		text-align: center;
		color: #9ca3af;
		font-size: 0.813rem;
		padding: 1rem;
		margin: 0;
	}
	
	/* Buttons */
	.btn {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.5rem 0.875rem;
		border-radius: 0.375rem;
		font-size: 0.813rem;
		font-weight: 500;
		border: none;
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.btn-primary {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.5rem 0.875rem;
		background: #189AB4;
		color: white;
		border: none;
		border-radius: 0.375rem;
		font-size: 0.813rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.btn-primary:hover {
		background: #05445E;
	}
	
	.btn-secondary {
		background: white;
		color: #374151;
		border: 1px solid #e5e7eb;
	}
	
	.btn-secondary:hover {
		background: #f9fafb;
	}
	
	.btn-sm {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.375rem 0.625rem;
		background: white;
		color: #374151;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		font-size: 0.75rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.btn-sm:hover {
		background: #f9fafb;
	}
	
	.btn-sm.btn-primary {
		background: #189AB4;
		color: white;
		border-color: #189AB4;
	}
	
	.btn-sm.btn-primary:hover {
		background: #05445E;
		border-color: #05445E;
	}
	
	/* Modal */
	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 9999;
	}
	
	.modal {
		background: white;
		border-radius: 0.5rem;
		width: 90%;
		max-width: 400px;
		max-height: 90vh;
		overflow-y: auto;
	}
	
	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		border-bottom: 1px solid #e5e7eb;
	}
	
	.modal-header h3 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
		color: #111827;
	}
	
	.btn-close {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border: none;
		background: transparent;
		color: #6b7280;
		cursor: pointer;
		border-radius: 0.25rem;
		transition: all 0.2s;
	}
	
	.btn-close:hover {
		background: #f3f4f6;
		color: #374151;
	}
	
	.modal-body {
		padding: 1rem;
	}
	
	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
		padding: 1rem;
		border-top: 1px solid #e5e7eb;
	}
	
	.form-group {
		margin-bottom: 0.875rem;
	}
	
	.form-group label {
		display: block;
		margin-bottom: 0.375rem;
		font-size: 0.813rem;
		font-weight: 500;
		color: #374151;
	}
	
	.form-group input,
	.form-group select,
	.form-group textarea {
		width: 100%;
		padding: 0.5rem 0.625rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		font-size: 0.813rem;
	}
	
	.form-group input:focus,
	.form-group select:focus,
	.form-group textarea:focus {
		outline: none;
		border-color: #189AB4;
		box-shadow: 0 0 0 2px rgba(24, 154, 180, 0.1);
	}
	
	/* Custom Fields Styles */
	.custom-fields-section {
		margin-top: 1rem;
		padding-top: 1rem;
		border-top: 1px solid #e5e7eb;
	}
	
	.custom-fields-title {
		margin: 0 0 0.75rem 0;
		font-size: 0.875rem;
		font-weight: 600;
		color: #374151;
	}
	
	.field-help {
		display: block;
		margin-top: 0.25rem;
		margin-bottom: 0.375rem;
		font-size: 0.75rem;
		color: #6b7280;
	}
	
	.required {
		color: #ef4444;
		margin-left: 0.125rem;
	}
	
	/* Responsive */
	@media (max-width: 768px) {
		.products-container {
			max-width: 100%;
		}
		
		.stats-grid {
			grid-template-columns: repeat(2, 1fr);
		}
		
		.activity-section {
			grid-template-columns: 1fr;
		}
		
		.nav-tabs {
			overflow-x: auto;
		}
		
		.products-grid {
			grid-template-columns: 1fr;
		}
	}
</style>