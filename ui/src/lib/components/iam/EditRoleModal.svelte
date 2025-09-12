<script>
	import { createEventDispatcher } from 'svelte';
	
	export let role = null;
	
	const dispatch = createEventDispatcher();
	
	let editedRole = {};
	
	$: if (role) {
		editedRole = {
			...role,
			metadata: {
				...role.metadata,
				disabled_features: role.metadata?.disabled_features || []
			}
		};
	}
	
	function handleSubmit() {
		dispatch('save', editedRole);
	}
	
	function handleClose() {
		dispatch('close');
	}
	
	function addFeature() {
		const feature = prompt('Enter feature name to disable:');
		if (feature && !editedRole.metadata.disabled_features.includes(feature)) {
			editedRole.metadata.disabled_features = [...editedRole.metadata.disabled_features, feature];
		}
	}
	
	function removeFeature(feature) {
		editedRole.metadata.disabled_features = editedRole.metadata.disabled_features.filter(f => f !== feature);
	}
</script>

{#if role}
<div class="modal-overlay" on:click={handleClose}>
	<div class="modal" on:click|stopPropagation>
		<h2>Edit Role: {role.display_name || role.name}</h2>
		
		<div class="form-grid">
			<div class="form-group">
				<label for="display-name">Display Name</label>
				<input 
					id="display-name" 
					type="text" 
					bind:value={editedRole.display_name} 
					placeholder="Enter display name"
					disabled={role.is_system}
				/>
			</div>
			
			<div class="form-group">
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
			
			<div class="form-group full-width">
				<label>
					Disabled Features
					<button class="btn-small" on:click|preventDefault={addFeature}>+ Add</button>
				</label>
				<div class="features-list">
					{#if editedRole.metadata.disabled_features?.length > 0}
						{#each editedRole.metadata.disabled_features as feature}
							<span class="feature-tag">
								{feature}
								<button class="remove-btn" on:click={() => removeFeature(feature)}>×</button>
							</span>
						{/each}
					{:else}
						<span class="no-features">No features disabled</span>
					{/if}
				</div>
			</div>
		</div>
		
		<div class="modal-actions">
			<button class="btn" on:click={handleClose}>Cancel</button>
			<button class="btn btn-primary" on:click={handleSubmit}>Save Changes</button>
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
		max-width: 700px;
		width: 90%;
		max-height: 90vh;
		overflow-y: auto;
	}
	
	.modal h2 {
		margin-top: 0;
		margin-bottom: 1.5rem;
	}
	
	.form-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
		margin-bottom: 1.5rem;
	}
	
	.form-group {
		display: flex;
		flex-direction: column;
	}
	
	.form-group.full-width {
		grid-column: 1 / -1;
	}
	
	.form-group label {
		margin-bottom: 0.5rem;
		font-weight: 500;
		color: #333;
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	
	.current-value {
		font-weight: normal;
		color: #666;
		font-size: 0.85rem;
	}
	
	.form-group input,
	.form-group textarea {
		padding: 0.5rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		font-size: 1rem;
	}
	
	.form-group input:disabled {
		background: #f5f5f5;
		cursor: not-allowed;
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
		background: #4CAF50;
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
	}
	
	.btn-small:hover {
		background: #45a049;
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