<script lang="ts">
	import { onMount } from 'svelte';
	import {
		Package2, Plus, Edit2, Trash2, Filter,
		CheckCircle, XCircle, Settings
	} from 'lucide-svelte';
	import SearchInput from '$lib/components/SearchInput.svelte';
	import { api, ErrorHandler } from '$lib/api';
	import { requireAdmin } from '$lib/utils/auth';
	import IconPicker from '$lib/components/IconPicker.svelte';
	import FieldEditor from '$lib/components/FieldEditor.svelte';
	import CustomFieldEditor from '$lib/components/CustomFieldEditor.svelte';
	import ReorderableList from '$lib/components/ReorderableList.svelte';
	import PricingTemplateModal from '$lib/components/PricingTemplateModal.svelte';
	import { getIconComponent } from '$lib/utils/icons';
	import { toasts } from '$lib/stores/toast';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import Modal from '$lib/components/ui/Modal.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import PageHeader from '$lib/components/ui/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';

	interface FieldConstraints {
		required?: boolean;
		min?: number;
		max?: number;
		minLength?: number;
		maxLength?: number;
		pattern?: string;
		options?: string[];
		default?: any;
		placeholder?: string;
	}

	interface FieldDefinition {
		id: string;  // e.g., "filter_text_1", "filter_numeric_1"
		name: string;
		label?: string;  // Display label
		type: string;  // numeric, text, boolean, enum, location
		required?: boolean;
		description?: string;
		section?: string;  // e.g., "General"
		order?: number;  // Sort order
		constraints: FieldConstraints;
	}

	// Constraints type for FieldEditor component
	interface EditorFieldConstraints {
		editableByUser: boolean;
		options?: string[];
		min?: number;
		max?: number;
		minLength?: number;
		maxLength?: number;
		format?: string;
	}

	// Field type for FieldEditor component (with non-optional required/description)
	interface EditorField {
		id: string;
		name: string;
		type: string;
		required: boolean;
		description: string;
		constraints: EditorFieldConstraints;
	}

	// Custom field type for CustomFieldEditor component
	interface CustomEditorField {
		id: string;
		name: string;
		type: string;
		required: boolean;
		description: string;
		section: string;
		order: number;
		constraints: EditorFieldConstraints;
	}

	interface ProductType {
		id: string;
		name: string;
		displayName: string;
		description: string;
		icon?: string;
		filterFieldsSchema?: FieldDefinition[];
		customFieldsSchema?: FieldDefinition[];
		pricingTemplates?: string[];
		billingMode: 'instant' | 'approval';
		billingType: 'one-time' | 'recurring';
		billingRecurringInterval?: 'day' | 'week' | 'month' | 'year';
		billingRecurringIntervalCount?: number;
		status: 'active' | 'pending' | 'deleted';
		createdAt: string;
		updatedAt: string;
	}

	interface PricingTemplate {
		id: string;
		name: string;
		displayName: string;
		description: string;
		category: string;
		priceFormula?: string;
		conditionFormula?: string;
		isActive: boolean;
	}

	let productTypes: ProductType[] = [];
	let pricingTemplates: PricingTemplate[] = [];
	let loading = true;
	let searchQuery = '';
	let showCreateModal = false;
	let showEditModal = false;
	let showDeleteConfirm = false;
	let selectedProductType: ProductType | null = null;
	let productTypeToDelete: string | null = null;
	let showCreatePricingModal = false;
	let showEditPricingModal = false;
	let editingPricingTemplate: any = null;
	let newPricingTemplate = {
		name: '',
		displayName: '',
		description: '',
		priceFormula: '',
		conditionFormula: '',
		category: 'standard',
		isActive: true
	};
	let availableVariables: any[] = [];
	let variables: any[] = []; // For formula editor in pricing template modal

	// Form data for new product type
	let newProductType: Partial<ProductType> = {
		name: '',
		displayName: '',
		description: '',
		icon: 'package',
		billingMode: 'instant',
		billingType: 'one-time',
		status: 'active',
		filterFieldsSchema: [],
		customFieldsSchema: [],
		pricingTemplates: []
	};

	$: filteredProductTypes = productTypes.filter(productType => {
		const matchesSearch = productType.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
			productType.displayName.toLowerCase().includes(searchQuery.toLowerCase()) ||
			productType.description?.toLowerCase().includes(searchQuery.toLowerCase());
		return matchesSearch;
	});

	onMount(async () => {
		if (!requireAdmin()) return;
		await loadProductTypes();
		await loadPricingTemplates();
	});

	async function loadProductTypes() {
		try {
			loading = true;
			const response = await api.get<ProductType[]>('/admin/ext/products/product-types');
			productTypes = Array.isArray(response) ? response : [];
		} catch (error) {
			console.error('Failed to load product types:', error);
			productTypes = [];
		} finally {
			loading = false;
		}
	}

	async function loadPricingTemplates() {
		try {
			const response = await api.get<PricingTemplate[]>('/admin/ext/products/pricing-templates');
			pricingTemplates = Array.isArray(response) ? response : [];
		} catch (error) {
			console.error('Failed to load pricing templates:', error);
			pricingTemplates = [];
		}
	}

	async function loadVariables() {
		try {
			const response = await api.get<any[]>('/admin/ext/products/variables');
			availableVariables = Array.isArray(response) ? response : [];
			variables = Array.isArray(response) ? response : []; // Also set variables for formula editor
		} catch (error) {
			console.error('Failed to load variables:', error);
			availableVariables = [];
			variables = [];
		}
	}
	
	async function createPricingTemplate() {
		try {
			const result = await api.post<{ id: string }>('/admin/ext/products/pricing-templates', newPricingTemplate);
			await loadPricingTemplates();

			// Add the new template to the selected product type
			if (selectedProductType) {
				if (!selectedProductType.pricingTemplates) {
					selectedProductType.pricingTemplates = [];
				}
				selectedProductType.pricingTemplates.push(result.id);
			}

			// Reset form
			newPricingTemplate = {
				name: '',
				displayName: '',
				description: '',
				priceFormula: '',
				conditionFormula: '',
				category: 'standard',
				isActive: true
			};
			showCreatePricingModal = false;
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}
	
	function handleCreatePricingTemplate() {
		showCreatePricingModal = true;
		loadVariables();
	}
	
	function handleEditPricingTemplate(event: CustomEvent) {
		const template = event.detail.item;
		editingPricingTemplate = { ...template };
		showEditPricingModal = true;
		loadVariables();
	}
	
	async function updatePricingTemplate() {
		try {
			await api.put(`/admin/ext/products/pricing-templates/${editingPricingTemplate.id}`, editingPricingTemplate);
			await loadPricingTemplates();
			showEditPricingModal = false;
			editingPricingTemplate = null;
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}
	
	function handleCreateVariable(context: string) {
		// This could open a modal to create a new variable
		// For now, just log it
		console.log('Create variable requested from:', context);
		// You could implement a variable creation modal here
		// or navigate to the variables page
		window.open('/admin/extensions/products/variables', '_blank');
	}
	
	async function createProductType() {
		try {
			// Ensure filterFieldsSchema is a valid array
			if (!Array.isArray(newProductType.filterFieldsSchema)) {
				newProductType.filterFieldsSchema = [];
			}
			// Ensure pricingTemplates is a valid array
			if (!Array.isArray(newProductType.pricingTemplates)) {
				newProductType.pricingTemplates = [];
			}

			const result = await api.post('/admin/ext/products/product-types', newProductType);
			if (result) {
				// Reset form
				newProductType = {
					name: '',
					displayName: '',
					description: '',
					icon: 'package',
					billingMode: 'instant',
					billingType: 'one-time',
					billingRecurringInterval: 'month',
					billingRecurringIntervalCount: 1,
					status: 'active',
					filterFieldsSchema: [],
					customFieldsSchema: [],
					pricingTemplates: []
				};
				showCreateModal = false;
				// Reload product types
				await loadProductTypes();
			}
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}
	
	async function saveProductType() {
		if (!selectedProductType) return;

		// Update filter fields (mapped to database columns)
		selectedProductType.filterFieldsSchema = schemaFields.filter(field => field.name).map((field): FieldDefinition => ({
			id: field.id,
			name: field.name,
			type: field.type,
			required: field.required || false,
			description: field.description || '',
			constraints: field.constraints as unknown as FieldConstraints
		}));

		// Update custom fields schema (stored in JSON)
		selectedProductType.customFieldsSchema = customSchemaFields.filter(field => field.name).map((field): FieldDefinition => ({
			id: field.id,
			name: field.name,
			type: field.type,
			required: field.required || false,
			description: field.description || '',
			constraints: field.constraints as unknown as FieldConstraints,
			section: field.section || 'General',
			order: field.order || 0
		}));
		
		try {
			const result = await api.put(`/admin/ext/products/product-types/${selectedProductType.id}`, selectedProductType);
			if (result) {
				showEditModal = false;
				selectedProductType = null;
				schemaFields = [];
				customSchemaFields = [];
				// Reload product types
				await loadProductTypes();
			}
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}
	
	function deleteProductType(id: string) {
		productTypeToDelete = id;
		showDeleteConfirm = true;
	}

	async function confirmDeleteProductType() {
		if (!productTypeToDelete) return;
		showDeleteConfirm = false;

		try {
			await api.delete(`/products/product-types/${productTypeToDelete}`);
			// Reload product types
			await loadProductTypes();
		} catch (error) {
			ErrorHandler.handle(error);
		}
		productTypeToDelete = null;
	}

	// Schema editor state
	let schemaFields: EditorField[] = [];  // For filter fields (mapped to DB columns)
	let customSchemaFields: CustomEditorField[] = [];  // For custom fields (stored in JSON)
	let showFieldTypeSelector = false;
	
	// Track used filter IDs
	function getUsedFilterIds(fields: FieldDefinition[]): Map<string, number> {
		const usage = new Map<string, number>();
		usage.set('numeric', 0);
		usage.set('text', 0);
		usage.set('boolean', 0);
		usage.set('enum', 0);
		usage.set('location', 0);
		
		fields.forEach(field => {
			if (field.id) {
				const parts = field.id.split('_');
				if (parts.length === 3 && parts[0] === 'filter') {
					const type = parts[1];
					const num = parseInt(parts[2]);
					if (usage.has(type)) {
						usage.set(type, Math.max(usage.get(type) || 0, num));
					}
				}
			}
		});
		
		return usage;
	}
	
	// Get next available filter ID for a type
	function getNextFilterId(type: string, fields: FieldDefinition[]): string | null {
		const usage = getUsedFilterIds(fields);
		const count = usage.get(type) || 0;
		
		// Check if we've reached the limit of 5
		if (count >= 5) {
			return null;
		}
		
		// Find the next available number (might not be count+1 if some were deleted)
		for (let i = 1; i <= 5; i++) {
			const id = `filter_${type}_${i}`;
			if (!fields.some(f => f.id === id)) {
				return id;
			}
		}
		
		return null;
	}
	
	// Check if a field type is available
	function isTypeAvailable(type: string, fields: FieldDefinition[]): boolean {
		return getNextFilterId(type, fields) !== null;
	}
	
	// Count fields by type
	function countFieldsByType(type: string, fields: FieldDefinition[]): number {
		return fields.filter(f => {
			if (f.id) {
				const parts = f.id.split('_');
				return parts[0] === 'filter' && parts[1] === type;
			}
			return false;
		}).length;
	}

	function openEditModal(productType: ProductType) {
		selectedProductType = { ...productType };
		// Load filter fields (that map to database columns)
		if (Array.isArray(productType.filterFieldsSchema)) {
			schemaFields = productType.filterFieldsSchema.map((field: FieldDefinition) => ({
				id: field.id || '',
				name: field.name || '',
				type: field.type || 'text',
				required: field.required || false,
				description: field.description || '',
				constraints: { editableByUser: true, ...field.constraints }
			}));
		} else {
			schemaFields = [];
		}
		// Load custom fields schema (stored in JSON)
		if (Array.isArray(productType.customFieldsSchema)) {
			customSchemaFields = productType.customFieldsSchema.map((field: FieldDefinition) => ({
				id: field.id || '',
				name: field.name || '',
				type: field.type || 'text',
				required: field.required || false,
				description: field.description || '',
				section: field.section || 'General',
				order: field.order || 0,
				constraints: { editableByUser: true, ...field.constraints }
			}));
		} else {
			customSchemaFields = [];
		}
		// Ensure pricingTemplates is an array
		if (!Array.isArray(selectedProductType.pricingTemplates)) {
			selectedProductType.pricingTemplates = [];
		}
		// Set default billing values if not present
		if (!selectedProductType.billingMode) {
			selectedProductType.billingMode = 'instant';
		}
		if (!selectedProductType.billingType) {
			selectedProductType.billingType = 'one-time';
		}
		if (selectedProductType.billingType === 'recurring') {
			if (!selectedProductType.billingRecurringInterval) {
				selectedProductType.billingRecurringInterval = 'month';
			}
			if (!selectedProductType.billingRecurringIntervalCount) {
				selectedProductType.billingRecurringIntervalCount = 1;
			}
		}
		showFieldTypeSelector = false;
		showEditModal = true;
	}

	function addSchemaField(type: string) {
		const id = getNextFilterId(type, schemaFields as unknown as FieldDefinition[]);
		if (!id) {
			toasts.warning(`Maximum of 5 ${type} fields reached`);
			return;
		}

		schemaFields = [...schemaFields, {
			id: id,
			name: '',
			type: type,
			required: false,
			description: '',
			constraints: { editableByUser: true }
		}];
		showFieldTypeSelector = false;
	}

	function removeSchemaField(index: number) {
		schemaFields = schemaFields.filter((_, i) => i !== index);
	}
</script>

<div class="page-container">
	<!-- Header -->
	<PageHeader
		title="Product Types"
		subtitle="Define types of entities that can own products (e.g., Store, Company, Team)"
		icon={Package2}
		backHref="/admin/extensions/products"
		variant="card"
	>
		<svelte:fragment slot="actions">
			<Button icon={Plus} on:click={() => showCreateModal = true}>
				New Product Type
			</Button>
		</svelte:fragment>
	</PageHeader>

	<!-- Main Content -->
	<div class="content-card">
		<!-- Toolbar -->
		<div class="toolbar">
			<div class="toolbar-left">
				<SearchInput bind:value={searchQuery} placeholder="Search product types..." maxWidth="320px" />
			</div>
			<div class="toolbar-right">
				<button class="btn-icon">
					<Filter size={16} />
				</button>
			</div>
		</div>

		<!-- Product Types Grid -->
		{#if loading}
			<div class="loading-container">
				<div class="loading loading-spinner loading-lg text-cyan-600"></div>
			</div>
		{:else if filteredProductTypes.length === 0}
			<EmptyState
				icon={Package2}
				title="No product types found"
				message="Create your first product type to organize your business structure"
			>
				<Button icon={Plus} on:click={() => showCreateModal = true}>
					Create Product Type
				</Button>
			</EmptyState>
		{:else}
			<div class="group-grid">
				{#each filteredProductTypes as productType}
					<div class="group-card" on:click={() => openEditModal(productType)} role="button" tabindex="0" on:keypress={(e) => e.key === 'Enter' && openEditModal(productType)}>
						<div class="group-header">
							<div class="group-icon">
								<svelte:component this={getIconComponent(productType.icon)} size={24} />
							</div>
							<span class="status-badge status-{productType.status}">
								{#if productType.status === 'active'}
									<CheckCircle size={12} />
									Active
								{:else if productType.status === 'pending'}
									<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
										<circle cx="12" cy="12" r="10"></circle>
										<polyline points="12 6 12 12 16 14"></polyline>
									</svg>
									Pending
								{:else}
									<XCircle size={12} />
									Deleted
								{/if}
							</span>
						</div>
						<div class="group-content">
							<h3 class="group-name">{productType.displayName}</h3>
							<code class="group-code">{productType.name}</code>
							<p class="group-description">{productType.description}</p>

							{#if productType.filterFieldsSchema && productType.filterFieldsSchema.length > 0}
								<div class="group-fields">
									<p class="fields-label">Filter Fields:</p>
									<div class="fields-list">
										{#each productType.filterFieldsSchema as field}
											<span class="field-badge" title="{field.type}{field.constraints?.required ? ' (required)' : ''}">{field.label || field.name}</span>
										{/each}
									</div>
								</div>
							{/if}

							{#if productType.pricingTemplates && productType.pricingTemplates.length > 0}
								<div class="group-fields">
									<p class="fields-label">Pricing Templates:</p>
									<div class="fields-list">
										{#each productType.pricingTemplates as templateId}
											{@const template = pricingTemplates.find(t => t.id === templateId)}
											{#if template}
												<span class="pricing-badge">{template.displayName}</span>
											{/if}
										{/each}
									</div>
								</div>
							{/if}

							<div class="billing-info">
								<span class="billing-badge billing-{productType.billingMode}">
									{productType.billingMode === 'instant' ? '⚡' : '✓'} {productType.billingMode}
								</span>
								<span class="billing-badge billing-{productType.billingType}">
									{#if productType.billingType === 'recurring'}
										🔄 {productType.billingRecurringIntervalCount || 1}
										{productType.billingRecurringInterval || 'month'}
									{:else}
										💳 One-time
									{/if}
								</span>
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

	.field-description {
		font-size: 0.875rem;
		color: #6b7280;
		margin: 0.25rem 0 0.75rem 0;
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
		font-size: 0.75rem;
		cursor: pointer;
		padding: 0;
	}

	.btn-link:hover {
		color: #0891b2;
	}

	.group-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
		gap: 1.5rem;
		padding: 1.5rem;
	}

	.group-card {
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		overflow: hidden;
		transition: all 0.2s;
		cursor: pointer;
		background: white;
	}

	.group-card:hover {
		box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
		border-color: #06b6d4;
		transform: translateY(-2px);
	}
	
	.group-card:focus {
		outline: none;
		border-color: #06b6d4;
		box-shadow: 0 0 0 3px rgba(6, 182, 212, 0.1);
	}

	.group-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		background: #f9fafb;
		border-bottom: 1px solid #e5e7eb;
	}

	.group-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 48px;
		height: 48px;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		color: #06b6d4;
	}

	.group-actions {
		display: flex;
		gap: 0.5rem;
	}

	.group-content {
		padding: 1rem;
	}

	.group-name {
		font-size: 1.125rem;
		font-weight: 600;
		color: #111827;
		margin: 0 0 0.25rem 0;
	}

	.group-code {
		display: inline-block;
		font-family: 'Courier New', monospace;
		font-size: 0.75rem;
		color: #6b7280;
		background: #f3f4f6;
		padding: 0.125rem 0.375rem;
		border-radius: 0.25rem;
		margin-bottom: 0.75rem;
	}

	.group-description {
		font-size: 0.875rem;
		color: #6b7280;
		margin: 0 0 1rem 0;
	}

	.group-fields {
		margin-bottom: 1rem;
	}

	.fields-label {
		font-size: 0.75rem;
		font-weight: 600;
		color: #6b7280;
		text-transform: uppercase;
		margin: 0 0 0.5rem 0;
	}

	.fields-list {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.field-badge {
		display: inline-block;
		padding: 0.25rem 0.5rem;
		background: #ecfdf5;
		color: #059669;
		font-size: 0.75rem;
		font-weight: 500;
		border-radius: 0.25rem;
	}
	
	.help-text {
		font-size: 0.875rem;
		color: #6b7280;
		margin: 0.25rem 0 0 0;
	}

	.group-footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding-top: 1rem;
		border-top: 1px solid #f3f4f6;
	}

	.status-badge {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.25rem 0.75rem;
		border-radius: 9999px;
		font-size: 0.75rem;
		font-weight: 500;
	}

	.status-active {
		background: #d1fae5;
		color: #065f46;
	}

	.status-pending {
		background: #fed7aa;
		color: #9a3412;
	}

	.status-deleted {
		background: #fee2e2;
		color: #991b1b;
	}

	.loading-container {
		display: flex;
		justify-content: center;
		align-items: center;
		padding: 4rem;
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

	.modal-footer-right {
		display: flex;
		gap: 0.75rem;
	}

	.schema-editor {
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		padding: 1rem;
		background: #f9fafb;
	}

	.schema-field {
		display: grid;
		grid-template-columns: 2fr 2fr 1fr auto auto;
		gap: 0.5rem;
		align-items: center;
		margin-bottom: 0.5rem;
		padding: 0.5rem;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.25rem;
	}

	.schema-field input,
	.schema-field select {
		padding: 0.375rem 0.5rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.25rem;
		font-size: 0.75rem;
	}

	.schema-field label {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		font-size: 0.75rem;
	}

	.btn-add-field {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.375rem 0.75rem;
		background: #06b6d4;
		color: white;
		border: none;
		border-radius: 0.25rem;
		font-size: 0.75rem;
		font-weight: 500;
		cursor: pointer;
		margin-top: 0.5rem;
	}

	.btn-add-field:hover {
		background: #0891b2;
	}

	.btn-remove {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		background: #fee2e2;
		color: #ef4444;
		border: none;
		border-radius: 0.25rem;
		cursor: pointer;
	}

	.btn-remove:hover {
		background: #fca5a5;
	}

	/* Improved Field Editor Styles */
	.field-editor-card {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		padding: 1rem;
		margin-bottom: 1rem;
	}

	.field-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
		padding-bottom: 0.75rem;
		border-bottom: 1px solid #f3f4f6;
	}

	.field-header h4 {
		margin: 0;
		font-size: 0.875rem;
		font-weight: 600;
		color: #374151;
	}

	.field-editor-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.75rem;
	}

	.field-col {
		display: flex;
		flex-direction: column;
	}

	.field-col.full-width {
		grid-column: span 2;
	}

	.field-col label {
		font-size: 0.75rem;
		font-weight: 500;
		color: #6b7280;
		margin-bottom: 0.25rem;
	}

	.field-col input,
	.field-col select {
		padding: 0.375rem 0.5rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.25rem;
		font-size: 0.813rem;
	}

	.constraints-section {
		margin-top: 1rem;
		padding-top: 1rem;
		border-top: 1px solid #f3f4f6;
	}

	.constraints-section h5 {
		margin: 0 0 0.75rem 0;
		font-size: 0.813rem;
		font-weight: 600;
		color: #374151;
	}

	.constraints-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.75rem;
	}

	.constraint-row {
		display: flex;
		flex-direction: column;
	}

	.constraint-row.full-width {
		grid-column: span 2;
	}

	.constraint-row label {
		font-size: 0.75rem;
		font-weight: 500;
		color: #6b7280;
		margin-bottom: 0.25rem;
	}

	.constraint-row input,
	.constraint-row select {
		padding: 0.375rem 0.5rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.25rem;
		font-size: 0.813rem;
	}

	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.813rem;
		color: #374151;
		grid-column: span 2;
	}

	.checkbox-label input[type="checkbox"] {
		margin: 0;
	}

	.full-width {
		grid-column: span 2;
	}
	
	/* Pricing Templates Selector */
	.pricing-templates-selector {
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		background: #fafbfc;
		overflow: hidden;
	}
	
	.no-templates {
		margin: 0;
		color: #6b7280;
		font-size: 0.875rem;
		text-align: center;
		padding: 2rem;
	}
	
	.templates-list {
		max-height: 240px;
		overflow-y: auto;
	}
	
	.template-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.875rem 1rem;
		background: white;
		border-bottom: 1px solid #f3f4f6;
		cursor: pointer;
		transition: all 0.2s;
		position: relative;
	}
	
	.template-item:last-child {
		border-bottom: none;
	}
	
	.template-item:hover {
		background: #f9fafb;
	}
	
	.template-item.selected {
		background: linear-gradient(to right, #ecfeff 0%, #f0fdfa 100%);
		border-left: 3px solid #06b6d4;
		padding-left: calc(1rem - 3px);
	}
	
	.template-item:focus {
		outline: none;
		box-shadow: inset 0 0 0 2px #06b6d4;
	}
	
	.template-check {
		width: 20px;
		height: 20px;
		border: 2px solid #d1d5db;
		border-radius: 0.375rem;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.2s;
		flex-shrink: 0;
		background: white;
	}
	
	.template-item.selected .template-check {
		background: #06b6d4;
		border-color: #06b6d4;
		color: white;
	}
	
	.template-content {
		flex: 1;
		min-width: 0;
	}
	
	.template-name {
		font-size: 0.875rem;
		font-weight: 600;
		color: #111827;
		margin-bottom: 0.125rem;
	}
	
	.template-item.selected .template-name {
		color: #0891b2;
	}
	
	.template-meta {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.75rem;
		color: #6b7280;
	}
	
	.template-category {
		padding: 0.125rem 0.375rem;
		background: #f3f4f6;
		border-radius: 0.25rem;
		font-weight: 500;
		text-transform: lowercase;
	}
	
	.template-item.selected .template-category {
		background: #e0f2fe;
		color: #0369a1;
	}
	
	.template-separator {
		color: #d1d5db;
	}
	
	.template-description {
		color: #9ca3af;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	
	.pricing-badge {
		display: inline-block;
		padding: 0.25rem 0.5rem;
		background: #fef3c7;
		color: #92400e;
		font-size: 0.75rem;
		font-weight: 500;
		border-radius: 0.25rem;
	}
	
	.billing-info {
		display: flex;
		gap: 0.5rem;
		margin-top: 0.75rem;
		padding-top: 0.75rem;
		border-top: 1px solid #f3f4f6;
	}
	
	.billing-badge {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.25rem 0.5rem;
		font-size: 0.75rem;
		font-weight: 500;
		border-radius: 0.25rem;
		text-transform: capitalize;
	}
	
	.billing-instant {
		background: #ecfdf5;
		color: #065f46;
	}
	
	.billing-approval {
		background: #fef3c7;
		color: #92400e;
	}
	
	.billing-one-time {
		background: #ede9fe;
		color: #5b21b6;
	}
	
	.billing-recurring {
		background: #dbeafe;
		color: #1e40af;
	}
	
	/* Custom scrollbar for templates list */
	.templates-list::-webkit-scrollbar {
		width: 6px;
	}
	
	.templates-list::-webkit-scrollbar-track {
		background: #f3f4f6;
		border-radius: 3px;
	}
	
	.templates-list::-webkit-scrollbar-thumb {
		background: #d1d5db;
		border-radius: 3px;
	}
	
	.templates-list::-webkit-scrollbar-thumb:hover {
		background: #9ca3af;
	}
</style>

<!-- Create Product Type Modal -->
<Modal show={showCreateModal} title="Create New Product Type" maxWidth="600px" on:close={() => showCreateModal = false}>
	<div class="form-row">
		<div class="form-group">
			<label for="name">Product Type Name</label>
			<input type="text" id="name" bind:value={newProductType.name}
				placeholder="e.g., store, company, team" />
		</div>
		<div class="form-group">
			<label for="displayName">Display Name</label>
			<input type="text" id="displayName" bind:value={newProductType.displayName}
				placeholder="e.g., Store, Company, Team" />
		</div>
	</div>
	<div class="form-group">
		<label for="description">Description</label>
		<textarea id="description" bind:value={newProductType.description} rows="2"
			placeholder="Describe what this product type represents"></textarea>
	</div>
	<div class="form-group">
		<label for="icon">Icon</label>
		<IconPicker bind:value={newProductType.icon} placeholder="Choose an icon" />
	</div>
	<div class="form-row">
		<div class="form-group">
			<label for="billingMode">Billing Mode</label>
			<select id="billingMode" bind:value={newProductType.billingMode}>
				<option value="instant">Instant</option>
				<option value="approval">Requires Approval</option>
			</select>
		</div>
		<div class="form-group">
			<label for="billingType">Billing Type</label>
			<select id="billingType" bind:value={newProductType.billingType}>
				<option value="one-time">One-time</option>
				<option value="recurring">Recurring</option>
			</select>
		</div>
	</div>
	{#if newProductType.billingType === 'recurring'}
		<div class="form-row">
			<div class="form-group">
				<label for="billingInterval">Recurring Interval</label>
				<select id="billingInterval" bind:value={newProductType.billingRecurringInterval}>
					<option value="day">Daily</option>
					<option value="week">Weekly</option>
					<option value="month">Monthly</option>
					<option value="year">Yearly</option>
				</select>
			</div>
			<div class="form-group">
				<label for="billingIntervalCount">Interval Count</label>
				<input type="number" id="billingIntervalCount" min="1" bind:value={newProductType.billingRecurringIntervalCount} placeholder="e.g., 2 for bi-weekly" />
			</div>
		</div>
	{/if}
	<div class="form-group">
		<label for="status">Status</label>
		<select id="status" bind:value={newProductType.status}>
			<option value="active">Active</option>
			<option value="pending">Pending</option>
			<option value="deleted">Deleted</option>
		</select>
	</div>
	<svelte:fragment slot="footer">
		<Button variant="secondary" on:click={() => showCreateModal = false}>Cancel</Button>
		<Button on:click={createProductType}>Create Product Type</Button>
	</svelte:fragment>
</Modal>

<!-- Edit Product Type Modal -->
<Modal show={showEditModal && !!selectedProductType} title="Edit Product Type" maxWidth="800px" on:close={() => { showEditModal = false; schemaFields = []; customSchemaFields = []; }}>
	{#if selectedProductType}
		<div class="form-row">
			<div class="form-group">
				<label for="edit-name">Product Type Name</label>
				<input type="text" id="edit-name" bind:value={selectedProductType.name} />
			</div>
			<div class="form-group">
				<label for="edit-displayName">Display Name</label>
				<input type="text" id="edit-displayName" bind:value={selectedProductType.displayName} />
			</div>
		</div>
		<div class="form-group">
			<label for="edit-description">Description</label>
			<textarea id="edit-description" bind:value={selectedProductType.description} rows="2"></textarea>
		</div>
		<div class="form-group">
			<label for="edit-icon">Icon</label>
			<IconPicker bind:value={selectedProductType.icon} placeholder="Choose an icon" />
		</div>

		<div class="form-row">
			<div class="form-group">
				<label for="edit-billingMode">Billing Mode</label>
				<select id="edit-billingMode" bind:value={selectedProductType.billingMode}>
					<option value="instant">Instant</option>
					<option value="approval">Requires Approval</option>
				</select>
			</div>
			<div class="form-group">
				<label for="edit-billingType">Billing Type</label>
				<select id="edit-billingType" bind:value={selectedProductType.billingType}>
					<option value="one-time">One-time</option>
					<option value="recurring">Recurring</option>
				</select>
			</div>
		</div>

		{#if selectedProductType.billingType === 'recurring'}
			<div class="form-row">
				<div class="form-group">
					<label for="edit-billingInterval">Recurring Interval</label>
					<select id="edit-billingInterval" bind:value={selectedProductType.billingRecurringInterval}>
						<option value="day">Daily</option>
						<option value="week">Weekly</option>
						<option value="month">Monthly</option>
						<option value="year">Yearly</option>
					</select>
				</div>
				<div class="form-group">
					<label for="edit-billingIntervalCount">Interval Count</label>
					<input type="number" id="edit-billingIntervalCount" min="1" bind:value={selectedProductType.billingRecurringIntervalCount} placeholder="e.g., 2 for bi-weekly" />
				</div>
			</div>
		{/if}

		<div class="form-group">
			<label>Filter Fields (Searchable/Filterable)</label>
			<p class="field-description">These fields map to database columns and can be used for filtering products. Limited to 5 of each type.</p>
			<FieldEditor
				fields={schemaFields}
				onFieldsChange={(newFields) => schemaFields = newFields}
			/>
		</div>

		<div class="form-group">
			<label>Custom Fields (Additional Properties)</label>
			<p class="field-description">These fields are stored as JSON and can hold any additional product data. Unlimited fields allowed.</p>
			<CustomFieldEditor
				fields={customSchemaFields}
				onFieldsChange={(newFields) => customSchemaFields = newFields}
			/>
		</div>


		<div class="form-group">
			<ReorderableList
				bind:selectedIds={selectedProductType.pricingTemplates}
				availableItems={pricingTemplates.map(t => ({
					id: t.id,
					name: t.name,
					displayName: t.displayName,
					description: t.description,
					category: t.category,
					priceFormula: t.priceFormula,
					conditionFormula: t.conditionFormula,
					isActive: t.isActive
				}))}
				title="Pricing Templates"
				helpText="Templates will be applied in the order shown. Drag to reorder."
				emptyMessage="No pricing templates selected"
				noItemsMessage="No pricing templates available."
				addButtonText="Add Pricing Template"
				createLink="/admin/extensions/products/pricing"
				createLinkText="Create templates first"
				allowCreateNew={true}
				allowEdit={true}
				on:createNew={handleCreatePricingTemplate}
				on:editItem={handleEditPricingTemplate}
			/>
		</div>

		<div class="form-group">
			<label for="edit-status">Status</label>
			<select id="edit-status" bind:value={selectedProductType.status}>
				<option value="active">Active</option>
				<option value="pending">Pending</option>
				<option value="deleted">Deleted</option>
			</select>
		</div>
	{/if}
	<svelte:fragment slot="footer">
		<Button variant="danger" on:click={() => {
			if (selectedProductType) {
				deleteProductType(selectedProductType.id);
				showEditModal = false;
			}
		}}>Delete</Button>
		<div class="modal-footer-right">
			<Button variant="secondary" on:click={() => { showEditModal = false; schemaFields = []; customSchemaFields = []; }}>Cancel</Button>
			<Button on:click={saveProductType}>Save</Button>
		</div>
	</svelte:fragment>
</Modal>

<ConfirmDialog
	bind:show={showDeleteConfirm}
	title="Delete Product Type"
	message="Are you sure you want to delete this product type? This will affect all products of this type."
	confirmText="Delete"
	variant="danger"
	on:confirm={confirmDeleteProductType}
/>

<!-- Create Pricing Template Modal -->
<PricingTemplateModal
	show={showCreatePricingModal}
	mode="create"
	bind:template={newPricingTemplate}
	variables={variables}
	on:close={() => showCreatePricingModal = false}
	on:save={createPricingTemplate}
	on:createVariable={() => handleCreateVariable('pricing')}
/>

<!-- Edit Pricing Template Modal -->
{#if editingPricingTemplate}
	<PricingTemplateModal
		show={showEditPricingModal}
		mode="edit"
		bind:template={editingPricingTemplate}
		variables={variables}
		on:close={() => {
			showEditPricingModal = false;
			editingPricingTemplate = null;
		}}
		on:save={updatePricingTemplate}
		on:createVariable={() => handleCreateVariable('pricing')}
	/>
{/if}