<script>
	import { createEventDispatcher } from 'svelte';
	
	const dispatch = createEventDispatcher();
	
	let policy = {
		subject: '',
		resource: '',
		action: '',
		effect: 'allow'
	};
	
	function handleSubmit() {
		dispatch('create', policy);
	}
	
	function handleClose() {
		dispatch('close');
	}
</script>

<div class="modal-overlay" on:click={handleClose}>
	<div class="modal" on:click|stopPropagation>
		<h2>Create New Policy</h2>
		
		<div class="form-group">
			<label for="policy-subject">Subject (role or user ID)</label>
			<input id="policy-subject" type="text" bind:value={policy.subject} placeholder="editor" />
		</div>
		
		<div class="form-group">
			<label for="policy-resource">Resource Pattern</label>
			<input id="policy-resource" type="text" bind:value={policy.resource} placeholder="/api/storage/*" />
		</div>
		
		<div class="form-group">
			<label for="policy-action">Action Pattern</label>
			<input id="policy-action" type="text" bind:value={policy.action} placeholder="read|write" />
		</div>
		
		<div class="form-group">
			<label for="policy-effect">Effect</label>
			<select id="policy-effect" bind:value={policy.effect}>
				<option value="allow">Allow</option>
				<option value="deny">Deny</option>
			</select>
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