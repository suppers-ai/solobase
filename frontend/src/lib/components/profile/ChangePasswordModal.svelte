<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import Modal from '$lib/components/ui/Modal.svelte';
	import Button from '$lib/components/ui/Button.svelte';

	export let show = false;
	export let error = '';
	export let passwordForm = {
		currentPassword: '',
		newPassword: '',
		confirmPassword: ''
	};

	const dispatch = createEventDispatcher();

	function handleClose() {
		passwordForm = { currentPassword: '', newPassword: '', confirmPassword: '' };
		dispatch('close');
	}

	function handleSubmit() {
		dispatch('submit');
	}
</script>

<Modal {show} title="Change Password" on:close={handleClose}>
	{#if error}
		<div class="alert alert-error">{error}</div>
	{/if}

	<div class="form-group">
		<label for="currentPassword">Current Password</label>
		<input
			type="password"
			id="currentPassword"
			bind:value={passwordForm.currentPassword}
			placeholder="Enter current password"
		/>
	</div>

	<div class="form-group">
		<label for="newPassword">New Password</label>
		<input
			type="password"
			id="newPassword"
			bind:value={passwordForm.newPassword}
			placeholder="Enter new password (min 8 characters)"
		/>
	</div>

	<div class="form-group">
		<label for="confirmPassword">Confirm New Password</label>
		<input
			type="password"
			id="confirmPassword"
			bind:value={passwordForm.confirmPassword}
			placeholder="Confirm new password"
		/>
	</div>

	<svelte:fragment slot="footer">
		<Button variant="secondary" on:click={handleClose}>Cancel</Button>
		<Button on:click={handleSubmit}>Change Password</Button>
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
