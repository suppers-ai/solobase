<script lang="ts">
	import { onMount } from 'svelte';
	import { Shield } from 'lucide-svelte';
	import { api, ErrorHandler, authFetch } from '$lib/api';
	import RolesManager from '$lib/components/iam/RolesManager.svelte';
	import PoliciesManager from '$lib/components/iam/PoliciesManager.svelte';
	import UserRolesManager from '$lib/components/iam/UserRolesManager.svelte';
	import PolicyTester from '$lib/components/iam/PolicyTester.svelte';
	import AuditLog from '$lib/components/iam/AuditLog.svelte';
	import PageHeader from '$lib/components/ui/PageHeader.svelte';
	import TabNavigation from '$lib/components/ui/TabNavigation.svelte';
	import LoadingSpinner from '$lib/components/ui/LoadingSpinner.svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';

	interface RoleMetadata {
		disabledFeatures?: string[];
		[key: string]: unknown;
	}

	interface Role {
		name: string;
		displayName?: string;
		description: string;
		type?: string;
		metadata?: RoleMetadata;
	}

	interface Policy {
		id?: string;
		subject: string;
		resource: string;
		action: string;
		effect: 'allow' | 'deny';
	}

	interface UserRole {
		name: string;
		displayName?: string;
	}

	interface User {
		id: string;
		email: string;
		firstName?: string;
		lastName?: string;
		roles?: UserRole[];
	}

	let activeTab = 'roles';
	let roles: Role[] = [];
	let policies: Policy[] = [];
	let users: User[] = [];
	let loading = true;

	let showDeleteRoleConfirm = false;
	let showDeletePolicyConfirm = false;
	let roleToDelete: Role | null = null;
	let policyToDelete: Policy | null = null;

	const tabs = [
		{ id: 'roles', label: 'Roles' },
		{ id: 'policies', label: 'Policies' },
		{ id: 'users', label: 'User Assignments' },
		{ id: 'test', label: 'Test Permissions' },
		{ id: 'audit', label: 'Audit Log' }
	];
	
	async function loadRoles() {
		try {
			const response = await authFetch('/api/admin/iam/roles');
			if (response.ok) {
				roles = await response.json();
			} else {
				console.error('Failed to load roles:', response.statusText);
				roles = [];
			}
		} catch (error) {
			console.error('Failed to load roles:', error);
			roles = [];
		}
	}

	async function loadPolicies() {
		try {
			const response = await authFetch('/api/admin/iam/policies');
			if (response.ok) {
				policies = await response.json();
			} else {
				console.error('Failed to load policies:', response.statusText);
				policies = [];
			}
		} catch (error) {
			console.error('Failed to load policies:', error);
			policies = [];
		}
	}

	async function loadUsers() {
		try {
			const response = await authFetch('/api/admin/iam/users');
			if (response.ok) {
				users = await response.json();
			} else {
				console.error('Failed to load users:', response.statusText);
				users = [];
			}
		} catch (error) {
			console.error('Failed to load users:', error);
			users = [];
		}
	}
	
	async function loadData() {
		loading = true;
		await Promise.all([loadRoles(), loadPolicies(), loadUsers()]);
		loading = false;
	}
	
	async function handleRoleCreated(event: CustomEvent<Role>) {
		const role = event.detail;
		try {
			const response = await authFetch('/api/admin/iam/roles', {
				method: 'POST',
				body: JSON.stringify(role)
			});

			if (response.ok) {
				await loadRoles();
			} else {
				ErrorHandler.handle('Failed to create role');
			}
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}

	function handleRoleDeleted(event: CustomEvent<Role>) {
		roleToDelete = event.detail;
		showDeleteRoleConfirm = true;
	}

	async function confirmDeleteRole() {
		if (!roleToDelete) return;
		showDeleteRoleConfirm = false;

		const response = await authFetch(`/api/admin/iam/roles/${roleToDelete.name}`, {
			method: 'DELETE'
		});

		if (response.ok) {
			await loadRoles();
		} else {
			ErrorHandler.handle('Failed to delete role');
		}
		roleToDelete = null;
	}

	async function handlePolicyCreated(event: CustomEvent<Policy>) {
		const policy = event.detail;
		const response = await authFetch('/api/admin/iam/policies', {
			method: 'POST',
			body: JSON.stringify(policy)
		});

		if (response.ok) {
			await loadPolicies();
		} else {
			ErrorHandler.handle('Failed to create policy');
		}
	}

	function handlePolicyDeleted(event: CustomEvent<Policy>) {
		policyToDelete = event.detail;
		showDeletePolicyConfirm = true;
	}

	async function confirmDeletePolicy() {
		if (!policyToDelete) return;
		showDeletePolicyConfirm = false;

		const response = await authFetch(`/api/admin/iam/policies/${policyToDelete.id}`, {
			method: 'DELETE'
		});

		if (response.ok) {
			await loadPolicies();
		} else {
			ErrorHandler.handle('Failed to delete policy');
		}
		policyToDelete = null;
	}

	async function handleRolesChanged() {
		await loadUsers();
	}
	
	onMount(() => {
		loadData();
	});
</script>

<div class="iam-container">
	<PageHeader
		title="Identity & Access Management"
		subtitle="Manage roles, permissions, and access policies"
		icon={Shield}
	/>
	
	{#if loading}
		<div class="loading">
			<LoadingSpinner size="lg" />
			<p>Loading IAM configuration...</p>
		</div>
	{:else}
		<TabNavigation {tabs} bind:activeTab />
		
		<div class="tab-content">
			{#if activeTab === 'roles'}
				<RolesManager 
					{roles}
					on:create={handleRoleCreated}
					on:delete={handleRoleDeleted}
					on:rolesChanged={loadRoles}
				/>
			{:else if activeTab === 'policies'}
				<PoliciesManager
					{policies}
					on:create={handlePolicyCreated}
					on:delete={handlePolicyDeleted}
					on:policiesChanged={loadPolicies}
				/>
			{:else if activeTab === 'users'}
				<UserRolesManager
					{users}
					{roles}
					on:rolesChanged={handleRolesChanged}
				/>
			{:else if activeTab === 'test'}
				<PolicyTester />
			{:else if activeTab === 'audit'}
				<AuditLog />
			{/if}
		</div>
	{/if}
</div>

<ConfirmDialog
	bind:show={showDeleteRoleConfirm}
	title="Delete Role"
	message="Are you sure you want to delete role {roleToDelete?.displayName || roleToDelete?.name}? This action cannot be undone."
	confirmText="Delete"
	variant="danger"
	on:confirm={confirmDeleteRole}
/>

<ConfirmDialog
	bind:show={showDeletePolicyConfirm}
	title="Delete Policy"
	message="Are you sure you want to delete the policy for {policyToDelete?.subject}?"
	confirmText="Delete"
	variant="danger"
	on:confirm={confirmDeletePolicy}
/>

<style>
	.iam-container {
		padding: 0;
		max-width: 1200px;
		margin: 0 auto;
	}

	.loading {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 3rem;
		color: #666;
		gap: 1rem;
	}

	.loading p {
		margin: 0;
	}

	.tab-content {
		margin-top: 2rem;
		animation: fadeIn 0.3s ease-in;
	}
	
	@keyframes fadeIn {
		from {
			opacity: 0;
			transform: translateY(10px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
	
	@media (max-width: 768px) {
		.iam-container {
			padding: 1rem;
		}
	}
</style>