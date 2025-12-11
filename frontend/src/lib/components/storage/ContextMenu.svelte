<script lang="ts">
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import { 
		Download, Eye, Edit2, Trash2, Copy, 
		Move, Share2, Info 
	} from 'lucide-svelte';

	export let show = false;
	export let x = 0;
	export let y = 0;
	export let item: any = null;

	const dispatch = createEventDispatcher();
	let menuEl: HTMLElement;

	function handleAction(action: string) {
		dispatch('action', { action, item });
		dispatch('close');
	}

	function handleClickOutside(event: MouseEvent) {
		if (menuEl && !menuEl.contains(event.target as Node)) {
			dispatch('close');
		}
	}

	onMount(() => {
		if (show) {
			document.addEventListener('click', handleClickOutside);
		}
	});

	onDestroy(() => {
		document.removeEventListener('click', handleClickOutside);
	});

	$: if (show) {
		document.addEventListener('click', handleClickOutside);
	} else {
		document.removeEventListener('click', handleClickOutside);
	}
</script>

{#if show && item}
	<div 
		class="context-menu" 
		style="left: {x}px; top: {y}px;"
		bind:this={menuEl}
	>
		{#if !item.isFolder}
			<button class="menu-item" on:click={() => handleAction('preview')}>
				<Eye size={16} />
				<span>Preview</span>
			</button>
			<button class="menu-item" on:click={() => handleAction('download')}>
				<Download size={16} />
				<span>Download</span>
			</button>
			<div class="menu-divider"></div>
		{/if}

		<button class="menu-item" on:click={() => handleAction('rename')}>
			<Edit2 size={16} />
			<span>Rename</span>
		</button>
		<button class="menu-item" on:click={() => handleAction('copy')}>
			<Copy size={16} />
			<span>Copy</span>
		</button>
		<button class="menu-item" on:click={() => handleAction('move')}>
			<Move size={16} />
			<span>Move</span>
		</button>

		{#if !item.isFolder}
			<button class="menu-item" on:click={() => handleAction('share')}>
				<Share2 size={16} />
				<span>Share</span>
			</button>
		{/if}
		
		<div class="menu-divider"></div>
		
		<button class="menu-item" on:click={() => handleAction('info')}>
			<Info size={16} />
			<span>Properties</span>
		</button>
		
		<div class="menu-divider"></div>
		
		<button class="menu-item danger" on:click={() => handleAction('delete')}>
			<Trash2 size={16} />
			<span>Delete</span>
		</button>
	</div>
{/if}

<style>
	.context-menu {
		position: fixed;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05);
		padding: 0.25rem;
		min-width: 200px;
		z-index: 1001;
	}

	.menu-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		width: 100%;
		padding: 0.5rem 0.75rem;
		background: transparent;
		border: none;
		border-radius: 0.25rem;
		font-size: 0.875rem;
		color: #374151;
		cursor: pointer;
		transition: all 0.15s;
		text-align: left;
	}

	.menu-item:hover {
		background: #f3f4f6;
	}

	.menu-item.danger {
		color: #dc2626;
	}

	.menu-item.danger:hover {
		background: #fee2e2;
	}

	.menu-divider {
		height: 1px;
		background: #e5e7eb;
		margin: 0.25rem 0;
	}
</style>