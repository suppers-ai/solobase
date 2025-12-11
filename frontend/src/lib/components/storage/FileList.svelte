<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import {
		Folder, File, Image, FileText, Film,
		Music, Archive, Code, MoreVertical,
		Download, Eye, Edit2, Trash2, Copy, Move
	} from 'lucide-svelte';
	import { formatFileSize, formatDateShort } from '$lib/utils/formatters';

	export let files: any[] = [];
	export let viewMode: 'grid' | 'list' = 'grid';
	export let selectedItems: Set<string> = new Set();
	export let loadingFiles = false;

	const dispatch = createEventDispatcher();

	function getFileIcon(type: string) {
		switch(type) {
			case 'folder': return Folder;
			case 'image': return Image;
			case 'pdf': return FileText;
			case 'video': return Film;
			case 'audio': return Music;
			case 'archive': return Archive;
			case 'code': return Code;
			default: return File;
		}
	}

	function getFileIconColor(type: string) {
		switch(type) {
			case 'folder': return '#f59e0b';
			case 'image': return '#10b981';
			case 'pdf': return '#ef4444';
			case 'video': return '#8b5cf6';
			case 'audio': return '#ec4899';
			case 'archive': return '#6366f1';
			case 'code': return '#14b8a6';
			default: return '#6b7280';
		}
	}

	function getFileExtension(fileName: string): string {
		const parts = fileName.split('.');
		return parts.length > 1 ? parts[parts.length - 1].toLowerCase() : '';
	}

	function determineFileType(fileName: string): string {
		const ext = getFileExtension(fileName);
		const imageExts = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg', 'bmp'];
		const videoExts = ['mp4', 'avi', 'mov', 'wmv', 'flv', 'mkv', 'webm'];
		const audioExts = ['mp3', 'wav', 'ogg', 'flac', 'aac', 'm4a'];
		const archiveExts = ['zip', 'rar', '7z', 'tar', 'gz', 'bz2'];
		const codeExts = ['js', 'ts', 'jsx', 'tsx', 'html', 'css', 'py', 'go', 'rs', 'c', 'cpp'];

		if (imageExts.includes(ext)) return 'image';
		if (videoExts.includes(ext)) return 'video';
		if (audioExts.includes(ext)) return 'audio';
		if (archiveExts.includes(ext)) return 'archive';
		if (codeExts.includes(ext)) return 'code';
		if (ext === 'pdf') return 'pdf';
		return 'file';
	}

	function handleSelect(item: any) {
		dispatch('select', item);
	}

	function handleOpen(item: any) {
		dispatch('open', item);
	}

	function handleAction(action: string, item: any) {
		dispatch('action', { action, item });
	}

	function toggleSelection(item: any) {
		const newSelection = new Set(selectedItems);
		if (newSelection.has(item.id)) {
			newSelection.delete(item.id);
		} else {
			newSelection.add(item.id);
		}
		dispatch('selectionChange', newSelection);
	}
</script>

