<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { X } from 'lucide-svelte';
	import ProductForm from './ProductForm.svelte';

	export let show = false;
	export let mode: 'create' | 'edit' = 'create';
	export let product: any = null;
	export let productTemplates: any[] = [];
	export let groups: any[] = [];
	export let title = mode === 'create' ? 'Create Product' : 'Edit Product';
	export let submitButtonText = mode === 'create' ? 'Create' : 'Save';
	export let customFieldsConfig: any = null;

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

{#if show}
	<div class="modal-overlay" on:click={handleClose} on:keydown={(e) => e.key === 'Escape' && handleClose()}>
		<div class="modal-content" on:click|stopPropagation role="dialog" aria-modal="true">
			<div class="modal-header">
				<h2>{title}</h2>
				<button class="close-btn" on:click={handleClose} aria-label="Close">
					<X size={20} />
				</button>
			</div>

			<ProductForm
				{mode}
				{product}
				{productTemplates}
				{groups}
				{customFieldsConfig}
				{submitButtonText}
				on:submit={handleFormSubmit}
				on:cancel={handleFormCancel}
			/>
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
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
		padding: 1rem;
	}

	.modal-content {
		background: white;
		border-radius: 0.5rem;
		width: 100%;
		max-width: 800px;
		max-height: 90vh;
		display: flex;
		flex-direction: column;
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
		overflow: hidden;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-bottom: 1px solid #e5e7eb;
		flex-shrink: 0;
	}

	.modal-header h2 {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 600;
		color: #111827;
	}

	.close-btn {
		background: none;
		border: none;
		color: #6b7280;
		cursor: pointer;
		padding: 0.25rem;
		border-radius: 0.25rem;
		transition: all 0.2s;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.close-btn:hover {
		color: #111827;
		background: #f3f4f6;
	}

	@media (max-width: 640px) {
		.modal-content {
			max-height: 100vh;
			border-radius: 0;
		}

		.modal-overlay {
			padding: 0;
		}
	}
</style>