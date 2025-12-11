<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { Table, Folder, FolderOpen, ChevronDown, RefreshCw } from 'lucide-svelte';

	export let tables: any[] = [];
	export let selectedTable = '';
	export let groupedTables: Map<string, any[]> = new Map();
	export let expandedGroups: Set<string> = new Set();
	export let loading = false;

	const dispatch = createEventDispatcher();

	function selectTable(tableName: string) {
		dispatch('select', tableName);
	}

	function toggleGroup(groupName: string) {
		const newExpanded = new Set(expandedGroups);
		if (newExpanded.has(groupName)) {
			newExpanded.delete(groupName);
		} else {
			newExpanded.add(groupName);
		}
		dispatch('toggleGroup', newExpanded);
	}

	function refresh() {
		dispatch('refresh');
	}

	$: sortedGroups = Array.from(groupedTables.entries()).sort((a, b) => {
		// Put "Authentication" first, then "Other" last, rest alphabetically
		if (a[0] === 'Authentication') return -1;
		if (b[0] === 'Authentication') return 1;
		if (a[0] === 'Other') return 1;
		if (b[0] === 'Other') return -1;
		return a[0].localeCompare(b[0]);
	});
</script>

<div class="sidebar">
	<div class="sidebar-header">
		<h3>Tables</h3>
		<button 
			class="icon-button" 
			on:click={refresh}
			class:spinning={loading}
			title="Refresh tables"
		>
			<RefreshCw size={16} />
		</button>
	</div>

	<div class="tables-list">
		{#if sortedGroups.length === 0}
			<div class="no-tables">
				<p>No tables found</p>
			</div>
		{:else}
			{#each sortedGroups as [groupName, groupTables]}
				<div class="table-group">
					<button
						class="group-header"
						on:click={() => toggleGroup(groupName)}
					>
						<div class="group-header-content">
							{#if expandedGroups.has(groupName)}
								<FolderOpen size={16} />
							{:else}
								<Folder size={16} />
							{/if}
							<span class="group-name">{groupName}</span>
							<span class="group-count">({groupTables.length})</span>
						</div>
						<ChevronDown 
							size={14} 
							class="chevron {expandedGroups.has(groupName) ? 'expanded' : ''}"
						/>
					</button>

					{#if expandedGroups.has(groupName)}
						<div class="group-tables">
							{#each groupTables as table}
								<button
									class="table-item"
									class:active={selectedTable === (table.name || table.value)}
									on:click={() => selectTable(table.name || table.value)}
								>
									<Table size={14} />
									<span class="table-name">{table.name || table.value}</span>
									{#if table.rowsCount !== undefined}
										<span class="table-rows">{table.rowsCount.toLocaleString()} rows</span>
									{/if}
								</button>
							{/each}
						</div>
					{/if}
				</div>
			{/each}
		{/if}
	</div>
</div>

<style>
	.sidebar {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		height: 100%;
		display: flex;
		flex-direction: column;
	}

	.sidebar-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		border-bottom: 1px solid #e5e7eb;
	}

	.sidebar-header h3 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
	}

	.icon-button {
		padding: 0.375rem;
		background: transparent;
		border: none;
		border-radius: 0.25rem;
		cursor: pointer;
		color: #6b7280;
		transition: all 0.2s;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.icon-button:hover {
		background: #f3f4f6;
		color: #374151;
	}

	.icon-button.spinning {
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		from { transform: rotate(0deg); }
		to { transform: rotate(360deg); }
	}

	.tables-list {
		flex: 1;
		overflow-y: auto;
		padding: 0.5rem;
	}

	.no-tables {
		padding: 2rem;
		text-align: center;
		color: #6b7280;
		font-size: 0.875rem;
	}

	.table-group {
		margin-bottom: 0.25rem;
	}

	.group-header {
		width: 100%;
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.5rem;
		background: transparent;
		border: none;
		border-radius: 0.375rem;
		cursor: pointer;
		text-align: left;
		transition: all 0.2s;
	}

	.group-header:hover {
		background: #f3f4f6;
	}

	.group-header-content {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex: 1;
	}

	.group-name {
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
	}

	.group-count {
		font-size: 0.75rem;
		color: #6b7280;
	}

	:global(.chevron) {
		transition: transform 0.2s;
		color: #6b7280;
	}

	:global(.chevron.expanded) {
		transform: rotate(180deg);
	}

	.group-tables {
		padding-left: 1.5rem;
	}

	.table-item {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem;
		background: transparent;
		border: none;
		border-radius: 0.25rem;
		cursor: pointer;
		text-align: left;
		transition: all 0.2s;
		font-size: 0.875rem;
	}

	.table-item:hover {
		background: #f3f4f6;
	}

	.table-item.active {
		background: #e0f2fe;
		color: #0369a1;
	}

	.table-name {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.table-rows {
		font-size: 0.75rem;
		color: #6b7280;
	}
</style>