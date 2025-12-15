<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { ComponentType } from 'svelte';

	interface Tab {
		id: string;
		label: string;
		icon?: ComponentType;
	}

	export let tabs: Tab[];
	export let activeTab: string;
	export let variant: 'default' | 'pills' | 'underline' = 'underline';

	const dispatch = createEventDispatcher<{ change: string }>();

	function handleTabClick(tabId: string) {
		activeTab = tabId;
		dispatch('change', tabId);
	}
</script>

<div class="tabs {variant}">
	{#each tabs as tab}
		<button
			class="tab"
			class:active={activeTab === tab.id}
			on:click={() => handleTabClick(tab.id)}
		>
			{#if tab.icon}
				<svelte:component this={tab.icon} size={18} />
			{/if}
			<span>{tab.label}</span>
		</button>
	{/each}
</div>

<style>
	.tabs {
		display: flex;
		gap: 0.5rem;
	}

	.tabs.underline {
		border-bottom: 1px solid #e5e7eb;
		gap: 0;
	}

	.tabs.pills {
		gap: 0.5rem;
	}

	.tab {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		font-size: 0.875rem;
		font-weight: 500;
		color: #6b7280;
		background: none;
		border: none;
		cursor: pointer;
		transition: all 0.15s ease;
		position: relative;
	}

	/* Underline variant */
	.tabs.underline .tab {
		border-bottom: 2px solid transparent;
		margin-bottom: -1px;
		border-radius: 0;
	}

	.tabs.underline .tab:hover {
		color: #374151;
		background: #f9fafb;
	}

	.tabs.underline .tab.active {
		color: #189AB4;
		border-bottom-color: #189AB4;
	}

	/* Pills variant */
	.tabs.pills .tab {
		border-radius: 0.375rem;
		background: #f3f4f6;
	}

	.tabs.pills .tab:hover {
		background: #e5e7eb;
	}

	.tabs.pills .tab.active {
		background: #189AB4;
		color: white;
	}

	/* Default variant */
	.tabs.default .tab {
		border-radius: 0.375rem;
	}

	.tabs.default .tab:hover {
		background: #f3f4f6;
		color: #374151;
	}

	.tabs.default .tab.active {
		background: #189AB4;
		color: white;
	}

	/* Icon styling */
	.tab :global(svg) {
		flex-shrink: 0;
	}

	@media (max-width: 640px) {
		.tabs {
			overflow-x: auto;
			-webkit-overflow-scrolling: touch;
			white-space: nowrap;
		}

		.tab {
			padding: 0.5rem 0.75rem;
			font-size: 0.813rem;
		}
	}
</style>
