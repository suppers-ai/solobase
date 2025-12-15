<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import Modal from '$lib/components/ui/Modal.svelte';

	interface RoleMetadata {
		disabledFeatures?: string[];
		[key: string]: unknown;
	}

	interface Role {
		name: string;
		displayName?: string;
		description?: string;
		type?: string;
		metadata?: RoleMetadata;
	}

	export let role: Role | null = null;

	const dispatch = createEventDispatcher();

	let editedRole: Role = {
		name: '',
		displayName: '',
		description: '',
		metadata: {
			disabledFeatures: []
		}
	};

	$: if (role) {
		editedRole = {
			...role,
			metadata: {
				...role.metadata,
				disabledFeatures: role.metadata?.disabledFeatures || []
			}
		};
	}

	$: show = role !== null;

	function handleSubmit() {
		dispatch('save', editedRole);
	}

	function handleClose() {
		dispatch('close');
	}

	function addFeature() {
		const feature = prompt('Enter feature name to disable:');
		const features = editedRole.metadata?.disabledFeatures ?? [];
		if (feature && editedRole.metadata && !features.includes(feature)) {
			editedRole.metadata.disabledFeatures = [...features, feature];
		}
	}

	function removeFeature(feature: string) {
		if (editedRole.metadata) {
			editedRole.metadata.disabledFeatures = (editedRole.metadata.disabledFeatures ?? []).filter(f => f !== feature);
		}
	}
</script>

<Modal {show} title="Edit Role: {role?.displayName || role?.name || ''}" maxWidth="700px" on:close={handleClose}>
	<form on:submit|preventDefault={handleSubmit}>
		<div class="form-grid">
			<div class="modal-form-group">
				<label for="display-name">Display Name</label>
				<input
					id="display-name"
					type="text"
					bind:value={editedRole.displayName}
					placeholder="Enter display name"
					disabled={role?.type === 'system'}
				/>
			</div>

			<div class="modal-form-group full-width">
				<label for="description">Description</label>
				<textarea
					id="description"
					bind:value={editedRole.description}
					placeholder="Enter role description"
					rows="2"
				/>
			</div>

			<div class="section-divider">
				<h3>Additional Settings</h3>
			</div>

			<div class="modal-form-group full-width">
				<label>
					Disabled Features
					<button type="button" class="btn-small" on:click={addFeature}>+ Add</button>
				</label>
				<div class="features-list">
					{#if (editedRole.metadata?.disabledFeatures?.length ?? 0) > 0}
						{#each editedRole.metadata?.disabledFeatures ?? [] as feature}
							<span class="feature-tag">
								{feature}
								<button type="button" class="remove-btn" on:click={() => removeFeature(feature)}>×</button>
							</span>
						{/each}
					{:else}
						<span class="no-features">No features disabled</span>
					{/if}
				</div>
			</div>
		</div>
	</form>

	<svelte:fragment slot="footer">
		<button class="modal-btn modal-btn-secondary" on:click={handleClose}>Cancel</button>
		<button class="modal-btn modal-btn-primary" on:click={handleSubmit}>Save Changes</button>
	</svelte:fragment>
</Modal>

<style>
	.form-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
	}

	.full-width {
		grid-column: 1 / -1;
	}

	.section-divider {
		grid-column: 1 / -1;
		margin: 1rem 0 0.5rem;
		padding-top: 1rem;
		border-top: 1px solid #e0e0e0;
	}

	.section-divider h3 {
		margin: 0;
		font-size: 1.1rem;
		color: #555;
	}

	.features-list {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		padding: 0.5rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		min-height: 40px;
	}

	.feature-tag {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		background: #e3f2fd;
		color: #1976d2;
		padding: 0.25rem 0.5rem;
		border-radius: 15px;
		font-size: 0.85rem;
	}

	.remove-btn {
		background: none;
		border: none;
		color: #1976d2;
		cursor: pointer;
		font-size: 1.2rem;
		line-height: 1;
		padding: 0;
	}

	.remove-btn:hover {
		color: #0d47a1;
	}

	.no-features {
		color: #999;
		font-style: italic;
		font-size: 0.9rem;
	}

	.btn-small {
		padding: 0.2rem 0.5rem;
		font-size: 0.85rem;
		background: #189AB4;
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
	}

	.btn-small:hover {
		background: #157a8f;
	}

	label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
</style>