{#if loadingFiles}
	<div class="loading">
		<div class="spinner"></div>
		<p>Loading files...</p>
	</div>
{:else if files.length === 0}
	<div class="empty-state">
		<File size={48} color="#9ca3af" />
		<h3>No files yet</h3>
		<p>Upload some files to get started</p>
	</div>
{:else if viewMode === 'grid'}
	<div class="file-grid">
		{#each files as item}
			{@const fileType = item.is_folder ? 'folder' : determineFileType(item.object_name)}
			{@const IconComponent = getFileIcon(fileType)}
			{@const iconColor = getFileIconColor(fileType)}

			<div
				class="file-card"
				class:selected={selectedItems.has(item.id)}
				on:dblclick={() => handleOpen(item)}
			>
				<div class="file-card-header">
					<input
						type="checkbox"
						checked={selectedItems.has(item.id)}
						on:click|stopPropagation={() => toggleSelection(item)}
					/>
					<button
						class="icon-button"
						on:click|stopPropagation={() => dispatch('menu', item)}
					>
						<MoreVertical size={16} />
					</button>
				</div>

				<div class="file-icon">
					<IconComponent size={48} color={iconColor} />
				</div>

				<div class="file-info">
					<p class="file-name" title={item.object_name}>{item.object_name}</p>
					<p class="file-meta">
						{#if !item.is_folder}
							{formatFileSize(item.size)} •
						{/if}
						{formatDateShort(item.last_modified)}
					</p>
				</div>
			</div>
		{/each}
	</div>
{:else}
	<div class="file-list">
		<table>
			<thead>
				<tr>
					<th style="width: 40px">
						<input
							type="checkbox"
							checked={selectedItems.size === files.length && files.length > 0}
							on:change={(e) => {
								if (e.currentTarget.checked) {
									dispatch('selectionChange', new Set(files.map(f => f.id)));
								} else {
									dispatch('selectionChange', new Set());
								}
							}}
						/>
					</th>
					<th>Name</th>
					<th style="width: 100px">Size</th>
					<th style="width: 120px">Modified</th>
					<th style="width: 100px">Actions</th>
				</tr>
			</thead>
			<tbody>
				{#each files as item}
					{@const fileType = item.is_folder ? 'folder' : determineFileType(item.object_name)}
					{@const IconComponent = getFileIcon(fileType)}
					{@const iconColor = getFileIconColor(fileType)}

					<tr
						class:selected={selectedItems.has(item.id)}
						on:dblclick={() => handleOpen(item)}
					>
						<td>
							<input
								type="checkbox"
								checked={selectedItems.has(item.id)}
								on:click|stopPropagation={() => toggleSelection(item)}
							/>
						</td>
						<td>
							<div class="file-name-cell">
								<IconComponent size={20} color={iconColor} />
								<span title={item.object_name}>{item.object_name}</span>
							</div>
						</td>
						<td>
							{#if !item.is_folder}
								{formatFileSize(item.size)}
							{:else}
								-
							{/if}
						</td>
						<td>{formatDateShort(item.last_modified)}</td>
						<td>
							<div class="action-buttons">
								{#if !item.is_folder}
									<button
										class="icon-button"
										title="Preview"
										on:click|stopPropagation={() => handleAction('preview', item)}
									>
										<Eye size={16} />
									</button>
									<button
										class="icon-button"
										title="Download"
										on:click|stopPropagation={() => handleAction('download', item)}
									>
										<Download size={16} />
									</button>
								{/if}
								<button
									class="icon-button"
									title="More"
									on:click|stopPropagation={() => dispatch('menu', item)}
								>
									<MoreVertical size={16} />
								</button>
							</div>
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
{/if}

<style>
	.loading {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 4rem 2rem;
		color: #6b7280;
	}

	.spinner {
		width: 40px;
		height: 40px;
		border: 3px solid #e5e7eb;
		border-top-color: #189AB4;
		border-radius: 50%;
		animation: spin 1s linear infinite;
		margin-bottom: 1rem;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 4rem 2rem;
		text-align: center;
		color: #6b7280;
	}

	.empty-state h3 {
		margin: 1rem 0 0.5rem 0;
		font-size: 1.25rem;
		font-weight: 600;
		color: #374151;
	}

	.empty-state p {
		margin: 0;
		font-size: 0.875rem;
	}

	/* Grid View */
	.file-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
		gap: 1rem;
		padding: 1rem;
	}

	.file-card {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		padding: 1rem;
		cursor: pointer;
		transition: all 0.2s;
		position: relative;
	}

	.file-card:hover {
		border-color: #189AB4;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
	}

	.file-card.selected {
		background: #e0f2fe;
		border-color: #189AB4;
	}

	.file-card-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.5rem;
	}

	.file-icon {
		display: flex;
		justify-content: center;
		padding: 1.5rem 0;
	}

	.file-info {
		text-align: center;
	}

	.file-name {
		margin: 0;
		font-size: 0.875rem;
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.file-meta {
		margin: 0.25rem 0 0 0;
		font-size: 0.75rem;
		color: #6b7280;
	}

	/* List View */
	.file-list {
		background: white;
		border-radius: 0.5rem;
		overflow: hidden;
	}

	table {
		width: 100%;
		border-collapse: collapse;
	}

	th {
		text-align: left;
		padding: 0.75rem 1rem;
		background: #f9fafb;
		border-bottom: 1px solid #e5e7eb;
		font-size: 0.875rem;
		font-weight: 600;
		color: #374151;
	}

	td {
		padding: 0.75rem 1rem;
		border-bottom: 1px solid #f3f4f6;
		font-size: 0.875rem;
	}

	tr:hover {
		background: #f9fafb;
	}

	tr.selected {
		background: #e0f2fe;
	}

	.file-name-cell {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.action-buttons {
		display: flex;
		gap: 0.25rem;
	}

	.icon-button {
		padding: 0.25rem;
		background: transparent;
		border: none;
		border-radius: 0.25rem;
		cursor: pointer;
		color: #6b7280;
		transition: all 0.2s;
	}

	.icon-button:hover {
		background: #f3f4f6;
		color: #374151;
	}
</style>