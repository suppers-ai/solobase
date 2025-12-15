<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import Modal from '$lib/components/ui/Modal.svelte';

	export let show = false;

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
		policy = { subject: '', resource: '', action: '', effect: 'allow' };
		dispatch('close');
	}
</script>

<Modal {show} title="Create New Policy" maxWidth="600px" on:close={handleClose}>
	<form on:submit|preventDefault={handleSubmit}>
		<div class="modal-form-group">
			<label for="policy-subject">Subject (role or user ID)</label>
			<input id="policy-subject" type="text" bind:value={policy.subject} placeholder="editor" />
		</div>

		<div class="modal-form-group">
			<label for="policy-resource">Resource Pattern</label>
			<input id="policy-resource" type="text" bind:value={policy.resource} placeholder="/api/storage/*" />
		</div>

		<div class="modal-form-group">
			<label for="policy-action">Action Pattern</label>
			<input id="policy-action" type="text" bind:value={policy.action} placeholder="read|write" />
		</div>

		<div class="modal-form-group">
			<label for="policy-effect">Effect</label>
			<select id="policy-effect" bind:value={policy.effect}>
				<option value="allow">Allow</option>
				<option value="deny">Deny</option>
			</select>
		</div>
	</form>

	<svelte:fragment slot="footer">
		<button class="modal-btn modal-btn-secondary" on:click={handleClose}>Cancel</button>
		<button class="modal-btn modal-btn-primary" on:click={handleSubmit}>Create</button>
	</svelte:fragment>
</Modal>
