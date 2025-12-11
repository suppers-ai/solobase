<script lang="ts">
	import { ChevronRight, Home } from 'lucide-svelte';
	import { createEventDispatcher } from 'svelte';

	export let currentPath = '';

	const dispatch = createEventDispatcher();

	$: pathSegments = currentPath ? currentPath.split('/').filter(Boolean) : [];

	function navigateToPath(index: number) {
		if (index === -1) {
			// Navigate to root
			dispatch('navigate', '');
		} else {
			// Navigate to specific path
			const newPath = pathSegments.slice(0, index + 1).join('/');
			dispatch('navigate', newPath);
		}
	}
</script>

<div class="breadcrumb">
	<button
		class="breadcrumb-item"
		class:active={!currentPath}
		on:click={() => navigateToPath(-1)}
	>
		<Home size={16} />
		<span>Root</span>
	</button>

	{#each pathSegments as segment, index}
		<ChevronRight size={16} class="separator" />
		<button
			class="breadcrumb-item"
			class:active={index === pathSegments.length - 1}
			on:click={() => navigateToPath(index)}
		>
			{segment}
		</button>
	{/each}
</div>

<style>
	.breadcrumb {
		display: flex;
		align-items: center;
		padding: 0.75rem 1rem;
		background: #f9fafb;
		border-bottom: 1px solid #e5e7eb;
		font-size: 0.875rem;
		overflow-x: auto;
	}

	.breadcrumb-item {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.25rem 0.5rem;
		background: transparent;
		border: none;
		border-radius: 0.25rem;
		color: #6b7280;
		cursor: pointer;
		transition: all 0.2s;
		white-space: nowrap;
	}

	.breadcrumb-item:hover {
		background: white;
		color: #374151;
	}

	.breadcrumb-item.active {
		color: #111827;
		font-weight: 500;
		cursor: default;
	}

	.breadcrumb-item.active:hover {
		background: transparent;
	}

	:global(.separator) {
		color: #d1d5db;
		flex-shrink: 0;
	}
</style>