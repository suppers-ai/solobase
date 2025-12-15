<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import Modal from '$lib/components/ui/Modal.svelte';

	interface User {
		id: string;
		email: string;
	}

	interface Role {
		name: string;
		displayName?: string;
		description: string;
	}

	export let show = false;
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

<Modal {show} title="Assign Role to {user?.email || ''}" maxWidth="600px" on:close={handleClose}>
	<div class="role-options">
		{#each roles as role}
			<button class="role-option" on:click={() => handleAssign(role.name)}>
				<h4>{role.displayName || role.name}</h4>
				<p>{role.description}</p>
			</button>
		{/each}
	</div>

	<svelte:fragment slot="footer">
		<button class="modal-btn modal-btn-secondary" on:click={handleClose}>Cancel</button>
	</svelte:fragment>
</Modal>

<style>
	.role-options {
		display: flex;
		flex-direction: column;
		gap: 1rem;
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
		border-color: #189AB4;
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
</style>
