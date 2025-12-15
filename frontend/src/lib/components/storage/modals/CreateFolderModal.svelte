<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import Modal from '$lib/components/ui/Modal.svelte';

	export let show = false;
	export let currentPath = '';

	const dispatch = createEventDispatcher();

	let folderName = '';
	let creating = false;

	function handleCreate() {
		if (folderName.trim()) {
			dispatch('create', { name: folderName.trim() });
		}
	}

	function handleClose() {
		folderName = '';
		dispatch('close');
	}
</script>

<Modal {show} title="Create New Folder" maxWidth="450px" on:close={handleClose}>
	<form on:submit|preventDefault={handleCreate}>
		<p class="path-info">
			Creating folder in: <strong>{currentPath || 'root'}</strong>
		</p>

		<div class="modal-form-group">
			<label for="folder-name">Folder Name</label>
			<input
				id="folder-name"
				type="text"
				bind:value={folderName}
				placeholder="New Folder"
				required
				disabled={creating}
				pattern="[^/\\]+"
				title="Folder names cannot contain slashes"
			/>
		</div>
	</form>

	<svelte:fragment slot="footer">
		<button type="button" class="modal-btn modal-btn-secondary" on:click={handleClose} disabled={creating}>
			Cancel
		</button>
		<button
			type="button"
			class="modal-btn modal-btn-primary"
			disabled={!folderName.trim() || creating}
			on:click={handleCreate}
		>
			{creating ? 'Creating...' : 'Create Folder'}
		</button>
	</svelte:fragment>
</Modal>

<style>
	.path-info {
		margin: 0 0 1rem 0;
		color: #6b7280;
		font-size: 0.875rem;
	}
</style>
