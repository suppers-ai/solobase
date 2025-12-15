<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { X } from 'lucide-svelte';
	import { fade, scale } from 'svelte/transition';

	export let show = false;
	export let title = '';
	export let maxWidth = '500px';
	export let closeOnOverlay = true;

	const dispatch = createEventDispatcher();

	function handleClose() {
		dispatch('close');
	}

	function handleOverlayClick() {
		if (closeOnOverlay) {
			handleClose();
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && show) {
			handleClose();
		}
	}
</script>

<svelte:window on:keydown={handleKeydown} />

{#if show}
	<div class="modal-overlay" on:click={handleOverlayClick} transition:fade={{ duration: 150 }}>
		<div
			class="modal"
			style="max-width: {maxWidth}"
			on:click|stopPropagation
			transition:scale={{ duration: 150, start: 0.95 }}
		>
			<div class="modal-header">
				<h3>{title}</h3>
				<button class="icon-button" on:click={handleClose} type="button" aria-label="Close">
					<X size={20} />
				</button>
			</div>

			<div class="modal-body">
				<slot />
			</div>

			{#if $$slots.footer}
				<div class="modal-footer">
					<slot name="footer" />
				</div>
			{/if}
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
	}

	.modal {
		background: white;
		border-radius: 0.5rem;
		width: 90%;
		box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.25);
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-bottom: 1px solid #e5e7eb;
	}

	.modal-header h3 {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 600;
	}

	.modal-body {
		padding: 1.5rem;
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid #e5e7eb;
	}

	.icon-button {
		padding: 0.25rem;
		background: transparent;
		border: none;
		border-radius: 0.25rem;
		cursor: pointer;
		color: #6b7280;
		transition: all 0.2s;
	}

	.icon-button:hover {
		background: #f3f4f6;
		color: #374151;
	}

	/* Export common button styles for consumers */
	:global(.modal-btn) {
		padding: 0.5rem 1rem;
		border: none;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	:global(.modal-btn:disabled) {
		opacity: 0.5;
		cursor: not-allowed;
	}

	:global(.modal-btn-primary) {
		background: #189AB4;
		color: white;
	}

	:global(.modal-btn-primary:hover:not(:disabled)) {
		background: #157a8f;
	}

	:global(.modal-btn-secondary) {
		background: #f3f4f6;
		color: #374151;
	}

	:global(.modal-btn-secondary:hover:not(:disabled)) {
		background: #e5e7eb;
	}

	:global(.modal-btn-danger) {
		background: #dc2626;
		color: white;
	}

	:global(.modal-btn-danger:hover:not(:disabled)) {
		background: #b91c1c;
	}

	/* Common form styles */
	:global(.modal-form-group) {
		margin-bottom: 1.5rem;
	}

	:global(.modal-form-group:last-child) {
		margin-bottom: 0;
	}

	:global(.modal-form-group label) {
		display: block;
		margin-bottom: 0.5rem;
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
	}

	:global(.modal-form-group input[type="text"]),
	:global(.modal-form-group input[type="email"]),
	:global(.modal-form-group input[type="password"]),
	:global(.modal-form-group textarea),
	:global(.modal-form-group select) {
		width: 100%;
		padding: 0.5rem 0.75rem;
		border: 1px solid #d1d5db;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		transition: all 0.2s;
	}

	:global(.modal-form-group input:focus),
	:global(.modal-form-group textarea:focus),
	:global(.modal-form-group select:focus) {
		outline: none;
		border-color: #189AB4;
		box-shadow: 0 0 0 3px rgba(24, 154, 180, 0.1);
	}

	:global(.modal-form-group input:disabled),
	:global(.modal-form-group textarea:disabled),
	:global(.modal-form-group select:disabled) {
		background: #f3f4f6;
		cursor: not-allowed;
	}

	:global(.modal-form-hint) {
		margin: 0.5rem 0 0 0;
		font-size: 0.75rem;
		color: #6b7280;
	}
</style>
