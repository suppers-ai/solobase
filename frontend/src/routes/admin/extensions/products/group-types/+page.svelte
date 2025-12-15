<script lang="ts">
	import { onMount } from 'svelte';
	import {
		Building2, Plus, Edit2, Trash2, Filter,
		CheckCircle, XCircle, Settings
	} from 'lucide-svelte';
	import SearchInput from '$lib/components/SearchInput.svelte';
	import { api, ErrorHandler } from '$lib/api';
	import { requireAdmin } from '$lib/utils/auth';
	import IconPicker from '$lib/components/IconPicker.svelte';
	import FieldEditor from '$lib/components/FieldEditor.svelte';
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

	interface EntityType {
		id: string;
		name: string;
		displayName: string;
		description: string;
		icon?: string;
		fields?: FieldDefinition[];
		status: 'active' | 'pending' | 'deleted';
		createdAt: string;
		updatedAt: string;
	}

	let groupTypes: EntityType[] = [];
	let loading = true;
	let searchQuery = '';
	let showCreateModal = false;
	let showEditModal = false;
	let showDeleteConfirm = false;
	let selectedEntityType: EntityType | null = null;
	let entityTypeToDelete: string | null = null;
	
	// Form data for new group type
	let newEntityType: Partial<EntityType> = {
		name: '',
		displayName: '',
		description: '',
		icon: 'building',
		status: 'active',
		fields: []
	};

	$: filteredEntityTypes = groupTypes.filter(groupType => {
		const matchesSearch = groupType.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
			groupType.displayName.toLowerCase().includes(searchQuery.toLowerCase()) ||
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
			const response = await api.get<EntityType[]>('/admin/ext/products/group-types');
			groupTypes = Array.isArray(response) ? response : [];
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

			const result = await api.post('/admin/ext/products/group-types', newEntityType);
			if (result) {
				// Reset form
				newEntityType = {
					name: '',
					displayName: '',
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
			ErrorHandler.handle(error);
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
			const result = await api.put(`/admin/ext/products/group-types/${selectedEntityType.id}`, selectedEntityType);
			if (result) {
				showEditModal = false;
				selectedEntityType = null;
				schemaFields = [];
				// Reload group types
				await loadEntityTypes();
			}
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}
	
	function deleteEntityType(id: string) {
		entityTypeToDelete = id;
		showDeleteConfirm = true;
	}

	async function confirmDeleteEntityType() {
		if (!entityTypeToDelete) return;
		showDeleteConfirm = false;

		try {
			await api.delete(`/admin/ext/products/group-types/${entityTypeToDelete}`);
			// Reload group types
			await loadEntityTypes();
		} catch (error) {
			ErrorHandler.handle(error);
		}
		entityTypeToDelete = null;
	}

	// Schema editor state
	let schemaFields: EditorField[] = [];
	let showFieldTypeSelector = false;
	
	// Track used filter IDs
	function getUsedFilterIds(fields: EditorField[]): Map<string, number> {
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
	function getNextFilterId(type: string, fields: EditorField[]): string | null {
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
	function isTypeAvailable(type: string, fields: EditorField[]): boolean {
		return getNextFilterId(type, fields) !== null;
	}

	// Count fields by type
	function countFieldsByType(type: string, fields: EditorField[]): number {
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
			schemaFields = groupType.fields.map((field: FieldDefinition) => ({
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
		showFieldTypeSelector = false;
		showEditModal = true;
	}

	function addSchemaField(type: string) {
		const id = getNextFilterId(type, schemaFields);
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
		title="Group Types"
		subtitle="Define types of groups that can own products (e.g., Store, Company, Team)"
		icon={Building2}
		backHref="/admin/extensions/products"
		variant="card"
	>
		<svelte:fragment slot="actions">
			<Button icon={Plus} on:click={() => showCreateModal = true}>
				New Group Type
			</Button>
		</svelte:fragment>
	</PageHeader>

	<!-- Main Content -->
	<div class="content-card">
		<!-- Toolbar -->
		<div class="toolbar">
			<div class="toolbar-left">
				<SearchInput bind:value={searchQuery} placeholder="Search group types..." maxWidth="320px" />
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
			<EmptyState
				icon={Building2}
				title="No group types found"
				message="Create your first group type to organize your business structure"
			>
				<Button icon={Plus} on:click={() => showCreateModal = true}>
					Create Group Type
				</Button>
			</EmptyState>
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
							<h3 class="group-name">{groupType.displayName}</h3>
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
</style>

<!-- Create Group Type Modal -->
<Modal show={showCreateModal} title="Create New Group Type" maxWidth="600px" on:close={() => showCreateModal = false}>
	<div class="form-row">
		<div class="form-group">
			<label for="name">Group Type Name</label>
			<input type="text" id="name" bind:value={newEntityType.name}
				placeholder="e.g., store, company, team" />
		</div>
		<div class="form-group">
			<label for="display_name">Display Name</label>
			<input type="text" id="displayName" bind:value={newEntityType.displayName}
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
	<svelte:fragment slot="footer">
		<Button variant="secondary" on:click={() => showCreateModal = false}>Cancel</Button>
		<Button on:click={createEntityType}>Create Group Type</Button>
	</svelte:fragment>
</Modal>

<!-- Edit Group Type Modal -->
<Modal show={showEditModal && !!selectedEntityType} title="Edit Group Type" maxWidth="700px" on:close={() => { showEditModal = false; schemaFields = []; }}>
	{#if selectedEntityType}
		<div class="form-row">
			<div class="form-group">
				<label for="edit-name">Group Type Name</label>
				<input type="text" id="edit-name" bind:value={selectedEntityType.name} />
			</div>
			<div class="form-group">
				<label for="edit-display_name">Display Name</label>
				<input type="text" id="edit-displayName" bind:value={selectedEntityType.displayName} />
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
	{/if}
	<svelte:fragment slot="footer">
		<Button variant="danger" on:click={() => {
			if (selectedEntityType) {
				deleteEntityType(selectedEntityType.id);
				showEditModal = false;
			}
		}}>Delete</Button>
		<div class="modal-footer-right">
			<Button variant="secondary" on:click={() => { showEditModal = false; schemaFields = []; }}>Cancel</Button>
			<Button on:click={saveEntityType}>Save</Button>
		</div>
	</svelte:fragment>
</Modal>

<ConfirmDialog
	bind:show={showDeleteConfirm}
	title="Delete Group Type"
	message="Are you sure you want to delete this group type? This will affect all groups of this type."
	confirmText="Delete"
	variant="danger"
	on:confirm={confirmDeleteEntityType}
/>