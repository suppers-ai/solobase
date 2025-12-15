<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import Modal from '$lib/components/ui/Modal.svelte';

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

	$: show = policy !== null;

	function handleSubmit() {
		dispatch('save', editedPolicy);
	}

	function handleClose() {
		dispatch('close');
	}
</script>

<Modal {show} title="Edit Policy" maxWidth="600px" on:close={handleClose}>
	<form on:submit|preventDefault={handleSubmit}>
		<div class="modal-form-group">
			<label for="policy-subject">Subject (role or user ID)</label>
			<input
				id="policy-subject"
				type="text"
				bind:value={editedPolicy.subject}
				placeholder="editor"
			/>
		</div>

		<div class="modal-form-group">
			<label for="policy-resource">Resource Pattern</label>
			<input
				id="policy-resource"
				type="text"
				bind:value={editedPolicy.resource}
				placeholder="/api/storage/*"
			/>
			<p class="modal-form-hint">Use * for wildcards (e.g., /api/users/* or *)</p>
		</div>

		<div class="modal-form-group">
			<label for="policy-action">Action Pattern</label>
			<input
				id="policy-action"
				type="text"
				bind:value={editedPolicy.action}
				placeholder="read|write"
			/>
			<p class="modal-form-hint">Use | for multiple actions (e.g., read|write|delete or *)</p>
		</div>

		<div class="modal-form-group">
			<label for="policy-effect">Effect</label>
			<select id="policy-effect" bind:value={editedPolicy.effect}>
				<option value="allow">Allow</option>
				<option value="deny">Deny</option>
			</select>
			<p class="modal-form-hint">Deny rules take precedence over allow rules</p>
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
	</form>

	<svelte:fragment slot="footer">
		<button class="modal-btn modal-btn-secondary" on:click={handleClose}>Cancel</button>
		<button
			class="modal-btn modal-btn-primary"
			on:click={handleSubmit}
			disabled={!editedPolicy.subject || !editedPolicy.resource || !editedPolicy.action}
		>
			Save Changes
		</button>
	</svelte:fragment>
</Modal>

<style>
	.policy-preview {
		background: #f5f5f5;
		padding: 1rem;
		border-radius: 4px;
		margin-top: 1rem;
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
</style>
