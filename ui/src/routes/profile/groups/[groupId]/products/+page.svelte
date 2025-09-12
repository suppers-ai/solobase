<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { 
		Package, Plus, Edit2, Trash2, Search, Filter,
		DollarSign, Tag, Calendar, ArrowLeft, Settings,
		ShoppingCart, Box, Calculator, Variable
	} from 'lucide-svelte';
	import { api } from '$lib/api';
	import { authStore } from '$lib/stores/auth';
	import { goto } from '$app/navigation';

	interface Product {
		id: string;
		group_id: string;
		product_type_id: string;
		name: string;
		display_name: string;
		description?: string;
		sku?: string;
		base_price: number;
		currency: string;
		variable_values?: any;
		metadata?: any;
		is_active: boolean;
		created_at: string;
		updated_at: string;
		// Calculated fields
		calculated_price?: number;
		product_type?: any;
	}

	let groupId = '';
	let group: any = null;
	let products: Product[] = [];
	let productTypes: any[] = [];
	let availableVariables: any[] = [];
	let loading = true;
	let searchQuery = '';
	let selectedType = 'all';
	let showCreateModal = false;
	let showEditModal = false;
	let showPricingModal = false;
	let selectedProduct: Product | null = null;
	
	// Check if user is admin
	$: isAdmin = $authStore.user?.role === 'admin';
	
	// Form data for new product
	let newProduct: Partial<Product> = {
		name: '',
		display_name: '',
		description: '',
		product_type_id: '',
		sku: '',
		base_price: 0,
		currency: 'USD',
		is_active: true,
		variable_values: {},
		metadata: {}
	};

	// Dynamic fields based on product type
	let dynamicFields: any = {};
	let variableValues: any = {};

	$: groupId = $page.params.groupId;
	$: filteredProducts = products.filter(product => {
		const matchesSearch = product.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
			product.display_name.toLowerCase().includes(searchQuery.toLowerCase()) ||
			product.sku?.toLowerCase().includes(searchQuery.toLowerCase()) ||
			product.description?.toLowerCase().includes(searchQuery.toLowerCase());
		const matchesType = selectedType === 'all' || product.product_type_id === selectedType;
		return matchesSearch && matchesType;
	});

	onMount(async () => {
		// Check if user is logged in
		const currentUser = $authStore.user;
		if (!currentUser) {
			goto('/login');
			return;
		}
		await loadData();
	});

	async function loadData() {
		try {
			loading = true;
			
			// Load group details
			const groupRes = await api.get(`/user/groups/${groupId}`);
			group = groupRes;
			
			// Load product types and variables
			const [typesRes, variablesRes] = await Promise.all([
				api.get('/products/product-types'),
				api.get('/products/variables')
			]);
			productTypes = typesRes || [];
			availableVariables = variablesRes || [];
			
			// Load group's products
			const productsRes = await api.get(`/user/groups/${groupId}/products`);
			products = productsRes || [];
			
			// Calculate prices for each product
			products = await Promise.all(products.map(async (product) => {
				try {
					const priceRes = await api.post('/products/calculate-price', {
						product_id: product.id,
						variables: product.variable_values
					});
					return { ...product, calculated_price: priceRes.price };
				} catch {
					return { ...product, calculated_price: product.base_price };
				}
			}));
		} catch (error) {
			console.error('Failed to load data:', error);
			products = [];
			productTypes = [];
		} finally {
			loading = false;
		}
	}

	function getProductTypeInfo(typeId: string) {
		return productTypes.find(t => t.id === typeId);
	}

	function onProductTypeChange() {
		const selectedType = productTypes.find(t => t.id === newProduct.product_type_id);
		if (selectedType) {
			// Initialize dynamic fields based on schema
			if (selectedType.fields_schema) {
				dynamicFields = {};
				Object.entries(selectedType.fields_schema).forEach(([key, field]: [string, any]) => {
					dynamicFields[key] = field.default || '';
				});
			}
			
			// Initialize variable values based on product type's default variables
			if (selectedType.default_variables) {
				variableValues = { ...selectedType.default_variables };
			}
		} else {
			dynamicFields = {};
			variableValues = {};
		}
	}

	function getRelevantVariables() {
		// Get variables relevant to the selected product type
		return availableVariables.filter(v => 
			v.category === 'product' || 
			v.category === 'group' || 
			v.source === 'user_input'
		);
	}
	
	async function createProduct() {
		try {
			// Set group ID
			newProduct.group_id = groupId;
			
			// Add dynamic fields to metadata
			if (Object.keys(dynamicFields).length > 0) {
				newProduct.metadata = { ...newProduct.metadata, ...dynamicFields };
			}
			
			// Add variable values
			newProduct.variable_values = variableValues;
			
			const result = await api.post(`/user/groups/${groupId}/products`, newProduct);
			if (result) {
				// Reset form
				newProduct = {
					name: '',
					display_name: '',
					description: '',
					product_type_id: '',
					sku: '',
					base_price: 0,
					currency: 'USD',
					is_active: true,
					variable_values: {},
					metadata: {}
				};
				dynamicFields = {};
				variableValues = {};
				showCreateModal = false;
				// Reload products
				await loadData();
			}
		} catch (error) {
			console.error('Failed to create product:', error);
			alert('Failed to create product');
		}
	}
	
	async function editProduct(product: Product) {
		selectedProduct = { ...product };
		
		// Load dynamic fields if product type has schema
		const productType = getProductTypeInfo(product.product_type_id);
		if (productType && productType.fields_schema) {
			dynamicFields = {};
			Object.entries(productType.fields_schema).forEach(([key, field]: [string, any]) => {
				dynamicFields[key] = product.metadata?.[key] || field.default || '';
			});
		}
		
		// Load variable values
		variableValues = product.variable_values || {};
		
		showEditModal = true;
	}
	
	async function updateProduct() {
		if (!selectedProduct) return;
		
		try {
			// Add dynamic fields to metadata
			if (Object.keys(dynamicFields).length > 0) {
				selectedProduct.metadata = { ...selectedProduct.metadata, ...dynamicFields };
			}
			
			// Update variable values
			selectedProduct.variable_values = variableValues;
			
			const result = await api.put(`/user/products/${selectedProduct.id}`, selectedProduct);
			if (result) {
				showEditModal = false;
				selectedProduct = null;
				dynamicFields = {};
				variableValues = {};
				// Reload products
				await loadData();
			}
		} catch (error) {
			console.error('Failed to update product:', error);
			alert('Failed to update product');
		}
	}
	
	async function deleteProduct(id: string) {
		if (!confirm('Are you sure you want to delete this product?')) return;
		
		try {
			await api.delete(`/user/products/${id}`);
			// Reload products
			await loadData();
		} catch (error) {
			console.error('Failed to delete product:', error);
			alert('Failed to delete product');
		}
	}

	async function openPricingModal(product: Product) {
		selectedProduct = product;
		variableValues = product.variable_values || {};
		showPricingModal = true;
	}

	async function calculatePrice() {
		if (!selectedProduct) return;
		
		try {
			const result = await api.post('/products/calculate-price', {
				product_id: selectedProduct.id,
				variables: variableValues
			});
			
			selectedProduct.calculated_price = result.price;
		} catch (error) {
			console.error('Failed to calculate price:', error);
		}
	}

	function formatPrice(price: number, currency: string = 'USD') {
		return new Intl.NumberFormat('en-US', {
			style: 'currency',
			currency: currency
		}).format(price);
	}
