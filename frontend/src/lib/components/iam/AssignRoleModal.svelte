<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	interface User {
		id: string;
		email: string;
	}

	interface Role {
		name: string;
		displayName?: string;
		description: string;
	}

	export let user: User;
	export let roles: Role[] = [];

	const dispatch = createEventDispatcher();

	function handleAssign(roleName: string) {
		dispatch('assign', { userId: user.id, roleName });
	}

	function handleClose() {
		dispatch('close');
	}
</script>

<div class="modal-overlay" on:click={handleClose}>
	<div class="modal" on:click|stopPropagation>
		<h2>Assign Role to {user.email}</h2>
		
		<div class="role-options">
			{#each roles as role}
				<button class="role-option" on:click={() => handleAssign(role.name)}>
					<h4>{role.displayName || role.name}</h4>
					<p>{role.description}</p>
				</button>
			{/each}
		</div>
		
		<div class="modal-actions">
			<button class="btn" on:click={handleClose}>Cancel</button>
		</div>
	</div>
</div>

<style>
	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0,0,0,0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}
	
	.modal {
		background: white;
		border-radius: 8px;
		padding: 2rem;
		max-width: 600px;
		width: 90%;
		max-height: 90vh;
		overflow-y: auto;
	}
	
	.modal h2 {
		margin-top: 0;
	}
	
	.role-options {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		margin: 1.5rem 0;
	}
	
	.role-option {
		text-align: left;
		padding: 1rem;
		border: 1px solid #e0e0e0;
		border-radius: 8px;
		background: white;
		cursor: pointer;
		transition: all 0.3s;
	}
	
	.role-option:hover {
		background: #f5f5f5;
		border-color: #4CAF50;
	}
	
	.role-option h4 {
		margin: 0 0 0.5rem;
		color: #333;
	}
	
	.role-option p {
		margin: 0;
		color: #666;
		font-size: 0.9rem;
	}
	
	.modal-actions {
		display: flex;
		justify-content: flex-end;
		gap: 1rem;
		margin-top: 2rem;
		padding-top: 1rem;
		border-top: 1px solid #e0e0e0;
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
</style>