<script lang="ts">
	import { onMount } from 'svelte';
	import { 
		Building2, Plus, Edit2, Trash2, Search, Settings,
		Users, Store, Briefcase, Home, Globe, MapPin,
		Phone, Mail, Calendar, MoreVertical, ExternalLink
	} from 'lucide-svelte';
	import { api } from '$lib/api';
	import { authStore } from '$lib/stores/auth';
	import { goto } from '$app/navigation';

	interface Group {
		id: string;
		group_type_id: string;
		user_id: string;
		name: string;
		display_name: string;
		description?: string;
		settings?: any;
		metadata?: any;
		is_active: boolean;
		created_at: string;
		updated_at: string;
		// Joined data
		group_type?: any;
		products_count?: number;
	}

	let groups: Group[] = [];
	let groupTypes: any[] = [];
	let loading = true;
	let searchQuery = '';
	let selectedType = 'all';
	let showCreateModal = false;
	let showEditModal = false;
	let selectedGroup: Group | null = null;
	
	// Form data for new group
	let newGroup: Partial<Group> = {
		name: '',
		display_name: '',
		description: '',
		group_type_id: '',
		is_active: true,
		metadata: {}
	};

	// Dynamic fields based on group type
	let dynamicFields: any = {};

	$: filteredGroups = groups.filter(group => {
		const matchesSearch = group.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
			group.display_name.toLowerCase().includes(searchQuery.toLowerCase()) ||
			group.description?.toLowerCase().includes(searchQuery.toLowerCase());
		const matchesType = selectedType === 'all' || group.group_type_id === selectedType;
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
			// Load group types first
			const typesRes = await api.get('/products/group-types');
			groupTypes = typesRes || [];
			
			// Load user's groups
			const groupsRes = await api.get('/user/groups');
			groups = groupsRes || [];
		} catch (error) {
			console.error('Failed to load data:', error);
			groups = [];
			groupTypes = [];
		} finally {
			loading = false;
		}
	}

	function getGroupTypeInfo(typeId: string) {
		return groupTypes.find(t => t.id === typeId);
	}

	function getIconForType(type: any) {
		if (!type) return Building2;
		switch (type.icon) {
			case 'store': return Store;
			case 'users': return Users;
			case 'briefcase': return Briefcase;
			case 'home': return Home;
			case 'globe': return Globe;
			default: return Building2;
		}
	}

	function onGroupTypeChange() {
		const selectedType = groupTypes.find(t => t.id === newGroup.group_type_id);
		if (selectedType && selectedType.fields_schema) {
			// Initialize dynamic fields based on schema
			dynamicFields = {};
			Object.entries(selectedType.fields_schema).forEach(([key, field]: [string, any]) => {
				dynamicFields[key] = field.default || '';
			});
		} else {
			dynamicFields = {};
		}
	}
	
	async function createGroup() {
		try {
			// Add dynamic fields to metadata
			if (Object.keys(dynamicFields).length > 0) {
				newGroup.metadata = { ...newGroup.metadata, ...dynamicFields };
			}
			
			const result = await api.post('/user/groups', newGroup);
			if (result) {
				// Reset form
				newGroup = {
					name: '',
					display_name: '',
					description: '',
					group_type_id: '',
					is_active: true,
					metadata: {}
				};
				dynamicFields = {};
				showCreateModal = false;
				// Reload groups
				await loadData();
			}
		} catch (error) {
			console.error('Failed to create group:', error);
			alert('Failed to create group');
		}
	}
	
	async function editGroup(group: Group) {
		selectedGroup = { ...group };
		// Load dynamic fields if group type has schema
		const groupType = getGroupTypeInfo(group.group_type_id);
		if (groupType && groupType.fields_schema) {
			dynamicFields = {};
			Object.entries(groupType.fields_schema).forEach(([key, field]: [string, any]) => {
				dynamicFields[key] = group.metadata?.[key] || field.default || '';
			});
		}
		showEditModal = true;
	}
	
	async function updateGroup() {
		if (!selectedGroup) return;
		
		try {
			// Add dynamic fields to metadata
			if (Object.keys(dynamicFields).length > 0) {
				selectedGroup.metadata = { ...selectedGroup.metadata, ...dynamicFields };
			}
			
			const result = await api.put(`/user/groups/${selectedGroup.id}`, selectedGroup);
			if (result) {
				showEditModal = false;
				selectedGroup = null;
				dynamicFields = {};
				// Reload groups
				await loadData();
			}
		} catch (error) {
			console.error('Failed to update group:', error);
			alert('Failed to update group');
		}
	}
	
	async function deleteGroup(id: string) {
		if (!confirm('Are you sure you want to delete this group? All associated products will also be deleted.')) return;
		
		try {
			await api.delete(`/user/groups/${id}`);
			// Reload groups
			await loadData();
		} catch (error) {
			console.error('Failed to delete group:', error);
			alert('Failed to delete group');
		}
	}

	function navigateToProducts(group: Group) {
		goto(`/profile/groups/${group.id}/products`);
	}
