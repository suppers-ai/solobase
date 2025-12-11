<script>
	import { createEventDispatcher } from 'svelte';
	
	export let user;
	
	const dispatch = createEventDispatcher();
	
	function handleRemoveRole(roleName) {
		dispatch('removeRole', { userId: user.id, roleName });
	}
	
	function handleAssignRole() {
		dispatch('assignRole');
	}
</script>

<tr>
	<td>{user.firstName || ''} {user.lastName || ''}</td>
	<td>{user.email}</td>
	<td>
		<div class="user-roles">
			{#if user.roles && user.roles.length > 0}
				{#each user.roles as role}
					<span class="role-tag">
						{role.display_name || role.name}
						<button class="remove-role" on:click={() => handleRemoveRole(role.name)}>×</button>
					</span>
				{/each}
			{:else}
				<span class="no-roles">No roles assigned</span>
			{/if}
		</div>
	</td>
	<td>
		<button class="btn btn-small" on:click={handleAssignRole}>
			Assign Role
		</button>
	</td>
</tr>

<style>
	td {
		padding: 0.75rem;
		border-bottom: 1px solid #e0e0e0;
	}
	
	.user-roles {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}
	
	.role-tag {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		background: #e3f2fd;
		color: #1976d2;
		padding: 0.25rem 0.5rem;
		border-radius: 15px;
		font-size: 0.85rem;
	}
	
	.remove-role {
		background: none;
		border: none;
		color: #1976d2;
		cursor: pointer;
		font-size: 1.2rem;
		line-height: 1;
		padding: 0;
	}
	
	.remove-role:hover {
		color: #0d47a1;
	}
	
	.no-roles {
		color: #999;
		font-style: italic;
		font-size: 0.9rem;
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
	
	.btn-small {
		padding: 0.25rem 0.5rem;
		font-size: 0.85rem;
	}
</style>