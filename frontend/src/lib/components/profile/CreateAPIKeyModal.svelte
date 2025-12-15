<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { Key } from 'lucide-svelte';
	import Modal from '$lib/components/ui/Modal.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import LoadingSpinner from '$lib/components/ui/LoadingSpinner.svelte';

	export let show = false;
	export let creating = false;
	export let error = '';
	export let keyName = '';
	export let keyExpiry: string | null = null;

	const dispatch = createEventDispatcher();

	function handleClose() {
		keyName = '';
		keyExpiry = null;
		dispatch('close');
	}

	function handleCreate() {
		dispatch('create', { name: keyName, expiry: keyExpiry });
	}
</script>

<Modal {show} title="Create API Key" on:close={handleClose}>
	{#if error}
		<div class="alert alert-error">{error}</div>
	{/if}

	<div class="form-group">
		<label for="keyName">Key Name</label>
		<input
			type="text"
			id="keyName"
			bind:value={keyName}
			placeholder="e.g., Production Server, CI/CD Pipeline"
		/>
		<span class="form-hint">A descriptive name to identify this key</span>
	</div>

	<div class="form-group">
		<label for="keyExpiry">Expiration (Optional)</label>
		<input
			type="datetime-local"
			id="keyExpiry"
			bind:value={keyExpiry}
		/>
		<span class="form-hint">Leave empty for a key that never expires</span>
	</div>

	<svelte:fragment slot="footer">
		<Button variant="secondary" on:click={handleClose}>Cancel</Button>
		<Button on:click={handleCreate} disabled={creating}>
			{#if creating}
				<LoadingSpinner size="sm" color="white" />
				Creating...
			{:else}
				<Key size={16} />
				Create Key
			{/if}
		</Button>
	</svelte:fragment>
</Modal>

<style>
	.form-group {
		margin-bottom: 1rem;
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
		padding: 0.625rem 0.875rem;
		border: 1px solid #d1d5db;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		transition: all 0.15s;
	}

	.form-group input:focus {
		outline: none;
		border-color: #189AB4;
		box-shadow: 0 0 0 3px rgba(24, 154, 180, 0.1);
	}

	.form-hint {
		display: block;
		margin-top: 0.25rem;
		font-size: 0.75rem;
		color: #6b7280;
	}

	.alert {
		padding: 0.75rem 1rem;
		border-radius: 0.375rem;
		margin-bottom: 1rem;
		font-size: 0.875rem;
	}

	.alert-error {
		background: #fee2e2;
		color: #991b1b;
		border: 1px solid #fecaca;
	}
</style>
