<script lang="ts">
	import { GripVertical, XCircle, Plus, X, PlusCircle, Edit2 } from 'lucide-svelte';
	import { createEventDispatcher } from 'svelte';
	
	export let selectedIds: string[] = [];
	export let availableItems: Array<{
		id: string;
		name: string;
		displayName: string;
		description?: string;
		category?: string;
		[key: string]: any;
	}> = [];
	export let title: string = 'Items';
	export let helpText: string = 'Items will be applied in the order shown. Drag to reorder.';
	export let emptyMessage: string = 'No items selected';
	export let noItemsMessage: string = 'No items available.';
	export let addButtonText: string = 'Add Item';
	export let createLink: string | null = null;
	export let createLinkText: string = 'Create items first';
	export let allowCreateNew: boolean = false;
	export let allowEdit: boolean = false;
	
	let showSelector = false;
	const dispatch = createEventDispatcher();
	
	// Get the actual item object from an ID
	function getItem(id: string) {
		return availableItems.find(item => item.id === id);
	}
	
	// Get items that haven't been selected yet
	$: unselectedItems = availableItems.filter(item => !selectedIds?.includes(item.id));
	
	// Handle drag and drop
	function handleDragStart(event: DragEvent, index: number) {
		if (event.dataTransfer) {
			event.dataTransfer.effectAllowed = 'move';
			event.dataTransfer.setData('text/plain', index.toString());
		}
	}
	
	function handleDragOver(event: DragEvent) {
		event.preventDefault();
		if (event.dataTransfer) {
			event.dataTransfer.dropEffect = 'move';
		}
	}
	
	function handleDrop(event: DragEvent, toIndex: number) {
		event.preventDefault();
		if (event.dataTransfer) {
			const fromIndex = parseInt(event.dataTransfer.getData('text/plain'));
			if (fromIndex !== toIndex) {
				const items = [...selectedIds];
				const [removed] = items.splice(fromIndex, 1);
				items.splice(toIndex, 0, removed);
				selectedIds = items;
			}
		}
	}
	
	// Add an item to the selection
	function addItem(itemId: string) {
		if (!selectedIds) {
			selectedIds = [];
		}
		selectedIds = [...selectedIds, itemId];
		showSelector = false;
	}
	
	// Remove an item from the selection
	function removeItem(itemId: string) {
		selectedIds = selectedIds.filter(id => id !== itemId);
	}
	
	// Handle create new
	function handleCreateNew() {
		showSelector = false;
		dispatch('createNew');
	}
	
	// Handle edit item
	function handleEditItem(itemId: string) {
		const item = getItem(itemId);
		if (item) {
			dispatch('editItem', { item });
		}
	}
</script>

