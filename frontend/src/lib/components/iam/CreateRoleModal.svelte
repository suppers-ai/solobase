<script>
	import { createEventDispatcher } from 'svelte';
	
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
			disabledFeatures: []
		}
	};
	
	function handleSubmit() {
		dispatch('create', role);
	}
	
	function handleClose() {
		dispatch('close');
	}
</script>

<div class="modal-overlay" on:click={handleClose}>
	<div class="modal" on:click|stopPropagation>
		<h2>Create New Role</h2>
		
		<div class="form-group">
			<label for="role-name">Name (lowercase, no spaces)</label>
			<input id="role-name" type="text" bind:value={role.name} placeholder="editor" />
		</div>
		
		<div class="form-group">
			<label for="role-display">Display Name</label>
			<input id="role-display" type="text" bind:value={role.displayName} placeholder="Content Editor" />
		</div>
		
		<div class="form-group">
			<label for="role-desc">Description</label>
			<textarea id="role-desc" bind:value={role.description} placeholder="Can create and edit content"></textarea>
		</div>
		
		<h3>Quotas & Limits</h3>
		
		<div class="form-group">
			<label for="storage-quota">Storage Quota (bytes)</label>
			<input id="storage-quota" type="number" bind:value={role.metadata.storageQuota} />
		</div>

		<div class="form-group">
			<label for="bandwidth-quota">Bandwidth Quota (bytes)</label>
			<input id="bandwidth-quota" type="number" bind:value={role.metadata.bandwidthQuota} />
		</div>

		<div class="form-group">
			<label for="upload-size">Max Upload Size (bytes)</label>
			<input id="upload-size" type="number" bind:value={role.metadata.maxUploadSize} />
		</div>

		<div class="form-group">
			<label for="rate-limit">Max Requests per Minute</label>
			<input id="rate-limit" type="number" bind:value={role.metadata.maxRequestsPerMin} />
		</div>
		
		<div class="modal-actions">
			<button class="btn" on:click={handleClose}>Cancel</button>
			<button class="btn btn-primary" on:click={handleSubmit}>Create</button>
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
	
	.modal h3 {
		margin-top: 2rem;
		margin-bottom: 1rem;
		color: #666;
		font-size: 1.1rem;
	}
	
	.form-group {
		margin-bottom: 1.5rem;
	}
	
	.form-group label {
		display: block;
		margin-bottom: 0.5rem;
		font-weight: 500;
		color: #333;
	}
	
	.form-group input,
	.form-group textarea {
		width: 100%;
		padding: 0.5rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		font-size: 1rem;
	}
	
	.form-group textarea {
		min-height: 100px;
		resize: vertical;
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
	
	.btn-primary {
		background: #4CAF50;
		color: white;
		border-color: #4CAF50;
	}
	
	.btn-primary:hover {
		background: #45a049;
	}
</style>