</script>

<div class="page-container">
	<!-- Header -->
	<div class="page-header">
		<div class="header-content">
			<div class="header-left">
				<button class="btn-icon" on:click={() => goto('/profile/products')} title="Back to Groups">
					<ArrowLeft size={20} />
				</button>
				<div>
					<div class="header-title">
						<Package size={24} />
						<h1>Products - {group?.display_name || 'Loading...'}</h1>
					</div>
					<p class="header-subtitle">Manage products for this group</p>
				</div>
			</div>
			<div class="header-actions">
				{#if isAdmin}
					<button class="btn btn-primary" on:click={() => showCreateModal = true}>
						<Plus size={16} />
						New Product
					</button>
				{/if}
			</div>
		</div>
	</div>

	<!-- Main Content -->
	<div class="content-card">
		<!-- Toolbar -->
		<div class="toolbar">
			<div class="toolbar-left">
				<div class="search-box">
					<Search size={16} />
					<input 
						type="text" 
						placeholder="Search products..."
						bind:value={searchQuery}
					/>
				</div>
				<select class="filter-select" bind:value={selectedType}>
					<option value="all">All Types</option>
					{#each productTypes as type}
						<option value={type.id}>{type.display_name}</option>
					{/each}
				</select>
			</div>
			<div class="toolbar-right">
				<button class="btn-icon">
					<Filter size={16} />
				</button>
			</div>
		</div>

		<!-- Products Grid -->
		{#if loading}
			<div class="loading-container">
				<div class="loading loading-spinner loading-lg text-cyan-600"></div>
			</div>
		{:else if filteredProducts.length === 0}
			<div class="empty-state">
				<Package size={48} class="text-gray-400" />
				<h3>No products found</h3>
				{#if isAdmin}
					<p>Add your first product to this group</p>
					<button class="btn btn-primary mt-4" on:click={() => showCreateModal = true}>
						<Plus size={16} />
						Add Product
					</button>
				{:else}
					<p>Product creation is currently restricted to administrators only.</p>
					<p class="text-sm text-gray-500 mt-2">Please contact an administrator if you need to create products.</p>
				{/if}
			</div>
		{:else}
			<div class="products-grid">
				{#each filteredProducts as product}
					{@const productType = getProductTypeInfo(product.product_type_id)}
					<div class="product-card">
						<div class="product-header">
							<div class="product-badge">
								{productType?.display_name || 'Unknown'}
							</div>
							<div class="product-actions">
								<button class="btn-icon" on:click={() => openPricingModal(product)} title="Pricing">
									<Calculator size={16} />
								</button>
								{#if isAdmin}
									<button class="btn-icon" on:click={() => editProduct(product)} title="Edit">
										<Edit2 size={16} />
									</button>
									<button class="btn-icon btn-icon-danger" on:click={() => deleteProduct(product.id)} title="Delete">
										<Trash2 size={16} />
									</button>
								{/if}
							</div>
						</div>
						<div class="product-body">
							<h3 class="product-name">{product.display_name}</h3>
							{#if product.sku}
								<p class="product-sku">SKU: {product.sku}</p>
							{/if}
							{#if product.description}
								<p class="product-description">{product.description}</p>
							{/if}
							
							<div class="product-pricing">
								<div class="price-row">
									<span class="price-label">Base Price:</span>
									<span class="price-value">{formatPrice(product.base_price, product.currency)}</span>
								</div>
								{#if product.calculated_price && product.calculated_price !== product.base_price}
									<div class="price-row calculated">
										<span class="price-label">Calculated:</span>
										<span class="price-value">{formatPrice(product.calculated_price, product.currency)}</span>
									</div>
								{/if}
							</div>
							
							{#if product.variable_values && Object.keys(product.variable_values).length > 0}
								<div class="product-variables">
									<p class="variables-label">Variables:</p>
									<div class="variables-list">
										{#each Object.entries(product.variable_values) as [key, value]}
											<span class="variable-chip">
												{key}: {value}
											</span>
										{/each}
									</div>
								</div>
							{/if}
							
							<div class="product-footer">
								<span class="status-badge {product.is_active ? 'status-active' : 'status-inactive'}">
									{product.is_active ? 'Active' : 'Inactive'}
								</span>
								<button class="btn-link" on:click={() => openPricingModal(product)}>
									<DollarSign size={14} />
									Configure Pricing
								</button>
							</div>
						</div>
					</div>
				{/each}
			</div>
		{/if}
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
		align-items: center;
	}

	.header-left {
		display: flex;
		align-items: center;
		gap: 1rem;
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

	.content-card {
		background: white;
		border-radius: 0.5rem;
		border: 1px solid #e5e7eb;
		overflow: hidden;
	}

	.toolbar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem 1.5rem;
		border-bottom: 1px solid #e5e7eb;
		gap: 1rem;
	}

	.toolbar-left {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		flex: 1;
	}

	.toolbar-right {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.search-box {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 0.75rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		background: white;
		flex: 1;
		max-width: 320px;
	}

	.search-box input {
		border: none;
		outline: none;
		flex: 1;
		font-size: 0.875rem;
	}

	.filter-select {
		padding: 0.5rem 0.75rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		background: white;
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

	.btn-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		background: white;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-icon:hover {
		background: #f9fafb;
	}

	.btn-icon-danger {
		color: #ef4444;
	}

	.btn-icon-danger:hover {
		background: #fee2e2;
		border-color: #fca5a5;
	}

	.btn-link {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		background: none;
		border: none;
		color: #06b6d4;
		font-size: 0.875rem;
		cursor: pointer;
		padding: 0;
	}

	.btn-link:hover {
		color: #0891b2;
	}

	.products-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
		gap: 1.5rem;
		padding: 1.5rem;
	}

	.product-card {
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		overflow: hidden;
		transition: all 0.2s;
	}

	.product-card:hover {
		box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
	}

	.product-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		background: #f9fafb;
		border-bottom: 1px solid #e5e7eb;
	}

	.product-badge {
		display: inline-block;
		padding: 0.25rem 0.75rem;
		background: #e0f2fe;
		color: #075985;
		border-radius: 9999px;
		font-size: 0.75rem;
		font-weight: 500;
	}

	.product-actions {
		display: flex;
		gap: 0.5rem;
	}

	.product-body {
		padding: 1rem;
	}

	.product-name {
		font-size: 1.125rem;
		font-weight: 600;
		color: #111827;
		margin: 0 0 0.25rem 0;
	}

	.product-sku {
		font-size: 0.75rem;
		color: #6b7280;
		font-family: 'Courier New', monospace;
		margin: 0 0 0.5rem 0;
	}

	.product-description {
		font-size: 0.875rem;
		color: #6b7280;
		margin: 0 0 1rem 0;
	}

	.product-pricing {
		background: #f9fafb;
		border-radius: 0.375rem;
		padding: 0.75rem;
		margin-bottom: 1rem;
	}

	.price-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.5rem;
	}

	.price-row:last-child {
		margin-bottom: 0;
	}

	.price-row.calculated {
		padding-top: 0.5rem;
		border-top: 1px solid #e5e7eb;
	}

	.price-label {
		font-size: 0.875rem;
		color: #6b7280;
	}

	.price-value {
		font-size: 1rem;
		font-weight: 600;
		color: #111827;
	}

	.product-variables {
		margin-bottom: 1rem;
	}

	.variables-label {
		font-size: 0.75rem;
		font-weight: 600;
		color: #6b7280;
		text-transform: uppercase;
		margin: 0 0 0.5rem 0;
	}

	.variables-list {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.variable-chip {
		display: inline-block;
		padding: 0.25rem 0.5rem;
		background: #fef3c7;
		color: #92400e;
		font-size: 0.75rem;
		font-family: 'Courier New', monospace;
		border-radius: 0.25rem;
	}

	.product-footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding-top: 1rem;
		border-top: 1px solid #f3f4f6;
	}

	.status-badge {
		display: inline-block;
		padding: 0.25rem 0.75rem;
		border-radius: 9999px;
		font-size: 0.75rem;
		font-weight: 500;
	}

	.status-active {
		background: #d1fae5;
		color: #065f46;
	}

	.status-inactive {
		background: #fee2e2;
		color: #991b1b;
	}

	.loading-container {
		display: flex;
		justify-content: center;
		align-items: center;
		padding: 4rem;
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 4rem;
		text-align: center;
	}

	.empty-state h3 {
		margin: 1rem 0 0.5rem 0;
		font-size: 1.125rem;
		font-weight: 600;
		color: #111827;
	}

	.empty-state p {
		margin: 0;
		color: #6b7280;
		font-size: 0.875rem;
	}

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
		max-width: 700px;
		max-height: 90vh;
		overflow-y: auto;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-bottom: 1px solid #e5e7eb;
	}

	.modal-header h2 {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 600;
		color: #111827;
	}

	.modal-body {
		padding: 1.5rem;
	}

	.form-group {
		margin-bottom: 1rem;
	}

	.form-group label {
		display: block;
		margin-bottom: 0.5rem;
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
	}

	.form-group input,
	.form-group select,
	.form-group textarea {
		width: 100%;
		padding: 0.5rem 0.75rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		font-size: 0.875rem;
	}

	.form-group input:focus,
	.form-group select:focus,
	.form-group textarea:focus {
		outline: none;
		border-color: #06b6d4;
		box-shadow: 0 0 0 3px rgba(6, 182, 212, 0.1);
	}

	.form-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid #e5e7eb;
	}

	.dynamic-fields {
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		padding: 1rem;
		margin-top: 1rem;
	}

	.dynamic-fields h4 {
		font-size: 0.875rem;
		font-weight: 600;
		color: #374151;
		margin: 0 0 1rem 0;
	}

	.variables-section {
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		padding: 1rem;
		margin-top: 1rem;
	}

	.variables-section h4 {
		font-size: 0.875rem;
		font-weight: 600;
		color: #374151;
		margin: 0 0 1rem 0;
	}

	.pricing-result {
		background: #ecfdf5;
		border: 1px solid #86efac;
		border-radius: 0.375rem;
		padding: 1rem;
		margin-top: 1rem;
		text-align: center;
	}

	.pricing-result .price {
		font-size: 2rem;
		font-weight: 600;
		color: #059669;
	}

	.pricing-result .label {
		font-size: 0.875rem;
		color: #6b7280;
		margin-top: 0.25rem;
	}
</style>

<!-- Create Product Modal -->
{#if showCreateModal}
	<div class="modal-overlay" on:click={() => showCreateModal = false}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h2>Add New Product</h2>
				<button class="btn-icon" on:click={() => showCreateModal = false}>
					×
				</button>
			</div>
			<div class="modal-body">
				<div class="form-group">
					<label for="product_type">Product Type</label>
					<select id="product_type" bind:value={newProduct.product_type_id} on:change={onProductTypeChange}>
						<option value="">Select Product Type</option>
						{#each productTypes as type}
							<option value={type.id}>{type.display_name}</option>
						{/each}
					</select>
				</div>
				
				<div class="form-row">
					<div class="form-group">
						<label for="name">Product Name</label>
						<input type="text" id="name" bind:value={newProduct.name} 
							placeholder="e.g., premium_plan, basic_widget" />
					</div>
					<div class="form-group">
						<label for="display_name">Display Name</label>
						<input type="text" id="display_name" bind:value={newProduct.display_name} 
							placeholder="e.g., Premium Plan, Basic Widget" />
					</div>
				</div>
				
				<div class="form-row">
					<div class="form-group">
						<label for="sku">SKU (Optional)</label>
						<input type="text" id="sku" bind:value={newProduct.sku} 
							placeholder="e.g., PRD-001" />
					</div>
					<div class="form-group">
						<label for="base_price">Base Price</label>
						<input type="number" id="base_price" bind:value={newProduct.base_price} 
							step="0.01" min="0" />
					</div>
				</div>
				
				<div class="form-group">
					<label for="description">Description</label>
					<textarea id="description" bind:value={newProduct.description} rows="2" 
						placeholder="Describe this product"></textarea>
				</div>
				
				{#if Object.keys(dynamicFields).length > 0}
					<div class="dynamic-fields">
						<h4>Product Details</h4>
						{#each Object.entries(dynamicFields) as [key, value]}
							{@const fieldSchema = productTypes.find(t => t.id === newProduct.product_type_id)?.fields_schema?.[key]}
							<div class="form-group">
								<label for="dynamic-{key}">{fieldSchema?.label || key}</label>
								{#if fieldSchema?.type === 'boolean'}
									<select id="dynamic-{key}" bind:value={dynamicFields[key]}>
										<option value={true}>Yes</option>
										<option value={false}>No</option>
									</select>
								{:else if fieldSchema?.type === 'number'}
									<input type="number" id="dynamic-{key}" bind:value={dynamicFields[key]} />
								{:else if fieldSchema?.type === 'date'}
									<input type="date" id="dynamic-{key}" bind:value={dynamicFields[key]} />
								{:else}
									<input type="text" id="dynamic-{key}" bind:value={dynamicFields[key]} 
										placeholder={fieldSchema?.description || ''} />
								{/if}
							</div>
						{/each}
					</div>
				{/if}
				
				{#if getRelevantVariables().length > 0}
					<div class="variables-section">
						<h4>Pricing Variables</h4>
						{#each getRelevantVariables() as variable}
							{#if variable.source === 'user_input' || variable.category === 'product'}
								<div class="form-group">
									<label for="var-{variable.name}">{variable.display_name}</label>
									{#if variable.type === 'boolean'}
										<select id="var-{variable.name}" bind:value={variableValues[variable.name]}>
											<option value={true}>Yes</option>
											<option value={false}>No</option>
										</select>
									{:else if variable.type === 'number'}
										<input type="number" id="var-{variable.name}" 
											bind:value={variableValues[variable.name]} 
											placeholder="Enter {variable.display_name}" />
									{:else}
										<input type="text" id="var-{variable.name}" 
											bind:value={variableValues[variable.name]} 
											placeholder="Enter {variable.display_name}" />
									{/if}
								</div>
							{/if}
						{/each}
					</div>
				{/if}
				
				<div class="form-group">
					<label for="is_active">Status</label>
					<select id="is_active" bind:value={newProduct.is_active}>
						<option value={true}>Active</option>
						<option value={false}>Inactive</option>
					</select>
				</div>
			</div>
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={() => showCreateModal = false}>Cancel</button>
				<button class="btn btn-primary" on:click={createProduct}>Add Product</button>
			</div>
		</div>
	</div>
{/if}

<!-- Edit Product Modal -->
{#if showEditModal && selectedProduct}
	<div class="modal-overlay" on:click={() => showEditModal = false}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h2>Edit Product</h2>
				<button class="btn-icon" on:click={() => showEditModal = false}>
					×
				</button>
			</div>
			<div class="modal-body">
				<div class="form-row">
					<div class="form-group">
						<label for="edit-name">Product Name</label>
						<input type="text" id="edit-name" bind:value={selectedProduct.name} />
					</div>
					<div class="form-group">
						<label for="edit-display_name">Display Name</label>
						<input type="text" id="edit-display_name" bind:value={selectedProduct.display_name} />
					</div>
				</div>
				
				<div class="form-row">
					<div class="form-group">
						<label for="edit-sku">SKU</label>
						<input type="text" id="edit-sku" bind:value={selectedProduct.sku} />
					</div>
					<div class="form-group">
						<label for="edit-base_price">Base Price</label>
						<input type="number" id="edit-base_price" bind:value={selectedProduct.base_price} 
							step="0.01" min="0" />
					</div>
				</div>
				
				<div class="form-group">
					<label for="edit-description">Description</label>
					<textarea id="edit-description" bind:value={selectedProduct.description} rows="2"></textarea>
				</div>
				
				{#if Object.keys(dynamicFields).length > 0}
					{@const productType = getProductTypeInfo(selectedProduct.product_type_id)}
					<div class="dynamic-fields">
						<h4>Product Details</h4>
						{#each Object.entries(dynamicFields) as [key, value]}
							{@const fieldSchema = productType?.fields_schema?.[key]}
							<div class="form-group">
								<label for="edit-dynamic-{key}">{fieldSchema?.label || key}</label>
								{#if fieldSchema?.type === 'boolean'}
									<select id="edit-dynamic-{key}" bind:value={dynamicFields[key]}>
										<option value={true}>Yes</option>
										<option value={false}>No</option>
									</select>
								{:else if fieldSchema?.type === 'number'}
									<input type="number" id="edit-dynamic-{key}" bind:value={dynamicFields[key]} />
								{:else if fieldSchema?.type === 'date'}
									<input type="date" id="edit-dynamic-{key}" bind:value={dynamicFields[key]} />
								{:else}
									<input type="text" id="edit-dynamic-{key}" bind:value={dynamicFields[key]} />
								{/if}
							</div>
						{/each}
					</div>
				{/if}
				
				{#if getRelevantVariables().length > 0}
					<div class="variables-section">
						<h4>Pricing Variables</h4>
						{#each getRelevantVariables() as variable}
							{#if variable.source === 'user_input' || variable.category === 'product'}
								<div class="form-group">
									<label for="edit-var-{variable.name}">{variable.display_name}</label>
									{#if variable.type === 'boolean'}
										<select id="edit-var-{variable.name}" bind:value={variableValues[variable.name]}>
											<option value={true}>Yes</option>
											<option value={false}>No</option>
										</select>
									{:else if variable.type === 'number'}
										<input type="number" id="edit-var-{variable.name}" 
											bind:value={variableValues[variable.name]} />
									{:else}
										<input type="text" id="edit-var-{variable.name}" 
											bind:value={variableValues[variable.name]} />
									{/if}
								</div>
							{/if}
						{/each}
					</div>
				{/if}
				
				<div class="form-group">
					<label for="edit-is_active">Status</label>
					<select id="edit-is_active" bind:value={selectedProduct.is_active}>
						<option value={true}>Active</option>
						<option value={false}>Inactive</option>
					</select>
				</div>
			</div>
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={() => showEditModal = false}>Cancel</button>
				<button class="btn btn-primary" on:click={updateProduct}>Update Product</button>
			</div>
		</div>
	</div>
{/if}

<!-- Pricing Calculator Modal -->
{#if showPricingModal && selectedProduct}
	<div class="modal-overlay" on:click={() => showPricingModal = false}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h2>Pricing Calculator - {selectedProduct.display_name}</h2>
				<button class="btn-icon" on:click={() => showPricingModal = false}>
					×
				</button>
			</div>
			<div class="modal-body">
				<div class="variables-section">
					<h4>Adjust Variables</h4>
					{#each getRelevantVariables() as variable}
						<div class="form-group">
							<label for="calc-var-{variable.name}">{variable.display_name}</label>
							{#if variable.type === 'boolean'}
								<select id="calc-var-{variable.name}" bind:value={variableValues[variable.name]}>
									<option value={true}>Yes</option>
									<option value={false}>No</option>
								</select>
							{:else if variable.type === 'number'}
								<input type="number" id="calc-var-{variable.name}" 
									bind:value={variableValues[variable.name]} 
									on:change={calculatePrice} />
							{:else}
								<input type="text" id="calc-var-{variable.name}" 
									bind:value={variableValues[variable.name]} 
									on:change={calculatePrice} />
							{/if}
						</div>
					{/each}
				</div>
				
				<button class="btn btn-primary" on:click={calculatePrice}>
					<Calculator size={16} />
					Calculate Price
				</button>
				
				{#if selectedProduct.calculated_price}
					<div class="pricing-result">
						<div class="price">{formatPrice(selectedProduct.calculated_price, selectedProduct.currency)}</div>
						<div class="label">Calculated Price</div>
					</div>
				{/if}
			</div>
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={() => showPricingModal = false}>Close</button>
			</div>
		</div>
	</div>
{/if}