<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import Modal from '$lib/components/ui/Modal.svelte';

	export let show = false;

	const dispatch = createEventDispatcher();

	let role = {
		name: '',
		displayName: '',
		description: '',
		metadata: {
			storageQuota: 1073741824, // 1GB default
			bandwidthQuota: 10737418240, // 10GB default
			maxUploadSize: 104857600, // 100MB default
			maxRequestsPerMin: 100,
			sessionTimeout: 3600,
			disabledFeatures: [] as string[]
		}
	};

	function handleSubmit() {
		dispatch('create', role);
	}

	function handleClose() {
		role = {
			name: '',
			displayName: '',
			description: '',
			metadata: {
				storageQuota: 1073741824,
				bandwidthQuota: 10737418240,
				maxUploadSize: 104857600,
				maxRequestsPerMin: 100,
				sessionTimeout: 3600,
				disabledFeatures: []
			}
		};
		dispatch('close');
	}
</script>

<Modal {show} title="Create New Role" maxWidth="600px" on:close={handleClose}>
	<form on:submit|preventDefault={handleSubmit}>
		<div class="modal-form-group">
			<label for="role-name">Name (lowercase, no spaces)</label>
			<input id="role-name" type="text" bind:value={role.name} placeholder="editor" />
		</div>

		<div class="modal-form-group">
			<label for="role-display">Display Name</label>
			<input id="role-display" type="text" bind:value={role.displayName} placeholder="Content Editor" />
		</div>

		<div class="modal-form-group">
			<label for="role-desc">Description</label>
			<textarea id="role-desc" bind:value={role.description} placeholder="Can create and edit content"></textarea>
		</div>

		<h3 class="section-title">Quotas & Limits</h3>

		<div class="modal-form-group">
			<label for="storage-quota">Storage Quota (bytes)</label>
			<input id="storage-quota" type="number" bind:value={role.metadata.storageQuota} />
		</div>

		<div class="modal-form-group">
			<label for="bandwidth-quota">Bandwidth Quota (bytes)</label>
			<input id="bandwidth-quota" type="number" bind:value={role.metadata.bandwidthQuota} />
		</div>

		<div class="modal-form-group">
			<label for="upload-size">Max Upload Size (bytes)</label>
			<input id="upload-size" type="number" bind:value={role.metadata.maxUploadSize} />
		</div>

		<div class="modal-form-group">
			<label for="rate-limit">Max Requests per Minute</label>
			<input id="rate-limit" type="number" bind:value={role.metadata.maxRequestsPerMin} />
		</div>
	</form>

	<svelte:fragment slot="footer">
		<button class="modal-btn modal-btn-secondary" on:click={handleClose}>Cancel</button>
		<button class="modal-btn modal-btn-primary" on:click={handleSubmit}>Create</button>
	</svelte:fragment>
</Modal>

<style>
	.section-title {
		margin-top: 1.5rem;
		margin-bottom: 1rem;
		color: #666;
		font-size: 1.1rem;
	}

	textarea {
		min-height: 100px;
		resize: vertical;
	}
</style>
