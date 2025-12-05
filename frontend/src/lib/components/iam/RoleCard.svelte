<script>
	import { createEventDispatcher } from 'svelte';
	
	export let role;
	
	const dispatch = createEventDispatcher();
	
	function handleDelete() {
		dispatch('delete', role);
	}
	
	function handleEdit() {
		dispatch('edit', role);
	}
</script>

<div class="role-card" class:system={role.type === 'system'}>
	<div class="role-header">
		<h3>{role.display_name || role.name}</h3>
		{#if role.type === 'system'}
			<span class="badge">System</span>
		{/if}
	</div>
	<p class="role-description">{role.description}</p>
	
	{#if role.metadata?.disabled_features?.length > 0}
		<div class="role-metadata">
			<h4>Disabled Features</h4>
			<ul>
				<li>{role.metadata.disabled_features.join(', ')}</li>
			</ul>
		</div>
	{/if}
	
	<div class="role-actions">
		<button class="btn btn-small" on:click={handleEdit}>
			Edit
		</button>
		{#if role.type !== 'system'}
			<button class="btn btn-small btn-danger" on:click={handleDelete}>
				Delete
			</button>
		{/if}
	</div>
</div>

<style>
	.role-card {
		border: 1px solid #e0e0e0;
		border-radius: 8px;
		padding: 1.5rem;
		background: #f9f9f9;
	}
	
	.role-card.system {
		background: #f0f7ff;
		border-color: #b3d9ff;
	}
	
	.role-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.5rem;
	}
	
	.role-header h3 {
		margin: 0;
		color: #333;
	}
	
	.role-description {
		color: #666;
		margin: 0.5rem 0 1rem;
	}
	
	.role-metadata {
		margin-top: 1rem;
		padding-top: 1rem;
		border-top: 1px solid #e0e0e0;
	}
	
	.role-metadata h4 {
		margin: 0 0 0.5rem;
		font-size: 0.9rem;
		color: #666;
	}
	
	.role-metadata ul {
		list-style: none;
		padding: 0;
		margin: 0;
	}
	
	.role-metadata li {
		font-size: 0.85rem;
		color: #666;
		padding: 0.25rem 0;
	}
	
	.role-actions {
		margin-top: 1rem;
		padding-top: 1rem;
		border-top: 1px solid #e0e0e0;
		display: flex;
		gap: 0.5rem;
	}
	
	.badge {
		display: inline-block;
		padding: 0.25rem 0.5rem;
		border-radius: 3px;
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
		background: #e3f2fd;
		color: #1976d2;
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
	
	.btn-small {
		padding: 0.25rem 0.5rem;
		font-size: 0.85rem;
	}
	
	.btn-danger {
		background: #f44336;
		color: white;
		border-color: #f44336;
	}
	
	.btn-danger:hover {
		background: #da190b;
	}
</style>