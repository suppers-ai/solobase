<script lang="ts">
	import { onMount } from 'svelte';
	import {
		Building2, Plus, Edit2, Trash2, Settings,
		Users, Store, Briefcase, Home, Globe, MapPin,
		Phone, Mail, Calendar, MoreVertical, ExternalLink
	} from 'lucide-svelte';
	import SearchInput from '$lib/components/SearchInput.svelte';
	import { api, ErrorHandler } from '$lib/api';
	import { authStore } from '$lib/stores/auth';
	import { goto } from '$app/navigation';
	import LoadingSpinner from '$lib/components/ui/LoadingSpinner.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import StatusBadge from '$lib/components/ui/StatusBadge.svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import Modal from '$lib/components/ui/Modal.svelte';
	import PageHeader from '$lib/components/ui/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';

	interface Group {
		id: string;
		groupTemplateId: string;
		userId: string;
		name: string;
		displayName: string;
		description?: string;
		settings?: any;
		customFields?: any;
		active: boolean;
		createdAt: string;
		updatedAt: string;
		// Joined data
		groupTemplate?: any;
		productsCount?: number;
	}

	let groups: Group[] = [];
	let groupTypes: any[] = [];
	let loading = true;
	let searchQuery = '';
	let selectedType = 'all';
	let showCreateModal = false;
	let showEditModal = false;
	let showDeleteConfirm = false;
	let selectedGroup: Group | null = null;
	let groupToDelete: Group | null = null;
	
	// Form data for new group
	let newGroup: Partial<Group> = {
		name: '',
		displayName: '',
		description: '',
		groupTemplateId: '',
		active: true,
		customFields: {}
	};

	// Dynamic fields based on group type
	let dynamicFields: any = {};

	$: filteredGroups = groups.filter(group => {
		const matchesSearch = group.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
			group.displayName.toLowerCase().includes(searchQuery.toLowerCase()) ||
			group.description?.toLowerCase().includes(searchQuery.toLowerCase());
		const matchesType = selectedType === 'all' || group.groupTemplateId === selectedType;
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
			const typesRes = await api.get<any[]>('/ext/products/group-types');
			groupTypes = typesRes || [];

			// Load user's groups
			const groupsRes = await api.get<Group[]>('/ext/products/groups');
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
		const selectedType = groupTypes.find(t => t.id === newGroup.groupTemplateId);
		if (selectedType && selectedType.filterFieldsSchema) {
			// Initialize dynamic fields based on schema
			dynamicFields = {};
			Object.entries(selectedType.filterFieldsSchema).forEach(([key, field]: [string, any]) => {
				dynamicFields[key] = field.default || '';
			});
		} else {
			dynamicFields = {};
		}
	}
	
	async function createGroup() {
		try {
			// Add dynamic fields to customFields
			if (Object.keys(dynamicFields).length > 0) {
				newGroup.customFields = { ...newGroup.customFields, ...dynamicFields };
			}

			const result = await api.post('/ext/products/groups', newGroup);
			if (result) {
				// Reset form
				newGroup = {
					name: '',
					displayName: '',
					description: '',
					groupTemplateId: '',
					active: true,
					customFields: {}
				};
				dynamicFields = {};
				showCreateModal = false;
				// Reload groups
				await loadData();
			}
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}
	
	async function editGroup(group: Group) {
		selectedGroup = { ...group };
		// Load dynamic fields if group type has schema
		const groupType = getGroupTypeInfo(group.groupTemplateId);
		if (groupType && groupType.filterFieldsSchema) {
			dynamicFields = {};
			Object.entries(groupType.filterFieldsSchema).forEach(([key, field]: [string, any]) => {
				dynamicFields[key] = group.customFields?.[key] || field.default || '';
			});
		}
		showEditModal = true;
	}
	
	async function updateGroup() {
		if (!selectedGroup) return;

		try {
			// Add dynamic fields to customFields
			if (Object.keys(dynamicFields).length > 0) {
				selectedGroup.customFields = { ...selectedGroup.customFields, ...dynamicFields };
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
			ErrorHandler.handle(error);
		}
	}
	
	function deleteGroup(group: Group) {
		groupToDelete = group;
		showDeleteConfirm = true;
	}

	async function confirmDeleteGroup() {
		if (!groupToDelete) return;
		showDeleteConfirm = false;

		try {
			await api.delete(`/user/groups/${groupToDelete.id}`);
			await loadData();
		} catch (error) {
			ErrorHandler.handle(error);
		}
		groupToDelete = null;
	}

	function navigateToProducts(group: Group) {
		goto(`/profile/groups/${group.id}/products`);
	}
</script>

<div class="page-container">
	<!-- Header -->
	<PageHeader
		title="My Groups"
		subtitle="Manage your business groups and organizations"
		icon={Building2}
		variant="card"
	>
		<svelte:fragment slot="actions">
			<Button icon={Plus} on:click={() => showCreateModal = true}>
				New Group
			</Button>
		</svelte:fragment>
	</PageHeader>

	<!-- Main Content -->
	<div class="content-card">
		<!-- Toolbar -->
		<div class="toolbar">
			<div class="toolbar-left">
				<SearchInput bind:value={searchQuery} placeholder="Search groups..." maxWidth="320px" />
				<select class="filter-select" bind:value={selectedType}>
					<option value="all">All Types</option>
					{#each groupTypes as type}
						<option value={type.id}>{type.displayName}</option>
					{/each}
				</select>
			</div>
		</div>

		<!-- Groups Grid -->
		{#if loading}
			<div class="loading-container">
				<LoadingSpinner size="lg" />
			</div>
		{:else if filteredGroups.length === 0}
			<EmptyState icon={Building2} title="No groups found" message="Create your first group to start managing products">
				<Button icon={Plus} on:click={() => showCreateModal = true}>
					Create Group
				</Button>
			</EmptyState>
		{:else}
			<div class="groups-grid">
				{#each filteredGroups as group}
					{@const groupType = getGroupTypeInfo(group.groupTemplateId)}
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
								<button class="btn-icon btn-icon-danger" on:click={() => deleteGroup(group)} title="Delete">
									<Trash2 size={16} />
								</button>
							</div>
						</div>
						<div class="group-body">
							<h3 class="group-name">{group.displayName}</h3>
							<p class="group-type">{groupType?.displayName || 'Unknown Type'}</p>
							{#if group.description}
								<p class="group-description">{group.description}</p>
							{/if}
							
							{#if group.customFields && Object.keys(group.customFields).length > 0}
								<div class="group-metadata">
									{#if group.customFields.address}
										<div class="metadata-item">
											<MapPin size={14} />
											{group.customFields.address}
										</div>
									{/if}
									{#if group.customFields.phone}
										<div class="metadata-item">
											<Phone size={14} />
											{group.customFields.phone}
										</div>
									{/if}
									{#if group.customFields.email}
										<div class="metadata-item">
											<Mail size={14} />
											{group.customFields.email}
										</div>
									{/if}
								</div>
							{/if}
							
							<div class="group-footer">
								<div class="group-stats">
									<span class="stat-item">
										{group.productsCount || 0} products
									</span>
									<StatusBadge status={group.active ? 'Active' : 'Inactive'} size="sm" />
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

<ConfirmDialog
	bind:show={showDeleteConfirm}
	title="Delete Group"
	message="Are you sure you want to delete this group? All associated products will also be deleted."
	confirmText="Delete"
	variant="danger"
	on:confirm={confirmDeleteGroup}
/>

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

	.filter-select {
		padding: 0.5rem 0.75rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		background: white;
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
<Modal show={showCreateModal} title="Create New Group" on:close={() => showCreateModal = false}>
	<div class="form-group">
		<label for="group_type">Group Type</label>
		<select id="group_type" bind:value={newGroup.groupTemplateId} on:change={onGroupTypeChange}>
			<option value="">Select Group Type</option>
			{#each groupTypes as type}
				<option value={type.id}>{type.displayName}</option>
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
			<label for="displayName">Display Name</label>
			<input type="text" id="displayName" bind:value={newGroup.displayName}
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
				{@const fieldSchema = groupTypes.find(t => t.id === newGroup.groupTemplateId)?.filterFieldsSchema?.[key]}
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
		<label for="active">Status</label>
		<select id="active" bind:value={newGroup.active}>
			<option value={true}>Active</option>
			<option value={false}>Inactive</option>
		</select>
	</div>

	<svelte:fragment slot="footer">
		<Button variant="secondary" on:click={() => showCreateModal = false}>Cancel</Button>
		<Button on:click={createGroup}>Create Group</Button>
	</svelte:fragment>
</Modal>

<!-- Edit Group Modal -->
<Modal show={showEditModal && !!selectedGroup} title="Edit Group" on:close={() => showEditModal = false}>
	{#if selectedGroup}
		<div class="form-row">
			<div class="form-group">
				<label for="edit-name">Group Name</label>
				<input type="text" id="edit-name" bind:value={selectedGroup.name} />
			</div>
			<div class="form-group">
				<label for="edit-displayName">Display Name</label>
				<input type="text" id="edit-displayName" bind:value={selectedGroup.displayName} />
			</div>
		</div>

		<div class="form-group">
			<label for="edit-description">Description</label>
			<textarea id="edit-description" bind:value={selectedGroup.description} rows="2"></textarea>
		</div>

		{#if Object.keys(dynamicFields).length > 0}
			{@const groupType = getGroupTypeInfo(selectedGroup.groupTemplateId)}
			<div class="dynamic-fields">
				<h4>Additional Information</h4>
				{#each Object.entries(dynamicFields) as [key, value]}
					{@const fieldSchema = groupType?.filterFieldsSchema?.[key]}
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
			<label for="edit-active">Status</label>
			<select id="edit-active" bind:value={selectedGroup.active}>
				<option value={true}>Active</option>
				<option value={false}>Inactive</option>
			</select>
		</div>
	{/if}

	<svelte:fragment slot="footer">
		<Button variant="secondary" on:click={() => showEditModal = false}>Cancel</Button>
		<Button on:click={updateGroup}>Update Group</Button>
	</svelte:fragment>
</Modal>