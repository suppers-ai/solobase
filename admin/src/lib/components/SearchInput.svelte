<script lang="ts">
	import { Search } from 'lucide-svelte';
	import { createEventDispatcher } from 'svelte';
	
	export let value = '';
	export let placeholder = 'Search...';
	export let maxWidth = '300px';
	export let disabled = false;
	
	const dispatch = createEventDispatcher();
	
	function handleInput(event: Event) {
		const target = event.target as HTMLInputElement;
		value = target.value;
		dispatch('input', value);
	}
	
	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			dispatch('search', value);
		}
		dispatch('keydown', event);
	}
</script>

<div class="search-box" style="max-width: {maxWidth}">
	<Search size={16} />
	<input 
		type="text"
		{placeholder}
		{disabled}
		bind:value
		on:input={handleInput}
		on:keydown={handleKeydown}
		on:focus
		on:blur
	/>
</div>

<style>
	.search-box {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex: 1;
		height: 36px;
		padding: 0 0.75rem;
		border: 1px solid var(--border-color, #e5e7eb);
		border-radius: 6px;
		background: white;
		transition: all 0.2s;
	}
	
	.search-box:focus-within {
		border-color: var(--primary-color, #06b6d4);
		box-shadow: 0 0 0 3px rgba(24, 154, 180, 0.1);
	}
	
	.search-box input {
		border: none;
		background: none;
		outline: none;
		flex: 1;
		font-size: 0.875rem;
		color: var(--text-primary, #111827);
	}
	
	.search-box input::placeholder {
		color: var(--text-muted, #6b7280);
	}
	
	.search-box input:disabled {
		cursor: not-allowed;
		opacity: 0.6;
	}
</style>