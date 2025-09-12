<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { ChevronRight, ChevronDown, X } from 'lucide-svelte';
	
	export let files: any[] = [];
	export let selectedFile: any = null;
	export let loading = false;
	export let mode: 'file' | 'folder' | 'both' = 'both';
	export let showModal = false;
	export let title = 'Select File or Folder';
	
	const dispatch = createEventDispatcher();
	
	let expandedFolders: Set<string> = new Set();
	
	// Debug logging
	$: if (files.length > 0) {
		console.log('FileExplorer received files:', files);
		console.log('FileExplorer selectedFile:', selectedFile);
		files.forEach(f => {
			const isSelected = selectedFile && (selectedFile.id === f.id || selectedFile.path === f.path);
			console.log(`File ${f.name} (id: ${f.id}): selected=${isSelected}`);
		});
	}
	
	function getFileEmoji(file: any): string {
		if (file.type === 'directory') return '📁';
		
		const name = file.name?.toLowerCase() || '';
		const extension = name.split('.').pop() || '';
		
		// Check for specific file types
		if (name.endsWith('.md') || name.endsWith('.markdown')) return '📝';
		if (name.endsWith('.html') || name.endsWith('.htm')) return '🌐';
		if (name.endsWith('.css') || name.endsWith('.scss') || name.endsWith('.sass')) return '🎨';
		if (name.endsWith('.js') || name.endsWith('.ts') || name.endsWith('.jsx') || name.endsWith('.tsx')) return '📜';
		if (name.endsWith('.json')) return '📋';
		if (name.endsWith('.toml') || name.endsWith('.yaml') || name.endsWith('.yml')) return '⚙️';
		if (name.endsWith('.jpg') || name.endsWith('.jpeg') || name.endsWith('.png') || name.endsWith('.gif') || name.endsWith('.svg')) return '🖼️';
		if (name.endsWith('.mp4') || name.endsWith('.avi') || name.endsWith('.mov') || name.endsWith('.webm')) return '🎬';
		if (name.endsWith('.mp3') || name.endsWith('.wav') || name.endsWith('.ogg')) return '🎵';
		if (name.endsWith('.pdf')) return '📕';
		if (name.endsWith('.zip') || name.endsWith('.tar') || name.endsWith('.gz')) return '📦';
		
		// Default file emoji
		return '📄';
	}
	
	function toggleFolder(path: string) {
		if (expandedFolders.has(path)) {
			expandedFolders.delete(path);
		} else {
			expandedFolders.add(path);
		}
		expandedFolders = expandedFolders;
	}
	
	function isItemSelected(item: any): boolean {
		if (!selectedFile) return false;
		return selectedFile.id === item.id || (selectedFile.path && selectedFile.path === item.path);
	}
	
	function selectItem(item: any) {
		// In file mode, clicking folders should toggle expansion, not select
		if (mode === 'file' && item.type === 'directory') {
			toggleFolder(item.path);
			return;
		}
		
		// In folder mode, only allow selecting folders
		if (mode === 'folder' && item.type !== 'directory') return;
		
		selectedFile = item;
		dispatch('select', item);
		
		if (showModal) {
			dispatch('confirm', item);
		}
	}
	
	function handleCancel() {
		selectedFile = null;
		dispatch('cancel');
	}
	
	function handleConfirm() {
		if (selectedFile) {
			dispatch('confirm', selectedFile);
		}
	}
	
	function renderTree(nodes: any[], level = 0): any[] {
		return nodes.map(node => ({
			...node,
			level,
			children: node.children ? renderTree(node.children, level + 1) : []
		}));
	}
	
	$: flatTree = renderTree(files);
</script>

