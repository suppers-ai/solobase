<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { X, Edit2 } from 'lucide-svelte';

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

{#if show && item}
	<div class="modal-overlay" on:click={handleClose}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h3>Rename {item.isFolder ? 'Folder' : 'File'}</h3>
				<button class="icon-button" on:click={handleClose} disabled={renaming}>
					<X size={20} />
				</button>
			</div>

			<form on:submit|preventDefault={handleRename}>
				<div class="modal-body">
					<div class="form-group">
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
						<p class="form-hint">
							Current: {item.objectName || item.name}
						</p>
					</div>
				</div>

				<div class="modal-footer">
					<button type="button" class="btn btn-secondary" on:click={handleClose} disabled={renaming}>
						Cancel
					</button>
					<button
						type="submit"
						class="btn btn-primary"
						disabled={!newName.trim() || newName === (item?.objectName || item?.name) || renaming}
					>
						{renaming ? 'Renaming...' : 'Rename'}
					</button>
				</div>
			</form>
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
		max-width: 450px;
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

	.form-group {
		margin: 0;
	}

	.form-group label {
		display: block;
		margin-bottom: 0.5rem;
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
	}

	.form-group input {
		width: 100%;
		padding: 0.5rem 0.75rem;
		border: 1px solid #d1d5db;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		transition: all 0.2s;
	}

	.form-group input:focus {
		outline: none;
		border-color: #189AB4;
		box-shadow: 0 0 0 3px rgba(24, 154, 180, 0.1);
	}

	.form-group input:disabled {
		background: #f3f4f6;
		cursor: not-allowed;
	}

	.form-hint {
		margin: 0.5rem 0 0 0;
		font-size: 0.75rem;
		color: #6b7280;
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

	.btn-primary {
		background: #189AB4;
		color: white;
	}

	.btn-primary:hover:not(:disabled) {
		background: #157a8f;
	}

	.btn-secondary {
		background: #f3f4f6;
		color: #374151;
	}

	.btn-secondary:hover:not(:disabled) {
		background: #e5e7eb;
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