<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import Modal from '$lib/components/ui/Modal.svelte';

	export let show = false;

	const dispatch = createEventDispatcher();

	let bucketName = '';
	let isPublic = false;
	let creating = false;

	function handleCreate() {
		if (bucketName.trim()) {
			dispatch('create', { name: bucketName.trim(), public: isPublic });
		}
	}

	function handleClose() {
		bucketName = '';
		isPublic = false;
		dispatch('close');
	}
</script>

<Modal {show} title="Create New Bucket" maxWidth="500px" on:close={handleClose}>
	<form on:submit|preventDefault={handleCreate}>
		<div class="modal-form-group">
			<label for="bucket-name">Bucket Name</label>
			<input
				id="bucket-name"
				type="text"
				bind:value={bucketName}
				placeholder="my-bucket"
				required
				disabled={creating}
				pattern="[a-z0-9][a-z0-9-]*[a-z0-9]"
				title="Bucket names must be lowercase, contain only letters, numbers and hyphens"
			/>
			<p class="modal-form-hint">
				Bucket names must be unique, lowercase, and contain only letters, numbers, and hyphens.
			</p>
		</div>

		<div class="modal-form-group">
			<label class="checkbox-label">
				<input
					type="checkbox"
					bind:checked={isPublic}
					disabled={creating}
				/>
				<span>Make bucket public</span>
			</label>
			<p class="modal-form-hint">
				Public buckets allow anyone to read files without authentication.
			</p>
		</div>
	</form>

	<svelte:fragment slot="footer">
		<button type="button" class="modal-btn modal-btn-secondary" on:click={handleClose} disabled={creating}>
			Cancel
		</button>
		<button
			type="button"
			class="modal-btn modal-btn-primary"
			disabled={!bucketName.trim() || creating}
			on:click={handleCreate}
		>
			{creating ? 'Creating...' : 'Create Bucket'}
		</button>
	</svelte:fragment>
</Modal>

<style>
	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
	}

	.checkbox-label input[type="checkbox"] {
		cursor: pointer;
	}

	.checkbox-label span {
		font-size: 0.875rem;
		color: #374151;
	}
</style>
