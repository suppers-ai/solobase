<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { X, Download, ExternalLink } from 'lucide-svelte';
	import { formatFileSize, formatDateShort } from '$lib/utils/formatters';

	export let show = false;
	export let item: any = null;
	export let previewUrl = '';

	const dispatch = createEventDispatcher();

	$: fileType = getFileType(item?.object_name || '');
	$: isPreviewable = ['image', 'video', 'audio', 'pdf', 'text'].includes(fileType);

	function getFileType(fileName: string): string {
		const ext = fileName.split('.').pop()?.toLowerCase() || '';
		
		const imageExts = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg', 'bmp'];
		const videoExts = ['mp4', 'webm', 'ogg'];
		const audioExts = ['mp3', 'wav', 'ogg', 'm4a'];
		const textExts = ['txt', 'md', 'json', 'js', 'ts', 'html', 'css', 'xml', 'yaml', 'yml'];
		
		if (imageExts.includes(ext)) return 'image';
		if (videoExts.includes(ext)) return 'video';
		if (audioExts.includes(ext)) return 'audio';
		if (ext === 'pdf') return 'pdf';
		if (textExts.includes(ext)) return 'text';
		return 'unknown';
	}

	function handleClose() {
		dispatch('close');
	}

	function handleDownload() {
		dispatch('download');
	}

	function handleOpenExternal() {
		if (previewUrl) {
			window.open(previewUrl, '_blank');
		}
	}
</script>

{#if show && item}
	<div class="modal-overlay" on:click={handleClose}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h3>{item.object_name}</h3>
				<div class="header-actions">
					<button class="icon-button" title="Open in new tab" on:click={handleOpenExternal}>
						<ExternalLink size={20} />
					</button>
					<button class="icon-button" title="Download" on:click={handleDownload}>
						<Download size={20} />
					</button>
					<button class="icon-button" on:click={handleClose}>
						<X size={20} />
					</button>
				</div>
			</div>

			<div class="modal-body">
				{#if isPreviewable && previewUrl}
					{#if fileType === 'image'}
						<div class="preview-container">
							<img src={previewUrl} alt={item.object_name} />
						</div>
					{:else if fileType === 'video'}
						<div class="preview-container">
							<video controls>
								<source src={previewUrl} />
								Your browser does not support the video tag.
							</video>
						</div>
					{:else if fileType === 'audio'}
						<div class="preview-container audio">
							<audio controls>
								<source src={previewUrl} />
								Your browser does not support the audio tag.
							</audio>
						</div>
					{:else if fileType === 'pdf'}
						<div class="preview-container pdf">
							<iframe src={previewUrl} title={item.object_name}></iframe>
						</div>
					{:else if fileType === 'text'}
						<div class="preview-container text">
							<iframe src={previewUrl} title={item.object_name}></iframe>
						</div>
					{/if}
				{:else}
					<div class="no-preview">
						<p>Preview not available for this file type</p>
						<button class="btn btn-primary" on:click={handleDownload}>
							<Download size={16} />
							Download File
						</button>
					</div>
				{/if}
			</div>

			<div class="modal-footer">
				<div class="file-info">
					<span>Size: {formatFileSize(item.size)}</span>
					<span>Modified: {formatDateShort(item.last_modified)}</span>
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
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
		z-index: 1000;
	}

	.modal {
		background: white;
		border-radius: 0.5rem;
		width: 90%;
		max-width: 900px;
		max-height: 90vh;
		display: flex;
		flex-direction: column;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem 1.5rem;
		border-bottom: 1px solid #e5e7eb;
	}

	.modal-header h3 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		flex: 1;
		margin-right: 1rem;
	}

	.header-actions {
		display: flex;
		gap: 0.5rem;
	}

	.modal-body {
		flex: 1;
		overflow: auto;
		background: #f9fafb;
	}

	.preview-container {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 400px;
		padding: 1rem;
	}

	.preview-container img {
		max-width: 100%;
		max-height: 600px;
		object-fit: contain;
	}

	.preview-container video {
		max-width: 100%;
		max-height: 600px;
	}

	.preview-container.audio {
		min-height: 200px;
	}

	.preview-container.pdf iframe,
	.preview-container.text iframe {
		width: 100%;
		height: 600px;
		border: none;
	}

	.no-preview {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		min-height: 300px;
		text-align: center;
	}

	.no-preview p {
		margin: 0 0 1.5rem 0;
		color: #6b7280;
		font-size: 1rem;
	}

	.modal-footer {
		padding: 1rem 1.5rem;
		border-top: 1px solid #e5e7eb;
	}

	.file-info {
		display: flex;
		gap: 1.5rem;
		font-size: 0.875rem;
		color: #6b7280;
	}

	.btn {
		padding: 0.5rem 1rem;
		border: none;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
	}

	.btn-primary {
		background: #189AB4;
		color: white;
	}

	.btn-primary:hover {
		background: #157a8f;
	}

	.icon-button {
		padding: 0.375rem;
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