</script>

<div class="page-container">
	<!-- Header -->
	<div class="page-header">
		<div class="header-content">
			<div class="header-left">
				<div class="header-title">
					<Building2 size={24} />
					<h1>My Groups</h1>
				</div>
				<p class="header-subtitle">Manage your business groups and organizations</p>
			</div>
			<div class="header-actions">
				<button class="btn btn-primary" on:click={() => showCreateModal = true}>
					<Plus size={16} />
					New Group
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
						placeholder="Search groups..."
						bind:value={searchQuery}
					/>
				</div>
				<select class="filter-select" bind:value={selectedType}>
					<option value="all">All Types</option>
					{#each groupTypes as type}
						<option value={type.id}>{type.display_name}</option>
					{/each}
				</select>
			</div>
		</div>

		<!-- Groups Grid -->
		{#if loading}
			<div class="loading-container">
				<div class="loading loading-spinner loading-lg text-cyan-600"></div>
			</div>
		{:else if filteredGroups.length === 0}
			<div class="empty-state">
				<Building2 size={48} class="text-gray-400" />
				<h3>No groups found</h3>
				<p>Create your first group to start managing products</p>
				<button class="btn btn-primary mt-4" on:click={() => showCreateModal = true}>
					<Plus size={16} />
					Create Group
				</button>
			</div>
		{:else}
			<div class="groups-grid">
				{#each filteredGroups as group}
					{@const groupType = getGroupTypeInfo(group.group_type_id)}
					<div class="group-card">
						<div class="group-header">
							<div class="group-icon">
								<svelte:component this={getIconForType(groupType)} size={24} />
							</div>
							<div class="group-actions">
								<button class="btn-icon" on:click={() => navigateToProducts(group)} title="Products">
									<ExternalLink size={16} />
								</button>
								<button class="btn-icon" on:click={() => editGroup(group)} title="Edit">
									<Edit2 size={16} />
								</button>
								<button class="btn-icon btn-icon-danger" on:click={() => deleteGroup(group.id)} title="Delete">
									<Trash2 size={16} />
								</button>
							</div>
						</div>
						<div class="group-body">
							<h3 class="group-name">{group.display_name}</h3>
							<p class="group-type">{groupType?.display_name || 'Unknown Type'}</p>
							{#if group.description}
								<p class="group-description">{group.description}</p>
							{/if}
							
							{#if group.metadata && Object.keys(group.metadata).length > 0}
								<div class="group-metadata">
									{#if group.metadata.address}
										<div class="metadata-item">
											<MapPin size={14} />
											{group.metadata.address}
										</div>
									{/if}
									{#if group.metadata.phone}
										<div class="metadata-item">
											<Phone size={14} />
											{group.metadata.phone}
										</div>
									{/if}
									{#if group.metadata.email}
										<div class="metadata-item">
											<Mail size={14} />
											{group.metadata.email}
										</div>
									{/if}
								</div>
							{/if}
							
							<div class="group-footer">
								<div class="group-stats">
									<span class="stat-item">
										{group.products_count || 0} products
									</span>
									<span class="status-badge {group.is_active ? 'status-active' : 'status-inactive'}">
										{group.is_active ? 'Active' : 'Inactive'}
									</span>
								</div>
								<button class="btn-link" on:click={() => navigateToProducts(group)}>
									View Products →
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

	.groups-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
		gap: 1.5rem;
		padding: 1.5rem;
	}

	.group-card {
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		overflow: hidden;
		transition: all 0.2s;
	}

	.group-card:hover {
		box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
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

	.group-body {
		padding: 1rem;
	}

	.group-name {
		font-size: 1.125rem;
		font-weight: 600;
		color: #111827;
		margin: 0 0 0.25rem 0;
	}

	.group-type {
		font-size: 0.875rem;
		color: #6b7280;
		margin: 0 0 0.75rem 0;
	}

	.group-description {
		font-size: 0.875rem;
		color: #6b7280;
		margin: 0 0 1rem 0;
	}

	.group-metadata {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin-bottom: 1rem;
	}

	.metadata-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.875rem;
		color: #6b7280;
	}

	.group-footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding-top: 1rem;
		border-top: 1px solid #f3f4f6;
	}

	.group-stats {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.stat-item {
		font-size: 0.875rem;
		color: #6b7280;
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
		max-width: 600px;
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
</style>

<!-- Create Group Modal -->
{#if showCreateModal}
	<div class="modal-overlay" on:click={() => showCreateModal = false}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h2>Create New Group</h2>
				<button class="btn-icon" on:click={() => showCreateModal = false}>
					×
				</button>
			</div>
			<div class="modal-body">
				<div class="form-group">
					<label for="group_type">Group Type</label>
					<select id="group_type" bind:value={newGroup.group_type_id} on:change={onGroupTypeChange}>
						<option value="">Select Group Type</option>
						{#each groupTypes as type}
							<option value={type.id}>{type.display_name}</option>
						{/each}
					</select>
				</div>
				
				<div class="form-row">
					<div class="form-group">
						<label for="name">Group Name</label>
						<input type="text" id="name" bind:value={newGroup.name} 
							placeholder="e.g., my_store, main_office" />
					</div>
					<div class="form-group">
						<label for="display_name">Display Name</label>
						<input type="text" id="display_name" bind:value={newGroup.display_name} 
							placeholder="e.g., My Store, Main Office" />
					</div>
				</div>
				
				<div class="form-group">
					<label for="description">Description</label>
					<textarea id="description" bind:value={newGroup.description} rows="2" 
						placeholder="Describe this group"></textarea>
				</div>
				
				{#if Object.keys(dynamicFields).length > 0}
					<div class="dynamic-fields">
						<h4>Additional Information</h4>
						{#each Object.entries(dynamicFields) as [key, value]}
							{@const fieldSchema = groupTypes.find(t => t.id === newGroup.group_type_id)?.fields_schema?.[key]}
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
				
				<div class="form-group">
					<label for="is_active">Status</label>
					<select id="is_active" bind:value={newGroup.is_active}>
						<option value={true}>Active</option>
						<option value={false}>Inactive</option>
					</select>
				</div>
			</div>
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={() => showCreateModal = false}>Cancel</button>
				<button class="btn btn-primary" on:click={createGroup}>Create Group</button>
			</div>
		</div>
	</div>
{/if}

<!-- Edit Group Modal -->
{#if showEditModal && selectedGroup}
	<div class="modal-overlay" on:click={() => showEditModal = false}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h2>Edit Group</h2>
				<button class="btn-icon" on:click={() => showEditModal = false}>
					×
				</button>
			</div>
			<div class="modal-body">
				<div class="form-row">
					<div class="form-group">
						<label for="edit-name">Group Name</label>
						<input type="text" id="edit-name" bind:value={selectedGroup.name} />
					</div>
					<div class="form-group">
						<label for="edit-display_name">Display Name</label>
						<input type="text" id="edit-display_name" bind:value={selectedGroup.display_name} />
					</div>
				</div>
				
				<div class="form-group">
					<label for="edit-description">Description</label>
					<textarea id="edit-description" bind:value={selectedGroup.description} rows="2"></textarea>
				</div>
				
				{#if Object.keys(dynamicFields).length > 0}
					{@const groupType = getGroupTypeInfo(selectedGroup.group_type_id)}
					<div class="dynamic-fields">
						<h4>Additional Information</h4>
						{#each Object.entries(dynamicFields) as [key, value]}
							{@const fieldSchema = groupType?.fields_schema?.[key]}
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
				
				<div class="form-group">
					<label for="edit-is_active">Status</label>
					<select id="edit-is_active" bind:value={selectedGroup.is_active}>
						<option value={true}>Active</option>
						<option value={false}>Inactive</option>
					</select>
				</div>
			</div>
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={() => showEditModal = false}>Cancel</button>
				<button class="btn btn-primary" on:click={updateGroup}>Update Group</button>
			</div>
		</div>
	</div>
{/if}