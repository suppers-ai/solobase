<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let checked = false;
	export let disabled = false;
	export let size: 'sm' | 'md' | 'lg' = 'md';
	export let label = '';
	export let labelPosition: 'left' | 'right' = 'right';

	const dispatch = createEventDispatcher();

	function handleClick() {
		if (disabled) return;
		checked = !checked;
		dispatch('change', checked);
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			handleClick();
		}
	}

	$: dimensions = {
		sm: { width: 36, height: 20, slider: 16, offset: 16 },
		md: { width: 44, height: 24, slider: 20, offset: 20 },
		lg: { width: 52, height: 28, slider: 24, offset: 24 }
	}[size];
</script>

<div class="toggle-container" class:has-label={label} class:label-left={labelPosition === 'left'}>
	{#if label && labelPosition === 'left'}
		<span class="toggle-label">{label}</span>
	{/if}

	<button
		type="button"
		class="toggle {size}"
		class:checked
		class:disabled
		style="width: {dimensions.width}px; height: {dimensions.height}px;"
		on:click={handleClick}
		on:keydown={handleKeydown}
		{disabled}
		role="switch"
		aria-checked={checked}
		aria-label={label || 'Toggle'}
	>
		<span
			class="toggle-slider"
			style="width: {dimensions.slider}px; height: {dimensions.slider}px; transform: translateX({checked ? dimensions.offset : 0}px);"
		></span>
	</button>

	{#if label && labelPosition === 'right'}
		<span class="toggle-label">{label}</span>
	{/if}
</div>

<style>
	.toggle-container {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
	}

	.toggle-container.has-label {
		cursor: pointer;
	}

	.toggle-label {
		font-size: 0.875rem;
		color: #374151;
		user-select: none;
	}

	.toggle {
		position: relative;
		background: #d1d5db;
		border: none;
		border-radius: 9999px;
		cursor: pointer;
		transition: background 0.2s ease;
		padding: 2px;
	}

	.toggle:hover:not(.disabled) {
		background: #9ca3af;
	}

	.toggle.checked {
		background: #10b981;
	}

	.toggle.checked:hover:not(.disabled) {
		background: #059669;
	}

	.toggle.disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.toggle:focus-visible {
		outline: none;
		box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.3);
	}

	.toggle-slider {
		display: block;
		background: white;
		border-radius: 50%;
		transition: transform 0.2s ease;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
	}
</style>
