<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { Save } from 'lucide-svelte';
	import Modal from '$lib/components/ui/Modal.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import LoadingSpinner from '$lib/components/ui/LoadingSpinner.svelte';

	export let show = false;
	export let saving = false;
	export let error = '';
	export let profileForm = {
		firstName: '',
		lastName: '',
		displayName: '',
		email: '',
		phone: '',
		location: ''
	};

	const dispatch = createEventDispatcher();

	function handleClose() {
		dispatch('close');
	}

	function handleSave() {
		dispatch('save');
	}
</script>

<Modal {show} title="Account Settings" on:close={handleClose}>
	{#if error}
		<div class="alert alert-error">{error}</div>
	{/if}

	<div class="form-row">
		<div class="form-group">
			<label for="firstName">First Name</label>
			<input
				type="text"
				id="firstName"
				bind:value={profileForm.firstName}
				placeholder="Enter first name"
			/>
		</div>

		<div class="form-group">
			<label for="lastName">Last Name</label>
			<input
				type="text"
				id="lastName"
				bind:value={profileForm.lastName}
				placeholder="Enter last name"
			/>
		</div>
	</div>

	<div class="form-group">
		<label for="displayName">Display Name</label>
		<input
			type="text"
			id="displayName"
			bind:value={profileForm.displayName}
			placeholder="Enter display name"
		/>
	</div>

	<div class="form-group">
		<label for="email">Email</label>
		<input
			type="email"
			id="email"
			value={profileForm.email}
			disabled
			class="disabled"
		/>
	</div>

	<div class="form-group">
		<label for="phone">Phone</label>
		<input
			type="tel"
			id="phone"
			bind:value={profileForm.phone}
			placeholder="Enter phone number"
		/>
	</div>

	<div class="form-group">
		<label for="location">Location</label>
		<input
			type="text"
			id="location"
			bind:value={profileForm.location}
			placeholder="Enter location"
		/>
	</div>

	<svelte:fragment slot="footer">
		<Button variant="secondary" on:click={handleClose}>Cancel</Button>
		<Button on:click={handleSave} disabled={saving}>
			{#if saving}
				<LoadingSpinner size="sm" color="white" />
				Saving...
			{:else}
				<Save size={16} />
				Save Changes
			{/if}
		</Button>
	</svelte:fragment>
</Modal>

<style>
	.form-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
	}

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

	.form-group input.disabled {
		background: #f3f4f6;
		color: #6b7280;
		cursor: not-allowed;
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

	@media (max-width: 640px) {
		.form-row {
			grid-template-columns: 1fr;
		}
	}
</style>
