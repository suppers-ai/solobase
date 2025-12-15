<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { Filter } from 'lucide-svelte';
	import SearchInput from '$lib/components/SearchInput.svelte';

	interface FilterOption {
		value: string;
		label: string;
	}

	interface FilterConfig {
		name: string;
		label?: string;
		options: FilterOption[];
		value?: string;
	}

	export let searchValue = '';
	export let searchPlaceholder = 'Search...';
	export let searchMaxWidth = '320px';
	export let filters: FilterConfig[] = [];
	export let showFilterButton = false;

	const dispatch = createEventDispatcher();

	function handleSearchChange() {
		dispatch('search', searchValue);
	}

	function handleFilterChange(filterName: string, value: string) {
		const filter = filters.find(f => f.name === filterName);
		if (filter) {
			filter.value = value;
			filters = filters; // Trigger reactivity
		}
		dispatch('filter', { name: filterName, value });
	}

	function handleFilterButtonClick() {
		dispatch('filterClick');
	}
</script>

<div class="filter-bar">
	<div class="filter-bar-left">
		<SearchInput
			bind:value={searchValue}
			placeholder={searchPlaceholder}
			maxWidth={searchMaxWidth}
			on:input={handleSearchChange}
		/>

		{#each filters as filter}
			<select
				class="filter-select"
				value={filter.value || ''}
				on:change={(e) => handleFilterChange(filter.name, e.currentTarget.value)}
			>
				{#if filter.label}
					<option value="" disabled>{filter.label}</option>
				{/if}
				{#each filter.options as option}
					<option value={option.value}>{option.label}</option>
				{/each}
			</select>
		{/each}

		<slot name="left" />
	</div>

	<div class="filter-bar-right">
		<slot name="right" />

		{#if showFilterButton}
			<button class="filter-btn" on:click={handleFilterButtonClick} aria-label="More filters">
				<Filter size={16} />
			</button>
		{/if}
	</div>
</div>

<style>
	.filter-bar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem 1.5rem;
		border-bottom: 1px solid #e5e7eb;
		gap: 1rem;
		flex-wrap: wrap;
	}

	.filter-bar-left {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		flex: 1;
		flex-wrap: wrap;
	}

	.filter-bar-right {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.filter-select {
		padding: 0.5rem 0.75rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		background: white;
		color: #374151;
		cursor: pointer;
		transition: all 0.15s;
	}

	.filter-select:hover {
		border-color: #d1d5db;
	}

	.filter-select:focus {
		outline: none;
		border-color: #189AB4;
		box-shadow: 0 0 0 3px rgba(24, 154, 180, 0.1);
	}

	.filter-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 36px;
		height: 36px;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		background: white;
		color: #6b7280;
		cursor: pointer;
		transition: all 0.15s;
	}

	.filter-btn:hover {
		background: #f9fafb;
		border-color: #d1d5db;
		color: #374151;
	}

	@media (max-width: 768px) {
		.filter-bar {
			padding: 0.75rem 1rem;
		}

		.filter-bar-left {
			width: 100%;
		}

		.filter-select {
			flex: 1;
			min-width: 120px;
		}
	}
</style>
