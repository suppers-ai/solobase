<script lang="ts">
	import { onMount } from 'svelte';
	import { 
		Building2, Plus, Edit2, Trash2, Search, Filter,
		CheckCircle, XCircle, Settings
	} from 'lucide-svelte';
	import { api } from '$lib/api';
	import { requireAdmin } from '$lib/utils/auth';
	import IconPicker from '$lib/components/IconPicker.svelte';
	import FieldEditor from '$lib/components/FieldEditor.svelte';
	import { getIconComponent } from '$lib/utils/icons';

	interface FieldConstraints {
		required?: boolean;
		min?: number;
		max?: number;
		min_length?: number;
		max_length?: number;
		pattern?: string;
		options?: string[];
		default?: any;
		placeholder?: string;
	}

	interface FieldDefinition {
		id: string;  // e.g., "filter_text_1", "filter_numeric_1"
		name: string;
		type: string;  // numeric, text, boolean, enum, location
		required?: boolean;
		description?: string;
		constraints: FieldConstraints;
	}

	interface EntityType {
		id: string;
		name: string;
		display_name: string;
		description: string;
		icon?: string;
		fields?: FieldDefinition[];
		status: 'active' | 'pending' | 'deleted';
		created_at: string;
		updated_at: string;
	}

	let groupTypes: EntityType[] = [];
	let loading = true;
	let searchQuery = '';
	let showCreateModal = false;
	let showEditModal = false;
	let selectedEntityType: EntityType | null = null;
	
	// Form data for new group type
	let newEntityType: Partial<EntityType> = {
		name: '',
		display_name: '',
		description: '',
		icon: 'building',
		status: 'active',
		fields: []
	};

	$: filteredEntityTypes = groupTypes.filter(groupType => {
		const matchesSearch = groupType.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
			groupType.display_name.toLowerCase().includes(searchQuery.toLowerCase()) ||
			groupType.description?.toLowerCase().includes(searchQuery.toLowerCase());
		return matchesSearch;
	});

	onMount(async () => {
		if (!requireAdmin()) return;
		await loadEntityTypes();
	});

	async function loadEntityTypes() {
		try {
			loading = true;
			const response = await api.get('/products/group-types');
			groupTypes = response || [];
		} catch (error) {
			console.error('Failed to load group types:', error);
			groupTypes = [];
		} finally {
			loading = false;
		}
	}
	
	async function createEntityType() {
		try {
			// Ensure fields is a valid array
			if (!Array.isArray(newEntityType.fields)) {
				newEntityType.fields = [];
			}

			const result = await api.post('/products/group-types', newEntityType);
			if (result) {
				// Reset form
				newEntityType = {
					name: '',
					display_name: '',
					description: '',
					icon: 'building',
					status: 'active',
					fields: []
				};
				showCreateModal = false;
				// Reload group types
				await loadEntityTypes();
			}
		} catch (error) {
			console.error('Failed to create group type:', error);
			alert('Failed to create group type');
		}
	}
	
	async function saveEntityType() {
		if (!selectedEntityType) return;
		
		// Update fields from schema editor
		selectedEntityType.fields = schemaFields.filter(field => field.name).map(field => ({
			id: field.id,
			name: field.name,
			type: field.type,
			required: field.required || false,
			description: field.description || '',
			constraints: field.constraints || {}
		}));
		
		try {
			const result = await api.put(`/products/group-types/${selectedEntityType.id}`, selectedEntityType);
			if (result) {
				showEditModal = false;
				selectedEntityType = null;
				schemaFields = [];
				// Reload group types
				await loadEntityTypes();
			}
		} catch (error) {
			console.error('Failed to save group type:', error);
			alert('Failed to save group type');
		}
	}
	
	async function deleteEntityType(id: string) {
		if (!confirm('Are you sure you want to delete this group type? This will affect all groups of this type.')) return;
		
		try {
			await api.delete(`/products/group-types/${id}`);
			// Reload group types
			await loadEntityTypes();
		} catch (error) {
			console.error('Failed to delete group type:', error);
			alert('Failed to delete group type');
		}
	}

	// Schema editor state
	let schemaFields: FieldDefinition[] = [];
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

	function openEditModal(groupType: EntityType) {
		selectedEntityType = { ...groupType };
		if (Array.isArray(groupType.fields)) {
			schemaFields = groupType.fields.map((field: any) => ({
				id: field.id || '',
				name: field.name || '',
				type: field.type || 'text',
				required: field.required || false,
				description: field.description || '',
				constraints: field.constraints || {}
			}));
		} else {
			schemaFields = [];
		}
		showFieldTypeSelector = false;
		showEditModal = true;
	}

	function addSchemaField(type: string) {
		const id = getNextFilterId(type, schemaFields);
		if (!id) {
			alert(`Maximum of 5 ${type} fields reached`);
			return;
		}
		
		schemaFields = [...schemaFields, {
			id: id,
			name: '',
			type: type,
			required: false,
			description: '',
			constraints: {}
		}];
		showFieldTypeSelector = false;
	}

	function removeSchemaField(index: number) {
		schemaFields = schemaFields.filter((_, i) => i !== index);
	}
