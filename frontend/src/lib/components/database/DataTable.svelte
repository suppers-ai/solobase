<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { ChevronLeft, ChevronRight, Download } from 'lucide-svelte';
	import LoadingSpinner from '$lib/components/ui/LoadingSpinner.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';

	export let data: any[] = [];
	export let columns: any[] = [];
	export let loading = false;
	export let currentPage = 1;
	export let totalPages = 1;
	export let totalRows = 0;
	export let rowsPerPage = 25;

	const dispatch = createEventDispatcher();

	function changePage(page: number) {
		if (page >= 1 && page <= totalPages) {
			dispatch('pageChange', page);
		}
	}

	function exportData() {
		dispatch('export');
	}

	function formatCellValue(value: any): string {
		if (value === null) return 'NULL';
		if (value === undefined) return '';
		if (typeof value === 'boolean') return value ? 'true' : 'false';
		if (typeof value === 'object') {
			try {
				return JSON.stringify(value);
			} catch {
				return '[Object]';
			}
		}
		return String(value);
	}

	function getCellClass(value: any): string {
		if (value === null) return 'null-value';
		if (typeof value === 'boolean') return 'boolean-value';
		if (typeof value === 'number') return 'number-value';
		if (typeof value === 'object') return 'json-value';
		return '';
	}
</script>

<div class="data-table-container">
	<div class="table-header">
		<div class="table-info">
			<span>Showing {(currentPage - 1) * rowsPerPage + 1} - {Math.min(currentPage * rowsPerPage, totalRows)} of {totalRows} rows</span>
		</div>
		<button class="export-button" on:click={exportData}>
			<Download size={16} />
			Export
		</button>
	</div>

	{#if loading}
		<div class="loading">
			<LoadingSpinner size="lg" />
			<p>Loading data...</p>
		</div>
	{:else if data.length === 0}
		<EmptyState message="No data available" compact />
	{:else}
		<div class="table-wrapper">
			<table>
				<thead>
					<tr>
						{#each columns as column}
							<th>
								<div class="column-header">
									<span>{column.name}</span>
									<span class="column-type">{column.type}</span>
								</div>
							</th>
						{/each}
					</tr>
				</thead>
				<tbody>
					{#each data as row}
						<tr>
							{#each columns as column}
								<td class={getCellClass(row[column.name])}>
									{formatCellValue(row[column.name])}
								</td>
							{/each}
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}

	{#if totalPages > 1}
		<div class="pagination">
			<button 
				class="page-button"
				disabled={currentPage === 1}
				on:click={() => changePage(currentPage - 1)}
				aria-label="Previous page"
			>
				<ChevronLeft size={16} />
			</button>

			<div class="page-numbers">
				{#if currentPage > 2}
					<button class="page-button" on:click={() => changePage(1)}>1</button>
					{#if currentPage > 3}
						<span class="page-dots">...</span>
					{/if}
				{/if}

				{#if currentPage > 1}
					<button class="page-button" on:click={() => changePage(currentPage - 1)}>
						{currentPage - 1}
					</button>
				{/if}

				<button class="page-button active">{currentPage}</button>

				{#if currentPage < totalPages}
					<button class="page-button" on:click={() => changePage(currentPage + 1)}>
						{currentPage + 1}
					</button>
				{/if}

				{#if currentPage < totalPages - 1}
					{#if currentPage < totalPages - 2}
						<span class="page-dots">...</span>
					{/if}
					<button class="page-button" on:click={() => changePage(totalPages)}>
						{totalPages}
					</button>
				{/if}
			</div>

			<button 
				class="page-button"
				disabled={currentPage === totalPages}
				on:click={() => changePage(currentPage + 1)}
				aria-label="Next page"
			>
				<ChevronRight size={16} />
			</button>
		</div>
	{/if}
</div>

<style>
	.data-table-container {
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.table-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		border-bottom: 1px solid #e5e7eb;
	}

	.table-info {
		font-size: 0.875rem;
		color: #6b7280;
	}

	.export-button {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		background: #189AB4;
		color: white;
		border: none;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.export-button:hover {
		background: #157a8f;
	}

	.loading {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		color: #6b7280;
	}


	.table-wrapper {
		flex: 1;
		overflow: auto;
	}

	table {
		width: 100%;
		border-collapse: collapse;
	}

	th {
		position: sticky;
		top: 0;
		background: #f9fafb;
		border-bottom: 1px solid #e5e7eb;
		padding: 0.75rem;
		text-align: left;
		font-size: 0.875rem;
		font-weight: 600;
		color: #374151;
	}

	.column-header {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.column-type {
		font-size: 0.75rem;
		font-weight: 400;
		color: #6b7280;
	}

	td {
		padding: 0.75rem;
		border-bottom: 1px solid #f3f4f6;
		font-size: 0.875rem;
	}

	.null-value {
		color: #9ca3af;
		font-style: italic;
	}

	.boolean-value {
		color: #059669;
	}

	.number-value {
		color: #0891b2;
	}

	.json-value {
		color: #7c3aed;
		font-family: monospace;
		font-size: 0.75rem;
	}

	.pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		padding: 1rem;
		border-top: 1px solid #e5e7eb;
	}

	.page-button {
		padding: 0.375rem 0.75rem;
		background: white;
		border: 1px solid #d1d5db;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		color: #374151;
		cursor: pointer;
		transition: all 0.2s;
	}

	.page-button:hover:not(:disabled) {
		background: #f3f4f6;
	}

	.page-button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.page-button.active {
		background: #189AB4;
		color: white;
		border-color: #189AB4;
	}

	.page-numbers {
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}

	.page-dots {
		padding: 0 0.5rem;
		color: #6b7280;
	}
</style>