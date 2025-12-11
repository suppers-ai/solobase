<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { X, AlertTriangle } from 'lucide-svelte';

	export let show = false;
	export let items: any[] = [];
	export let deleting = false;

	const dispatch = createEventDispatcher();

	$: itemCount = items.length;
	$: itemText = itemCount === 1 ? 'item' : 'items';

	function handleDelete() {
		dispatch('confirm');
	}

	function handleClose() {
		dispatch('close');
	}
</script>

{#if show}
	<div class="modal-overlay" on:click={handleClose}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h3>Delete {itemText}</h3>
				<button class="icon-button" on:click={handleClose} disabled={deleting}>
					<X size={20} />
				</button>
			</div>

			<div class="modal-body">
				<div class="warning">
					<AlertTriangle size={24} color="#ef4444" />
					<div>
						<p class="warning-title">
							Are you sure you want to delete {itemCount} {itemText}?
						</p>
						<p class="warning-text">
							This action cannot be undone.
						</p>
					</div>
				</div>

				{#if items.length <= 10}
					<div class="item-list">
						<p class="list-title">Items to be deleted:</p>
						<ul>
							{#each items as item}
								<li>{item.object_name || item.name}</li>
							{/each}
						</ul>
					</div>
				{/if}
			</div>

			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={handleClose} disabled={deleting}>
					Cancel
				</button>
				<button class="btn btn-danger" on:click={handleDelete} disabled={deleting}>
					{deleting ? 'Deleting...' : `Delete ${itemCount} ${itemText}`}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal {
		background: white;
		border-radius: 0.5rem;
		width: 90%;
		max-width: 500px;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-bottom: 1px solid #e5e7eb;
	}

	.modal-header h3 {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 600;
	}

	.modal-body {
		padding: 1.5rem;
	}

	.warning {
		display: flex;
		gap: 1rem;
		padding: 1rem;
		background: #fef2f2;
		border: 1px solid #fee2e2;
		border-radius: 0.375rem;
		margin-bottom: 1rem;
	}

	.warning-title {
		margin: 0 0 0.25rem 0;
		font-size: 0.875rem;
		font-weight: 600;
		color: #991b1b;
	}

	.warning-text {
		margin: 0;
		font-size: 0.875rem;
		color: #7f1d1d;
	}

	.item-list {
		margin-top: 1rem;
	}

	.list-title {
		margin: 0 0 0.5rem 0;
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
	}

	.item-list ul {
		margin: 0;
		padding-left: 1.5rem;
		max-height: 200px;
		overflow-y: auto;
	}

	.item-list li {
		font-size: 0.875rem;
		color: #6b7280;
		padding: 0.25rem 0;
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid #e5e7eb;
	}

	.btn {
		padding: 0.5rem 1rem;
		border: none;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-secondary {
		background: #f3f4f6;
		color: #374151;
	}

	.btn-secondary:hover:not(:disabled) {
		background: #e5e7eb;
	}

	.btn-danger {
		background: #ef4444;
		color: white;
	}

	.btn-danger:hover:not(:disabled) {
		background: #dc2626;
	}

	.icon-button {
		padding: 0.25rem;
		background: transparent;
		border: none;
		border-radius: 0.25rem;
		cursor: pointer;
		color: #6b7280;
		transition: all 0.2s;
	}

	.icon-button:hover:not(:disabled) {
		background: #f3f4f6;
		color: #374151;
	}

	.icon-button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>