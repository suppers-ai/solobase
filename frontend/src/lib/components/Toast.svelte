<script lang="ts">
	import { fly, fade } from 'svelte/transition';
	import { flip } from 'svelte/animate';
	import { CheckCircle, XCircle, AlertCircle, Info, X } from 'lucide-svelte';
	import { toasts } from '$lib/stores/toast';

	function getIcon(type: string) {
		switch (type) {
			case 'success': return CheckCircle;
			case 'error': return XCircle;
			case 'warning': return AlertCircle;
			case 'info': return Info;
			default: return Info;
		}
	}

	function getIconColor(type: string) {
		switch (type) {
			case 'success': return '#10b981';
			case 'error': return '#ef4444';
			case 'warning': return '#f59e0b';
			case 'info': return '#3b82f6';
			default: return '#6b7280';
		}
	}
</script>

<div class="toast-container" aria-live="polite" aria-atomic="false">
	{#each $toasts as toast (toast.id)}
		<div
			class="toast toast-{toast.type}"
			in:fly={{ y: -20, duration: 300 }}
			out:fade={{ duration: 200 }}
			animate:flip={{ duration: 300 }}
			role="alert"
			aria-live={toast.type === 'error' ? 'assertive' : 'polite'}
		>
			<div class="toast-content">
				<svelte:component 
					this={getIcon(toast.type)} 
					size={20} 
					color={getIconColor(toast.type)}
				/>
				<div class="toast-message">
					{#if toast.title}
						<div class="toast-title">{toast.title}</div>
					{/if}
					<div class="toast-description">{toast.message}</div>
				</div>
			</div>
			{#if toast.dismissible}
				<button
					class="toast-close"
					on:click={() => toasts.dismiss(toast.id)}
					aria-label="Dismiss notification"
				>
					<X size={16} />
				</button>
			{/if}
		</div>
	{/each}
</div>

<style>
	.toast-container {
		position: fixed;
		top: 1rem;
		right: 1rem;
		z-index: 9999;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		pointer-events: none;
	}

	.toast {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
		max-width: 400px;
		padding: 1rem;
		background: white;
		border-radius: 0.5rem;
		box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05);
		pointer-events: all;
		position: relative;
	}

	.toast-success {
		border-left: 4px solid #10b981;
	}

	.toast-error {
		border-left: 4px solid #ef4444;
	}

	.toast-warning {
		border-left: 4px solid #f59e0b;
	}

	.toast-info {
		border-left: 4px solid #3b82f6;
	}

	.toast-content {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
		flex: 1;
	}

	.toast-message {
		flex: 1;
	}

	.toast-title {
		font-weight: 600;
		font-size: 0.875rem;
		color: #111827;
		margin-bottom: 0.25rem;
	}

	.toast-description {
		font-size: 0.875rem;
		color: #6b7280;
		line-height: 1.5;
	}

	.toast-close {
		padding: 0.25rem;
		background: transparent;
		border: none;
		border-radius: 0.25rem;
		cursor: pointer;
		color: #6b7280;
		transition: all 0.2s;
	}

	.toast-close:hover {
		background: #f3f4f6;
		color: #374151;
	}

	.toast-close:focus {
		outline: none;
		box-shadow: 0 0 0 2px rgba(24, 154, 180, 0.2);
	}

	@media (max-width: 640px) {
		.toast-container {
			left: 1rem;
			right: 1rem;
		}

		.toast {
			max-width: none;
		}
	}
</style>