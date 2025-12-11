<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	interface Policy {
		subject: string;
		resource: string;
		action: string;
		effect: 'allow' | 'deny';
	}

	export let policy: Policy | null = null;

	const dispatch = createEventDispatcher();

	let editedPolicy: Policy = {
		subject: '',
		resource: '',
		action: '',
		effect: 'allow'
	};

	$: if (policy) {
		editedPolicy = { ...policy };
	}

	function handleSubmit() {
		dispatch('save', editedPolicy);
	}

	function handleClose() {
		dispatch('close');
	}
</script>

{#if policy}
<div class="modal-overlay" on:click={handleClose}>
	<div class="modal" on:click|stopPropagation>
		<h2>Edit Policy</h2>
		
		<div class="form-group">
			<label for="policy-subject">Subject (role or user ID)</label>
			<input 
				id="policy-subject" 
				type="text" 
				bind:value={editedPolicy.subject} 
				placeholder="editor"
			/>
		</div>
		
		<div class="form-group">
			<label for="policy-resource">Resource Pattern</label>
			<input 
				id="policy-resource" 
				type="text" 
				bind:value={editedPolicy.resource} 
				placeholder="/api/storage/*"
			/>
			<small>Use * for wildcards (e.g., /api/users/* or *)</small>
		</div>
		
		<div class="form-group">
			<label for="policy-action">Action Pattern</label>
			<input 
				id="policy-action" 
				type="text" 
				bind:value={editedPolicy.action} 
				placeholder="read|write"
			/>
			<small>Use | for multiple actions (e.g., read|write|delete or *)</small>
		</div>
		
		<div class="form-group">
			<label for="policy-effect">Effect</label>
			<select id="policy-effect" bind:value={editedPolicy.effect}>
				<option value="allow">Allow</option>
				<option value="deny">Deny</option>
			</select>
			<small>Deny rules take precedence over allow rules</small>
		</div>
		
		<div class="policy-preview">
			<h4>Policy Preview:</h4>
			<code>
				{editedPolicy.effect === 'allow' ? 'ALLOW' : 'DENY'} 
				{editedPolicy.subject || '...'} to 
				{editedPolicy.action || '...'} on 
				{editedPolicy.resource || '...'}
			</code>
		</div>
		
		<div class="modal-actions">
			<button class="btn" on:click={handleClose}>Cancel</button>
			<button 
				class="btn btn-primary" 
				on:click={handleSubmit}
				disabled={!editedPolicy.subject || !editedPolicy.resource || !editedPolicy.action}
			>
				Save Changes
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
		margin-bottom: 1.5rem;
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
	.form-group select {
		width: 100%;
		padding: 0.5rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		font-size: 1rem;
	}
	
	.form-group small {
		display: block;
		margin-top: 0.25rem;
		color: #666;
		font-size: 0.85rem;
	}
	
	.policy-preview {
		background: #f5f5f5;
		padding: 1rem;
		border-radius: 4px;
		margin: 1.5rem 0;
	}
	
	.policy-preview h4 {
		margin: 0 0 0.5rem;
		font-size: 0.9rem;
		color: #666;
	}
	
	.policy-preview code {
		display: block;
		font-family: monospace;
		color: #333;
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
	
	.btn:hover:not(:disabled) {
		background: #f5f5f5;
	}
	
	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	
	.btn-primary {
		background: #4CAF50;
		color: white;
		border-color: #4CAF50;
	}
	
	.btn-primary:hover:not(:disabled) {
		background: #45a049;
	}
</style>