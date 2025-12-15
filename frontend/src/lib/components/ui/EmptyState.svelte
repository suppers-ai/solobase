<script lang="ts">
	import type { ComponentType } from 'svelte';

	export let title: string = '';
	export let message: string = '';
	export let icon: ComponentType | null = null;
	export let iconSize: number = 48;
	export let compact: boolean = false;
</script>

<div class="empty-state" class:compact>
	{#if icon}
		<div class="icon">
			<svelte:component this={icon} size={iconSize} />
		</div>
	{/if}
	{#if title}
		<h3>{title}</h3>
	{/if}
	{#if message}
		<p>{message}</p>
	{/if}
	<slot />
</div>

<style>
	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 4rem 2rem;
		text-align: center;
		color: #9ca3af;
	}

	.empty-state.compact {
		padding: 2rem 1rem;
	}

	.icon {
		color: #d1d5db;
		margin-bottom: 1rem;
	}

	h3 {
		margin: 0 0 0.5rem 0;
		font-size: 1.125rem;
		font-weight: 600;
		color: #111827;
	}

	p {
		margin: 0;
		color: #6b7280;
		font-size: 0.875rem;
		max-width: 400px;
	}

	.empty-state :global(button) {
		margin-top: 1rem;
	}
</style>
