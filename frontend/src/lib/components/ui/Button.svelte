<script lang="ts">
	import type { ComponentType } from 'svelte';
	import { createEventDispatcher } from 'svelte';

	export let variant: 'primary' | 'secondary' | 'danger' | 'ghost' | 'link' = 'primary';
	export let size: 'sm' | 'md' | 'lg' = 'md';
	export let icon: ComponentType | null = null;
	export let iconOnly: boolean = false;
	export let disabled: boolean = false;
	export let loading: boolean = false;
	export let type: 'button' | 'submit' | 'reset' = 'button';
	export let href: string = '';

	const dispatch = createEventDispatcher();

	function handleClick(event: MouseEvent) {
		if (!disabled && !loading) {
			dispatch('click', event);
		}
	}

	$: iconSize = size === 'sm' ? 14 : size === 'lg' ? 18 : 16;
</script>

{#if href}
	<a
		{href}
		class="btn {variant} {size}"
		class:icon-only={iconOnly}
		class:disabled
		on:click={handleClick}
	>
		{#if loading}
			<span class="spinner"></span>
		{:else if icon}
			<svelte:component this={icon} size={iconSize} />
		{/if}
		{#if !iconOnly}
			<slot />
		{/if}
	</a>
{:else}
	<button
		{type}
		class="btn {variant} {size}"
		class:icon-only={iconOnly}
		{disabled}
		on:click={handleClick}
	>
		{#if loading}
			<span class="spinner"></span>
		{:else if icon}
			<svelte:component this={icon} size={iconSize} />
		{/if}
		{#if !iconOnly}
			<slot />
		{/if}
	</button>
{/if}

<style>
	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		font-weight: 500;
		border: none;
		border-radius: 0.375rem;
		cursor: pointer;
		transition: all 0.15s ease;
		text-decoration: none;
		white-space: nowrap;
	}

	/* Sizes */
	.btn.sm {
		padding: 0.375rem 0.75rem;
		font-size: 0.813rem;
		gap: 0.375rem;
	}

	.btn.md {
		padding: 0.5rem 1rem;
		font-size: 0.875rem;
	}

	.btn.lg {
		padding: 0.625rem 1.25rem;
		font-size: 1rem;
	}

	/* Icon only */
	.btn.icon-only.sm {
		padding: 0.375rem;
		width: 28px;
		height: 28px;
	}

	.btn.icon-only.md {
		padding: 0.5rem;
		width: 32px;
		height: 32px;
	}

	.btn.icon-only.lg {
		padding: 0.625rem;
		width: 40px;
		height: 40px;
	}

	/* Variants */
	.btn.primary {
		background: #189AB4;
		color: white;
	}

	.btn.primary:hover:not(:disabled) {
		background: #147a91;
	}

	.btn.secondary {
		background: white;
		color: #374151;
		border: 1px solid #e5e7eb;
	}

	.btn.secondary:hover:not(:disabled) {
		background: #f9fafb;
		border-color: #d1d5db;
	}

	.btn.danger {
		background: #ef4444;
		color: white;
	}

	.btn.danger:hover:not(:disabled) {
		background: #dc2626;
	}

	.btn.ghost {
		background: transparent;
		color: #374151;
	}

	.btn.ghost:hover:not(:disabled) {
		background: #f3f4f6;
	}

	.btn.link {
		background: transparent;
		color: #189AB4;
		padding: 0;
	}

	.btn.link:hover:not(:disabled) {
		color: #147a91;
		text-decoration: underline;
	}

	/* Disabled state */
	.btn:disabled,
	.btn.disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	/* Spinner */
	.spinner {
		width: 1em;
		height: 1em;
		border: 2px solid currentColor;
		border-right-color: transparent;
		border-radius: 50%;
		animation: spin 0.6s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	/* Focus state */
	.btn:focus-visible {
		outline: none;
		box-shadow: 0 0 0 3px rgba(24, 154, 180, 0.3);
	}
</style>
