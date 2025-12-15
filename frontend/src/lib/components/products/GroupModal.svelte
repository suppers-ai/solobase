<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import Modal from '$lib/components/ui/Modal.svelte';
	import GroupForm from './GroupForm.svelte';

	export let show = false;
	export let mode: 'create' | 'edit' = 'create';
	export let group: any = null;
	export let title = mode === 'create' ? 'Create Group' : 'Edit Group';
	export let submitButtonText = mode === 'create' ? 'Create Group' : 'Save Changes';

	const dispatch = createEventDispatcher();

	function handleClose() {
		dispatch('close');
		show = false;
	}

	function handleFormSubmit(event: CustomEvent) {
		dispatch('submit', event.detail);
		handleClose();
	}

	function handleFormCancel() {
		handleClose();
	}
</script>

<Modal {show} {title} maxWidth="600px" on:close={handleClose}>
	<GroupForm
		{mode}
		{group}
		{submitButtonText}
		on:submit={handleFormSubmit}
		on:cancel={handleFormCancel}
	/>
</Modal>
