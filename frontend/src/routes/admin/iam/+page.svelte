<script lang="ts">
	import { onMount } from 'svelte';
	import { api, ErrorHandler, authFetch } from '$lib/api';
	import RolesManager from '$lib/components/iam/RolesManager.svelte';
	import PoliciesManager from '$lib/components/iam/PoliciesManager.svelte';
	import UserRolesManager from '$lib/components/iam/UserRolesManager.svelte';
	import PolicyTester from '$lib/components/iam/PolicyTester.svelte';
	import AuditLog from '$lib/components/iam/AuditLog.svelte';

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

	async function handleRoleDeleted(event: CustomEvent<Role>) {
		const role = event.detail;
		if (!confirm(`Delete role ${role.displayName || role.name}?`)) {
			return;
		}

		const response = await authFetch(`/api/admin/iam/roles/${role.name}`, {
			method: 'DELETE'
		});

		if (response.ok) {
			await loadRoles();
		} else {
			ErrorHandler.handle('Failed to delete role');
		}
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

	async function handlePolicyDeleted(event: CustomEvent<Policy>) {
		const policy = event.detail;
		if (!confirm(`Delete policy for ${policy.subject}?`)) {
			return;
		}

		const response = await authFetch(`/api/admin/iam/policies/${policy.id}`, {
			method: 'DELETE'
		});

		if (response.ok) {
			await loadPolicies();
		} else {
			ErrorHandler.handle('Failed to delete policy');
		}
	}

	async function handleRolesChanged() {
		await loadUsers();
	}
	
	onMount(() => {
		loadData();
	});
</script>

<div class="iam-container">
	<div class="page-header">
		<h1>Identity & Access Management</h1>
		<p class="subtitle">Manage roles, permissions, and access policies</p>
	</div>
	
	{#if loading}
		<div class="loading">Loading IAM configuration...</div>
	{:else}
		<div class="tabs">
			<button 
				class="tab" 
				class:active={activeTab === 'roles'}
				on:click={() => activeTab = 'roles'}
			>
				Roles
			</button>
			<button 
				class="tab" 
				class:active={activeTab === 'policies'}
				on:click={() => activeTab = 'policies'}
			>
				Policies
			</button>
			<button 
				class="tab" 
				class:active={activeTab === 'users'}
				on:click={() => activeTab = 'users'}
			>
				User Assignments
			</button>
			<button 
				class="tab" 
				class:active={activeTab === 'test'}
				on:click={() => activeTab = 'test'}
			>
				Test Permissions
			</button>
			<button 
				class="tab" 
				class:active={activeTab === 'audit'}
				on:click={() => activeTab = 'audit'}
			>
				Audit Log
			</button>
		</div>
		
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

<style>
	.iam-container {
		padding: 2rem;
		max-width: 1200px;
		margin: 0 auto;
	}
	
	.page-header {
		margin-bottom: 2rem;
	}
	
	.page-header h1 {
		margin: 0 0 0.5rem;
		color: #333;
	}
	
	.subtitle {
		margin: 0;
		color: #666;
		font-size: 1.1rem;
	}
	
	.loading {
		text-align: center;
		padding: 3rem;
		color: #666;
	}
	
	.tabs {
		display: flex;
		gap: 1rem;
		margin-bottom: 2rem;
		border-bottom: 2px solid #e0e0e0;
		overflow-x: auto;
	}
	
	.tab {
		padding: 0.75rem 1.5rem;
		background: none;
		border: none;
		color: #666;
		cursor: pointer;
		font-size: 1rem;
		font-weight: 500;
		position: relative;
		white-space: nowrap;
		transition: color 0.3s;
	}
	
	.tab:hover {
		color: #333;
	}
	
	.tab.active {
		color: #4CAF50;
	}
	
	.tab.active::after {
		content: '';
		position: absolute;
		bottom: -2px;
		left: 0;
		right: 0;
		height: 2px;
		background: #4CAF50;
	}
	
	.tab-content {
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
		
		.tabs {
			gap: 0.5rem;
		}
		
		.tab {
			padding: 0.5rem 1rem;
			font-size: 0.9rem;
		}
	}
</style>