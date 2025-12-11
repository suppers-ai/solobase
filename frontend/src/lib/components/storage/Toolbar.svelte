<script lang="ts">
	import { 
		Upload, FolderPlus, Download, Trash2, 
		Copy, Move, Grid3x3, List, RefreshCw 
	} from 'lucide-svelte';
	import { createEventDispatcher } from 'svelte';

	export let selectedCount = 0;
	export let viewMode: 'grid' | 'list' = 'grid';
	export let refreshing = false;

	const dispatch = createEventDispatcher();

	function handleAction(action: string) {
		dispatch('action', action);
	}

	function handleViewChange(mode: 'grid' | 'list') {
		dispatch('viewChange', mode);
	}
</script>

<div class="toolbar">
	<div class="toolbar-left">
		<button class="btn btn-primary" on:click={() => handleAction('upload')}>
			<Upload size={16} />
			Upload
		</button>
		<button class="btn btn-secondary" on:click={() => handleAction('createFolder')}>
			<FolderPlus size={16} />
			New Folder
		</button>

		{#if selectedCount > 0}
			<div class="divider"></div>
			<span class="selected-count">{selectedCount} selected</span>
			<button 
				class="btn btn-secondary" 
				on:click={() => handleAction('download')}
				title="Download selected"
			>
				<Download size={16} />
			</button>
			<button 
				class="btn btn-secondary" 
				on:click={() => handleAction('copy')}
				title="Copy selected"
			>
				<Copy size={16} />
			</button>
			<button 
				class="btn btn-secondary" 
				on:click={() => handleAction('move')}
				title="Move selected"
			>
				<Move size={16} />
			</button>
			<button 
				class="btn btn-danger" 
				on:click={() => handleAction('delete')}
				title="Delete selected"
			>
				<Trash2 size={16} />
			</button>
		{/if}
	</div>

	<div class="toolbar-right">
		<button 
			class="btn-icon" 
			on:click={() => handleAction('refresh')}
			class:spinning={refreshing}
			title="Refresh"
		>
			<RefreshCw size={18} />
		</button>
		<div class="view-toggle">
			<button 
				class="view-btn" 
				class:active={viewMode === 'grid'}
				on:click={() => handleViewChange('grid')}
				title="Grid view"
			>
				<Grid3x3 size={18} />
			</button>
			<button 
				class="view-btn" 
				class:active={viewMode === 'list'}
				on:click={() => handleViewChange('list')}
				title="List view"
			>
				<List size={18} />
			</button>
		</div>
	</div>
</div>

<style>
	.toolbar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		background: white;
		border-bottom: 1px solid #e5e7eb;
	}

	.toolbar-left {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.toolbar-right {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.divider {
		width: 1px;
		height: 24px;
		background: #e5e7eb;
		margin: 0 0.5rem;
	}

	.selected-count {
		font-size: 0.875rem;
		color: #6b7280;
		font-weight: 500;
	}

	.btn {
		padding: 0.375rem 0.75rem;
		border: none;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-primary {
		background: #189AB4;
		color: white;
	}

	.btn-primary:hover:not(:disabled) {
		background: #157a8f;
	}

	.btn-secondary {
		background: #f3f4f6;
		color: #374151;
	}

	.btn-secondary:hover:not(:disabled) {
		background: #e5e7eb;
	}

	.btn-danger {
		background: #fee2e2;
		color: #dc2626;
	}

	.btn-danger:hover:not(:disabled) {
		background: #fecaca;
	}

	.btn-icon {
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

	.btn-icon:hover {
		background: #f3f4f6;
		color: #374151;
	}

	.btn-icon.spinning {
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		from { transform: rotate(0deg); }
		to { transform: rotate(360deg); }
	}

	.view-toggle {
		display: flex;
		background: #f3f4f6;
		border-radius: 0.375rem;
		padding: 2px;
	}

	.view-btn {
		padding: 0.25rem 0.5rem;
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

	.view-btn:hover {
		color: #374151;
	}

	.view-btn.active {
		background: white;
		color: #111827;
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
	}
</style>