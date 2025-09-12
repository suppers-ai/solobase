<script>
	import { createEventDispatcher } from 'svelte';
	import RoleCard from './RoleCard.svelte';
	import CreateRoleModal from './CreateRoleModal.svelte';
	import EditRoleModal from './EditRoleModal.svelte';
	
	export let roles = [];
	
	const dispatch = createEventDispatcher();
	
	let showCreateModal = false;
	let showEditModal = false;
	let selectedRole = null;
	
	async function handleCreateRole(event) {
		const role = event.detail;
		const response = await fetch('/api/iam/roles', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				'Authorization': `Bearer ${localStorage.getItem('token')}`
			},
			body: JSON.stringify(role)
		});
		
		if (response.ok) {
			dispatch('rolesChanged');
			showCreateModal = false;
		} else {
			alert('Failed to create role');
		}
	}
	
	async function handleDeleteRole(event) {
		const role = event.detail;
		if (!confirm(`Are you sure you want to delete role "${role.display_name || role.name}"?`)) {
			return;
		}
		
		const response = await fetch(`/api/iam/roles/${role.name}`, {
			method: 'DELETE',
			headers: {
				'Authorization': `Bearer ${localStorage.getItem('token')}`
			}
		});
		
		if (response.ok) {
			dispatch('rolesChanged');
		} else {
			const error = await response.text();
			alert(`Failed to delete role: ${error}`);
		}
	}
	
	function handleEditRole(event) {
		selectedRole = event.detail;
		showEditModal = true;
	}
	
	async function handleSaveRole(event) {
		const role = event.detail;
		const response = await fetch(`/api/iam/roles/${role.name}`, {
			method: 'PUT',
			headers: {
				'Content-Type': 'application/json',
				'Authorization': `Bearer ${localStorage.getItem('token')}`
			},
			body: JSON.stringify(role)
		});
		
		if (response.ok) {
			dispatch('rolesChanged');
			showEditModal = false;
			selectedRole = null;
		} else {
			alert('Failed to update role');
		}
	}
</script>

<div class="section">
	<div class="section-header">
		<h2>Roles</h2>
		<button class="btn btn-primary" on:click={() => showCreateModal = true}>
			Create Role
		</button>
	</div>
	
	<div class="roles-grid">
		{#each roles as role}
			<RoleCard 
				{role} 
				on:delete={handleDeleteRole}
				on:edit={handleEditRole}
			/>
		{/each}
	</div>
</div>

{#if showCreateModal}
	<CreateRoleModal
		on:create={handleCreateRole}
		on:close={() => showCreateModal = false}
	/>
{/if}

{#if showEditModal}
	<EditRoleModal
		role={selectedRole}
		on:save={handleSaveRole}
		on:close={() => {
			showEditModal = false;
			selectedRole = null;
		}}
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
	
	.roles-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
		gap: 1.5rem;
	}
	
	.btn {
		padding: 0.5rem 1rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		background: white;
		color: #333;
		cursor: pointer;
		transition: all 0.3s;
	}
	
	.btn:hover {
		background: #f5f5f5;
	}
	
	.btn-primary {
		background: #4CAF50;
		color: white;
		border-color: #4CAF50;
	}
	
	.btn-primary:hover {
		background: #45a049;
	}
</style>