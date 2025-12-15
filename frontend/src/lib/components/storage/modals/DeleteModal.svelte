<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { AlertTriangle } from 'lucide-svelte';
	import Modal from '$lib/components/ui/Modal.svelte';

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

<Modal {show} title="Delete {itemText}" maxWidth="500px" on:close={handleClose} closeOnOverlay={!deleting}>
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
					<li>{item.objectName || item.name}</li>
				{/each}
			</ul>
		</div>
	{/if}

	<svelte:fragment slot="footer">
		<button class="modal-btn modal-btn-secondary" on:click={handleClose} disabled={deleting}>
			Cancel
		</button>
		<button class="modal-btn modal-btn-danger" on:click={handleDelete} disabled={deleting}>
			{deleting ? 'Deleting...' : `Delete ${itemCount} ${itemText}`}
		</button>
	</svelte:fragment>
</Modal>

<style>
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
</style>
