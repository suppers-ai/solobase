<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import Modal from '$lib/components/ui/Modal.svelte';

	export let show = false;
	export let item: any = null;
	export let renaming = false;

	const dispatch = createEventDispatcher();

	let newName = '';

	$: if (item) {
		newName = item.objectName || item.name || '';
	}

	function handleRename() {
		if (newName.trim() && newName !== (item?.objectName || item?.name)) {
			dispatch('rename', { newName: newName.trim() });
		}
	}

	function handleClose() {
		newName = '';
		dispatch('close');
	}
</script>

<Modal {show} title="Rename {item?.isFolder ? 'Folder' : 'File'}" maxWidth="450px" on:close={handleClose} closeOnOverlay={!renaming}>
	<form on:submit|preventDefault={handleRename}>
		<div class="modal-form-group">
			<label for="new-name">New Name</label>
			<input
				id="new-name"
				type="text"
				bind:value={newName}
				required
				disabled={renaming}
				pattern="[^/\\]+"
				title="Names cannot contain slashes"
			/>
			<p class="modal-form-hint">
				Current: {item?.objectName || item?.name}
			</p>
		</div>
	</form>

	<svelte:fragment slot="footer">
		<button class="modal-btn modal-btn-secondary" on:click={handleClose} disabled={renaming}>
			Cancel
		</button>
		<button
			class="modal-btn modal-btn-primary"
			on:click={handleRename}
			disabled={!newName.trim() || newName === (item?.objectName || item?.name) || renaming}
		>
			{renaming ? 'Renaming...' : 'Rename'}
		</button>
	</svelte:fragment>
</Modal>