{#if showModal}
	<div class="modal-overlay" on:click={handleCancel}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h3>{title}</h3>
				<button class="close-button" on:click={handleCancel}>
					<X size={20} />
				</button>
			</div>
			<div class="modal-body">
				<div class="file-explorer">
					{#if loading}
						<div class="loading">Loading files...</div>
					{:else if files.length === 0}
						<div class="empty">No files found</div>
					{:else}
						<div class="file-tree">
							{#each files as node}
								<div class="file-node">
									{#if node.type === 'directory'}
										<div class="folder-node">
											<button
												class="folder-toggle"
												on:click={() => toggleFolder(node.path)}
											>
												{#if expandedFolders.has(node.path)}
													<ChevronDown size={16} />
												{:else}
													<ChevronRight size={16} />
												{/if}
											</button>
											<button 
												class="node-item folder"
												class:selected={isItemSelected(node)}
												on:click={() => selectItem(node)}
											>
												<span class="file-emoji">{getFileEmoji(node)}</span>
												<span>{node.name}</span>
											</button>
										</div>
										{#if expandedFolders.has(node.path) && node.children}
											<div class="folder-children">
												{#each node.children as child}
													<svelte:self 
														files={[child]} 
														bind:selectedFile
														{mode}
														showModal={false}
														on:select
													/>
												{/each}
											</div>
										{/if}
									{:else}
										<button 
											class="node-item file"
											class:selected={isItemSelected(node)}
											on:click={() => selectItem(node)}
											disabled={mode === 'folder'}
										>
											<span class="file-emoji">{getFileEmoji(node)}</span>
											<span>{node.name}</span>
										</button>
									{/if}
								</div>
							{/each}
						</div>
					{/if}
				</div>
			</div>
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={handleCancel}>Cancel</button>
				<button 
					class="btn btn-primary" 
					on:click={handleConfirm}
					disabled={!selectedFile}
				>
					Select {mode === 'folder' ? 'Folder' : mode === 'file' ? 'File' : 'Item'}
				</button>
			</div>
		</div>
	</div>
{:else}
	<div class="file-explorer">
		{#if loading}
			<div class="loading">Loading files...</div>
		{:else if files.length === 0}
			<div class="empty">No files found</div>
		{:else}
			<div class="file-tree">
				{#each files as node}
					<div class="file-node">
						{#if node.type === 'directory'}
							<div class="folder-node">
								<button
									class="folder-toggle"
									on:click={() => toggleFolder(node.path)}
								>
									{#if expandedFolders.has(node.path)}
										<ChevronDown size={16} />
									{:else}
										<ChevronRight size={16} />
									{/if}
								</button>
								<button 
									class="node-item folder"
									class:selected={isItemSelected(node)}
									on:click={() => selectItem(node)}
								>
									<span class="file-emoji">{getFileEmoji(node)}</span>
									<span>{node.name}</span>
								</button>
							</div>
							{#if expandedFolders.has(node.path) && node.children}
								<div class="folder-children">
									{#each node.children as child}
										<svelte:self 
											files={[child]} 
											bind:selectedFile
											{mode}
											showModal={false}
											on:select
										/>
									{/each}
								</div>
							{/if}
						{:else}
							<button 
								class="node-item file"
								class:selected={isItemSelected(node)}
								on:click={() => selectItem(node)}
								disabled={mode === 'folder'}
							>
								<span class="file-emoji">{getFileEmoji(node)}</span>
								<span>{node.name}</span>
							</button>
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	</div>
{/if}

<style>
	.file-explorer {
		width: 100%;
		height: 100%;
		overflow-y: auto;
		padding: 0.25rem;
	}
	
	.file-tree {
		font-size: 0.875rem;
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}
	
	.file-node {
		margin-bottom: 0.125rem;
	}
	
	.folder-node {
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}
	
	.folder-toggle {
		background: none;
		border: none;
		padding: 0.125rem;
		cursor: pointer;
		color: var(--text-secondary);
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 0.25rem;
		transition: background 0.2s;
	}
	
	.folder-toggle:hover {
		background: var(--bg-hover);
	}
	
	.node-item {
		flex: 1;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.25rem 0.5rem;
		background: none;
		border: none;
		border-radius: 0.25rem;
		cursor: pointer;
		color: var(--text-primary);
		text-align: left;
		transition: background-color 0.15s;
		font-size: 0.8125rem;
		line-height: 1.4;
		font-family: inherit;
	}
	
	.node-item:not(.selected) {
		background: transparent;
		border: none;
	}
	
	.node-item:hover:not(:disabled):not(.selected) {
		background-color: rgba(0, 0, 0, 0.05);
	}
	
	.node-item:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	
	.node-item.folder {
		font-weight: 500;
	}
	
	.node-item.selected {
		background-color: rgba(6, 182, 212, 0.1);
		color: var(--text-primary);
		box-shadow: inset 0 0 0 1px rgba(6, 182, 212, 0.3);
	}
	
	.node-item.selected:hover {
		background-color: rgba(6, 182, 212, 0.15);
		color: var(--text-primary);
	}
	
	/* Ensure folder and file classes don't override */
	.node-item.folder,
	.node-item.file {
		background: inherit;
		border: inherit;
	}
	
	.file-emoji {
		font-size: 1rem;
		line-height: 1;
		display: inline-block;
		width: 1.25rem;
		text-align: center;
	}
	
	.folder-children {
		margin-left: 1.25rem;
		margin-top: 0.125rem;
		padding-left: 0.75rem;
		border-left: 1px solid rgba(0, 0, 0, 0.1);
	}
	
	.loading, .empty {
		padding: 2rem;
		text-align: center;
		color: var(--text-secondary);
	}
	
	/* Modal styles */
	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 10000;
	}
	
	.modal {
		background: var(--bg-primary);
		border-radius: 0.75rem;
		width: 90%;
		max-width: 600px;
		max-height: 70vh;
		display: flex;
		flex-direction: column;
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
		z-index: 10001;
		position: relative;
	}
	
	.modal-header {
		padding: 1.25rem 1.5rem;
		border-bottom: 1px solid var(--border-color);
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	
	.modal-header h3 {
		margin: 0;
		font-size: 1.125rem;
		font-weight: 600;
		color: var(--text-primary);
	}
	
	.close-button {
		background: none;
		border: none;
		color: var(--text-secondary);
		cursor: pointer;
		padding: 0.25rem;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 0.375rem;
		transition: all 0.2s;
	}
	
	.close-button:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
	}
	
	.modal-body {
		flex: 1;
		padding: 1.5rem;
		overflow-y: auto;
		min-height: 300px;
	}
	
	.modal-footer {
		padding: 1rem 1.5rem;
		border-top: 1px solid var(--border-color);
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
	}
	
	.btn {
		padding: 0.5rem 1rem;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		border: none;
	}
	
	.btn-primary {
		background: var(--primary);
		color: white;
	}
	
	.btn-primary:hover:not(:disabled) {
		opacity: 0.9;
	}
	
	.btn-primary:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	
	.btn-secondary {
		background: transparent;
		color: var(--text-primary);
		border: 1px solid var(--border-color);
	}
	
	.btn-secondary:hover {
		background: var(--bg-hover);
	}
</style>