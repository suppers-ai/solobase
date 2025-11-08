<script>
	import { createEventDispatcher } from 'svelte';
	
	export let policies = [];
	
	const dispatch = createEventDispatcher();
	
	function handleDelete(policy) {
		dispatch('delete', policy);
	}
</script>

<table class="policies-table">
	<thead>
		<tr>
			<th>Subject</th>
			<th>Resource</th>
			<th>Action</th>
			<th>Effect</th>
			<th>Actions</th>
		</tr>
	</thead>
	<tbody>
		{#each policies as policy}
			<tr>
				<td>{policy.subject}</td>
				<td><code>{policy.resource}</code></td>
				<td><code>{policy.action}</code></td>
				<td>
					<span class="badge" class:allow={policy.effect === 'allow'} class:deny={policy.effect === 'deny'}>
						{policy.effect}
					</span>
				</td>
				<td>
					<div class="actions">
						<button class="btn btn-small" on:click={() => dispatch('edit', policy)}>
							Edit
						</button>
						<button class="btn btn-small btn-danger" on:click={() => handleDelete(policy)}>
							Delete
						</button>
					</div>
				</td>
			</tr>
		{/each}
		{#if policies.length === 0}
			<tr>
				<td colspan="5" class="empty">No policies defined</td>
			</tr>
		{/if}
	</tbody>
</table>

<style>
	.policies-table {
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
	
	td {
		padding: 0.75rem;
		border-bottom: 1px solid #e0e0e0;
	}
	
	.empty {
		text-align: center;
		color: #999;
		font-style: italic;
	}
	
	code {
		background: #f5f5f5;
		padding: 0.2rem 0.4rem;
		border-radius: 3px;
		font-family: monospace;
	}
	
	.badge {
		display: inline-block;
		padding: 0.25rem 0.5rem;
		border-radius: 3px;
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
	}
	
	.badge.allow {
		background: #d4edda;
		color: #155724;
	}
	
	.badge.deny {
		background: #f8d7da;
		color: #721c24;
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
	
	.actions {
		display: flex;
		gap: 0.5rem;
	}
</style>