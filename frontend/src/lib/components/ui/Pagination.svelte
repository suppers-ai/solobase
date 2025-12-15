<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { ChevronLeft, ChevronRight } from 'lucide-svelte';

	export let currentPage = 1;
	export let totalPages = 1;
	export let totalItems = 0;
	export let pageSize = 25;
	export let showInfo = true;
	export let maxVisiblePages = 5;

	const dispatch = createEventDispatcher();

	$: startItem = totalItems > 0 ? (currentPage - 1) * pageSize + 1 : 0;
	$: endItem = Math.min(currentPage * pageSize, totalItems);

	function changePage(page: number) {
		if (page >= 1 && page <= totalPages && page !== currentPage) {
			dispatch('change', page);
		}
	}

	function getVisiblePages(current: number, total: number, max: number): (number | '...')[] {
		if (total <= max) {
			return Array.from({ length: total }, (_, i) => i + 1);
		}

		const pages: (number | '...')[] = [];
		const half = Math.floor(max / 2);

		// Always show first page
		pages.push(1);

		// Calculate start and end of middle section
		let start = Math.max(2, current - half + 1);
		let end = Math.min(total - 1, current + half - 1);

		// Adjust if we're near the beginning
		if (current <= half + 1) {
			end = Math.min(total - 1, max - 1);
		}

		// Adjust if we're near the end
		if (current >= total - half) {
			start = Math.max(2, total - max + 2);
		}

		// Add ellipsis after first page if needed
		if (start > 2) {
			pages.push('...');
		}

		// Add middle pages
		for (let i = start; i <= end; i++) {
			pages.push(i);
		}

		// Add ellipsis before last page if needed
		if (end < total - 1) {
			pages.push('...');
		}

		// Always show last page
		if (total > 1) {
			pages.push(total);
		}

		return pages;
	}

	$: visiblePages = getVisiblePages(currentPage, totalPages, maxVisiblePages);
</script>

{#if totalPages > 0}
	<div class="pagination">
		{#if showInfo}
			<div class="pagination-info">
				{#if totalItems > 0}
					Showing {startItem} to {endItem} of {totalItems}
				{:else}
					No items
				{/if}
			</div>
		{/if}

		{#if totalPages > 1}
			<div class="pagination-controls">
				<button
					class="page-btn"
					disabled={currentPage === 1}
					on:click={() => changePage(currentPage - 1)}
					aria-label="Previous page"
				>
					<ChevronLeft size={16} />
				</button>

				<div class="page-numbers">
					{#each visiblePages as page}
						{#if page === '...'}
							<span class="page-dots">...</span>
						{:else}
							<button
								class="page-btn"
								class:active={currentPage === page}
								on:click={() => changePage(page)}
							>
								{page}
							</button>
						{/if}
					{/each}
				</div>

				<button
					class="page-btn"
					disabled={currentPage === totalPages}
					on:click={() => changePage(currentPage + 1)}
					aria-label="Next page"
				>
					<ChevronRight size={16} />
				</button>
			</div>
		{/if}
	</div>
{/if}

<style>
	.pagination {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		border-top: 1px solid #e5e7eb;
		background: white;
	}

	.pagination-info {
		font-size: 0.875rem;
		color: #64748b;
	}

	.pagination-controls {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.page-numbers {
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}

	.page-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		min-width: 32px;
		height: 32px;
		padding: 0 0.5rem;
		background: white;
		border: 1px solid #d1d5db;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
		cursor: pointer;
		transition: all 0.15s;
	}

	.page-btn:hover:not(:disabled):not(.active) {
		background: #f3f4f6;
		border-color: #9ca3af;
	}

	.page-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.page-btn.active {
		background: #189AB4;
		color: white;
		border-color: #189AB4;
	}

	.page-dots {
		padding: 0 0.375rem;
		color: #6b7280;
		font-size: 0.875rem;
	}

	@media (max-width: 640px) {
		.pagination {
			flex-direction: column;
			gap: 0.75rem;
		}

		.pagination-info {
			order: 2;
		}

		.pagination-controls {
			order: 1;
		}
	}
</style>
