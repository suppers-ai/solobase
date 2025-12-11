<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import UserRoleRow from './UserRoleRow.svelte';
	import AssignRoleModal from './AssignRoleModal.svelte';
	import { ErrorHandler, authFetch } from '$lib/api';

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

	interface Role {
		name: string;
		displayName?: string;
		description: string;
	}

	interface RoleAssignEvent {
		userId: string;
		roleName: string;
	}

	export let users: User[] = [];
	export let roles: Role[] = [];

	const dispatch = createEventDispatcher();

	let selectedUser: User | null = null;
	let showAssignModal = false;

	function handleAssignRole(user: User) {
		selectedUser = user;
		showAssignModal = true;
	}

	async function handleRoleAssigned(event: CustomEvent<RoleAssignEvent>) {
		const { userId, roleName } = event.detail;
		const response = await authFetch(`/api/admin/iam/users/${userId}/roles`, {
			method: 'POST',
			body: JSON.stringify({ role: roleName })
		});

		if (response.ok) {
			dispatch('rolesChanged', userId);
			showAssignModal = false;
		} else {
			ErrorHandler.handle('Failed to assign role');
		}
	}

	async function handleRoleRemoved(event: CustomEvent<RoleAssignEvent>) {
		const { userId, roleName } = event.detail;
		if (!confirm(`Remove role ${roleName} from user?`)) {
			return;
		}

		const response = await authFetch(`/api/admin/iam/users/${userId}/roles/${roleName}`, {
			method: 'DELETE'
		});

		if (response.ok) {
			dispatch('rolesChanged', userId);
		} else {
			ErrorHandler.handle('Failed to remove role');
		}
	}
</script>

<div class="section">
	<div class="section-header">
		<h2>User Role Assignments</h2>
	</div>
	
	<table class="users-table">
		<thead>
			<tr>
				<th>User</th>
				<th>Email</th>
				<th>Roles</th>
				<th>Actions</th>
			</tr>
		</thead>
		<tbody>
			{#each users as user}
				<UserRoleRow 
					{user}
					on:assignRole={() => handleAssignRole(user)}
					on:removeRole={handleRoleRemoved}
				/>
			{/each}
			{#if users.length === 0}
				<tr>
					<td colspan="4" class="empty">No users found</td>
				</tr>
			{/if}
		</tbody>
	</table>
</div>

{#if showAssignModal && selectedUser}
	<AssignRoleModal
		user={selectedUser}
		{roles}
		on:assign={handleRoleAssigned}
		on:close={() => showAssignModal = false}
	/>
{/if}

<style>
	.section {
		background: white;
		border-radius: 8px;
		padding: 2rem;
		box-shadow: 0 2px 4px rgba(0,0,0,0.1);
	}
	
	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 2rem;
	}
	
	.section-header h2 {
		margin: 0;
		color: #333;
	}
	
	.users-table {
		width: 100%;
		border-collapse: collapse;
	}
	
	th {
		text-align: left;
		padding: 0.75rem;
		background: #f5f5f5;
		border-bottom: 2px solid #e0e0e0;
		font-weight: 600;
	}
	
	.empty {
		text-align: center;
		color: #999;
		font-style: italic;
		padding: 2rem;
	}
</style>