<div class="reorderable-list">
	<label>{title}</label>
	<p class="help-text">{helpText}</p>
	
	<div class="list-manager">
		{#if selectedIds && selectedIds.length > 0}
			<div class="selected-list">
				{#each selectedIds as itemId, index}
					{@const item = getItem(itemId)}
					{#if item}
						<div class="selected-item"
							draggable="true"
							on:dragstart={(e) => handleDragStart(e, index)}
							on:dragover={handleDragOver}
							on:drop={(e) => handleDrop(e, index)}>
							<div class="drag-handle">
								<GripVertical size={16} />
							</div>
							<div class="item-order">{index + 1}</div>
							<div class="item-content">
								<div class="item-name">{item.displayName || item.name}</div>
								<div class="item-meta">
									{#if item.category}
										<span class="item-category">{item.category}</span>
									{/if}
									{#if item.description}
										{#if item.category}
											<span class="item-separator">•</span>
										{/if}
										<span class="item-description">{item.description}</span>
									{/if}
								</div>
							</div>
							<div class="item-actions">
								{#if allowEdit}
									<button class="edit-btn" 
										on:click={() => handleEditItem(itemId)}
										title="Edit item"
										type="button">
										<Edit2 size={16} />
									</button>
								{/if}
								<button class="remove-btn" 
									on:click={() => removeItem(itemId)}
									title="Remove item"
									type="button">
									<XCircle size={16} />
								</button>
							</div>
						</div>
					{/if}
				{/each}
			</div>
		{:else}
			<div class="no-items-selected">
				<p>{emptyMessage}</p>
			</div>
		{/if}
		
		{#if availableItems.length > 0}
			<div class="add-item-section">
				<button type="button" class="btn-add-item" 
					on:click={() => showSelector = !showSelector}>
					<Plus size={16} />
					{addButtonText}
				</button>
				
				{#if showSelector}
					<div class="item-selector">
						<div class="selector-header">
							<span class="selector-title">Select {title}</span>
							<button type="button" class="close-selector" on:click={() => showSelector = false}>
								<X size={16} />
							</button>
						</div>
						
						{#if allowCreateNew}
							<div class="create-new-option" 
								on:click={handleCreateNew}
								on:keypress={(e) => e.key === 'Enter' && handleCreateNew()}
								role="button"
								tabindex="0">
								<PlusCircle size={16} />
								<span>Create New {title.slice(0, -1)}</span>
							</div>
						{/if}
						
						{#if unselectedItems.length > 0}
							{#each unselectedItems as item}
								<div class="item-option"
									on:click={() => addItem(item.id)}
									on:keypress={(e) => e.key === 'Enter' && addItem(item.id)}
									role="button"
									tabindex="0">
									<div class="item-option-content">
										<div class="item-name">{item.displayName || item.name}</div>
										<div class="item-meta">
											{#if item.category}
												<span class="item-category">{item.category}</span>
											{/if}
											{#if item.description}
												{#if item.category}
													<span class="item-separator">•</span>
												{/if}
												<span class="item-description">{item.description}</span>
											{/if}
										</div>
									</div>
								</div>
							{/each}
						{:else}
							<p class="no-more-items">All items have been added</p>
						{/if}
					</div>
				{/if}
			</div>
		{:else}
			<p class="no-items">
				{noItemsMessage}
				{#if createLink}
					<a href={createLink}>{createLinkText}</a>.
				{/if}
			</p>
		{/if}
	</div>
</div>

<style>
	.reorderable-list {
		display: flex;
		flex-direction: column;
	}
	
	.reorderable-list label {
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
		margin-bottom: 0.25rem;
	}

	.help-text {
		font-size: 0.875rem;
		color: #6b7280;
		margin: 0 0 0.75rem 0;
	}

	.list-manager {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.selected-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.selected-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		cursor: move;
		transition: all 0.2s;
	}

	.selected-item:hover {
		background: #f0fdf4;
		border-color: #86efac;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
	}

	.selected-item:active {
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.15);
	}

	.drag-handle {
		color: #9ca3af;
		cursor: grab;
		flex-shrink: 0;
		transition: color 0.2s;
	}

	.drag-handle:active {
		cursor: grabbing;
	}

	.selected-item:hover .drag-handle {
		color: #6b7280;
	}

	.item-order {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 24px;
		height: 24px;
		background: #f3f4f6;
		color: #6b7280;
		font-size: 0.75rem;
		font-weight: 600;
		border-radius: 50%;
		flex-shrink: 0;
		transition: all 0.2s;
	}

	.selected-item:hover .item-order {
		background: #d1fae5;
		color: #065f46;
	}

	.item-content {
		flex: 1;
		min-width: 0;
	}

	.item-name {
		font-weight: 500;
		color: #111827;
		margin-bottom: 0.25rem;
		transition: color 0.2s;
	}

	.selected-item:hover .item-name {
		color: #059669;
	}

	.item-meta {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.75rem;
		color: #6b7280;
	}

	.item-category {
		padding: 0.125rem 0.375rem;
		background: #f3f4f6;
		border-radius: 0.25rem;
		font-weight: 500;
		transition: all 0.2s;
	}

	.selected-item:hover .item-category {
		background: #ecfdf5;
		color: #047857;
	}

	.item-separator {
		color: #d1d5db;
	}

	.item-description {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	
	.item-actions {
		display: flex;
		gap: 0.25rem;
		flex-shrink: 0;
	}

	.edit-btn,
	.remove-btn {
		padding: 0.25rem;
		background: transparent;
		border: none;
		color: #9ca3af;
		cursor: pointer;
		transition: all 0.2s;
		flex-shrink: 0;
		border-radius: 0.25rem;
	}
	
	.edit-btn:hover {
		color: #3b82f6;
		background: #dbeafe;
	}

	.remove-btn:hover {
		color: #ef4444;
		background: #fee2e2;
	}

	.no-items-selected {
		padding: 2rem;
		text-align: center;
		background: #f9fafb;
		border: 1px dashed #d1d5db;
		border-radius: 0.375rem;
	}

	.no-items-selected p {
		margin: 0;
		color: #6b7280;
	}

	.add-item-section {
		position: relative;
	}

	.btn-add-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		background: white;
		border: 1px dashed #d1d5db;
		border-radius: 0.375rem;
		color: #6b7280;
		font-size: 0.875rem;
		cursor: pointer;
		transition: all 0.2s;
		width: 100%;
		justify-content: center;
	}

	.btn-add-item:hover {
		border-color: #9ca3af;
		color: #4b5563;
		background: #f9fafb;
	}

	.item-selector {
		position: absolute;
		top: calc(100% + 0.5rem);
		left: 0;
		right: 0;
		max-height: 300px;
		overflow-y: auto;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
		z-index: 10;
	}
	
	.selector-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem;
		border-bottom: 1px solid #e5e7eb;
		background: #f9fafb;
		position: sticky;
		top: 0;
		z-index: 1;
	}
	
	.selector-title {
		font-size: 0.875rem;
		font-weight: 600;
		color: #374151;
	}
	
	.close-selector {
		padding: 0.25rem;
		background: transparent;
		border: none;
		color: #6b7280;
		cursor: pointer;
		transition: all 0.2s;
		border-radius: 0.25rem;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	
	.close-selector:hover {
		background: #e5e7eb;
		color: #374151;
	}
	
	.create-new-option {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem;
		background: linear-gradient(to right, #f0fdf4, #dcfce7);
		border-bottom: 2px solid #10b981;
		cursor: pointer;
		color: #059669;
		font-weight: 500;
		transition: all 0.2s;
	}
	
	.create-new-option:hover {
		background: linear-gradient(to right, #dcfce7, #bbf7d0);
		color: #047857;
	}

	.item-option {
		padding: 0.75rem;
		border-bottom: 1px solid #f3f4f6;
		cursor: pointer;
		transition: all 0.2s;
		outline: none;
	}

	.item-option:hover {
		background: linear-gradient(to right, #f0fdf4, #dcfce7);
	}

	.item-option:focus {
		background: #f0fdf4;
		box-shadow: inset 0 0 0 2px #86efac;
	}

	.item-option:last-child {
		border-bottom: none;
	}

	.item-option-content {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.item-option:hover .item-name {
		color: #059669;
	}

	.item-option:hover .item-category {
		background: #d1fae5;
		color: #065f46;
	}

	.no-more-items {
		padding: 1rem;
		text-align: center;
		color: #9ca3af;
		font-size: 0.875rem;
		margin: 0;
	}

	.no-items {
		color: #6b7280;
		font-size: 0.875rem;
	}

	.no-items a {
		color: #3b82f6;
		text-decoration: none;
	}

	.no-items a:hover {
		text-decoration: underline;
	}

	/* Custom scrollbar for item selector */
	.item-selector::-webkit-scrollbar {
		width: 6px;
	}

	.item-selector::-webkit-scrollbar-track {
		background: #f3f4f6;
		border-radius: 3px;
	}

	.item-selector::-webkit-scrollbar-thumb {
		background: #d1d5db;
		border-radius: 3px;
	}

	.item-selector::-webkit-scrollbar-thumb:hover {
		background: #9ca3af;
	}
</style>