</script>

<div class="page-container">
	<!-- Header -->
	<div class="page-header">
		<a href="/admin/extensions/products" class="back-button">
			<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
				<polyline points="15 18 9 12 15 6"></polyline>
			</svg>
		</a>
		<div class="header-content">
			<div class="header-left">
				<div class="header-title">
					<Building2 size={24} />
					<h1>Group Types</h1>
				</div>
				<p class="header-subtitle">Define types of groups that can own products (e.g., Store, Company, Team)</p>
			</div>
			<div class="header-actions">
				<button class="btn btn-primary" on:click={() => showCreateModal = true}>
					<Plus size={16} />
					New Group Type
				</button>
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
						placeholder="Search group types..."
						bind:value={searchQuery}
					/>
				</div>
			</div>
			<div class="toolbar-right">
				<button class="btn-icon">
					<Filter size={16} />
				</button>
			</div>
		</div>

		<!-- Group Types Grid -->
		{#if loading}
			<div class="loading-container">
				<div class="loading loading-spinner loading-lg text-cyan-600"></div>
			</div>
		{:else if filteredEntityTypes.length === 0}
			<div class="empty-state">
				<Building2 size={48} class="text-gray-400" />
				<h3>No group types found</h3>
				<p>Create your first group type to organize your business structure</p>
				<button class="btn btn-primary mt-4" on:click={() => showCreateModal = true}>
					<Plus size={16} />
					Create Group Type
				</button>
			</div>
		{:else}
			<div class="group-grid">
				{#each filteredEntityTypes as groupType}
					<div class="group-card" on:click={() => openEditModal(groupType)} role="button" tabindex="0" on:keypress={(e) => e.key === 'Enter' && openEditModal(groupType)}>
						<div class="group-header">
							<div class="group-icon">
								<svelte:component this={getIconComponent(groupType.icon)} size={24} />
							</div>
							<span class="status-badge status-{groupType.status}">
								{#if groupType.status === 'active'}
									<CheckCircle size={12} />
									Active
								{:else if groupType.status === 'pending'}
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
							<h3 class="group-name">{groupType.display_name}</h3>
							<code class="group-code">{groupType.name}</code>
							<p class="group-description">{groupType.description}</p>
							
							{#if groupType.fields && groupType.fields.length > 0}
								<div class="group-fields">
									<p class="fields-label">Custom Fields:</p>
									<div class="fields-list">
										{#each groupType.fields as field}
											<span class="field-badge" title="{field.type}{field.constraints?.required ? ' (required)' : ''}">{field.label || field.name}</span>
										{/each}
									</div>
								</div>
							{/if}
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
		position: relative;
	}

	.back-button {
		position: absolute;
		top: 1.5rem;
		left: 1.5rem;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 36px;
		height: 36px;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		background: white;
		color: #6b7280;
		text-decoration: none;
		transition: all 0.2s;
	}

	.back-button:hover {
		background: #f9fafb;
		color: #111827;
	}

	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-left: 48px;
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
	
	.btn-danger {
		background: #ef4444;
		color: white;
	}
	
	.btn-danger:hover {
		background: #dc2626;
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
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-top: 1px solid #e5e7eb;
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
</style>

<!-- Create Group Type Modal -->
{#if showCreateModal}
	<div class="modal-overlay" on:click={() => showCreateModal = false}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h2>Create New Group Type</h2>
				<button class="btn-icon" on:click={() => showCreateModal = false}>
					×
				</button>
			</div>
			<div class="modal-body">
				<div class="form-row">
					<div class="form-group">
						<label for="name">Group Type Name</label>
						<input type="text" id="name" bind:value={newEntityType.name} 
							placeholder="e.g., store, company, team" />
					</div>
					<div class="form-group">
						<label for="display_name">Display Name</label>
						<input type="text" id="display_name" bind:value={newEntityType.display_name} 
							placeholder="e.g., Store, Company, Team" />
					</div>
				</div>
				<div class="form-group">
					<label for="description">Description</label>
					<textarea id="description" bind:value={newEntityType.description} rows="2" 
						placeholder="Describe what this group type represents"></textarea>
				</div>
				<div class="form-group">
					<label for="icon">Icon</label>
					<IconPicker bind:value={newEntityType.icon} placeholder="Choose an icon" />
				</div>
				<div class="form-group">
					<label for="status">Status</label>
					<select id="status" bind:value={newEntityType.status}>
						<option value="active">Active</option>
						<option value="pending">Pending</option>
						<option value="deleted">Deleted</option>
					</select>
				</div>
			</div>
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={() => showCreateModal = false}>Cancel</button>
				<button class="btn btn-primary" on:click={createEntityType}>Create Group Type</button>
			</div>
		</div>
	</div>
{/if}

<!-- Edit Group Type Modal -->
{#if showEditModal && selectedEntityType}
	<div class="modal-overlay" on:click={() => showEditModal = false}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h2>Edit Group Type</h2>
				<button class="btn-icon" on:click={() => showEditModal = false}>
					×
				</button>
			</div>
			<div class="modal-body">
				<div class="form-row">
					<div class="form-group">
						<label for="edit-name">Group Type Name</label>
						<input type="text" id="edit-name" bind:value={selectedEntityType.name} />
					</div>
					<div class="form-group">
						<label for="edit-display_name">Display Name</label>
						<input type="text" id="edit-display_name" bind:value={selectedEntityType.display_name} />
					</div>
				</div>
				<div class="form-group">
					<label for="edit-description">Description</label>
					<textarea id="edit-description" bind:value={selectedEntityType.description} rows="2"></textarea>
				</div>
				<div class="form-group">
					<label for="edit-icon">Icon</label>
					<IconPicker bind:value={selectedEntityType.icon} placeholder="Choose an icon" />
				</div>
				
				<div class="form-group">
					<label>Custom Fields Definition</label>
					<FieldEditor 
						fields={schemaFields} 
						onFieldsChange={(newFields) => schemaFields = newFields} 
					/>
				</div>
				
				<div class="form-group">
					<label for="edit-status">Status</label>
					<select id="edit-status" bind:value={selectedEntityType.status}>
						<option value="active">Active</option>
						<option value="pending">Pending</option>
						<option value="deleted">Deleted</option>
					</select>
				</div>
			</div>
			<div class="modal-footer">
				<button class="btn btn-danger" on:click={() => { 
					if (confirm('Are you sure you want to delete this group type? This will affect all groups of this type.')) {
						deleteEntityType(selectedEntityType.id);
						showEditModal = false;
					}
				}}>Delete</button>
				<div class="modal-footer-right">
					<button class="btn btn-secondary" on:click={() => { showEditModal = false; schemaFields = []; }}>Cancel</button>
					<button class="btn btn-primary" on:click={saveEntityType}>Save</button>
				</div>
			</div>
		</div>
	</div>
{/if}