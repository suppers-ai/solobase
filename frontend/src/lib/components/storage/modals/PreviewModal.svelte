<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { Download, ExternalLink } from 'lucide-svelte';
	import { formatFileSize, formatDateShort } from '$lib/utils/formatters';
	import Modal from '$lib/components/ui/Modal.svelte';

	export let show = false;
	export let item: any = null;
	export let previewUrl = '';

	const dispatch = createEventDispatcher();

	$: fileType = getFileType(item?.objectName || '');
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

<Modal {show} title={item?.objectName || ''} maxWidth="900px" on:close={handleClose}>
	<svelte:fragment slot="default">
		<div class="header-actions">
			<button class="icon-button" title="Open in new tab" on:click={handleOpenExternal}>
				<ExternalLink size={20} />
			</button>
			<button class="icon-button" title="Download" on:click={handleDownload}>
				<Download size={20} />
			</button>
		</div>

		<div class="preview-body">
			{#if isPreviewable && previewUrl}
				{#if fileType === 'image'}
					<div class="preview-container">
						<img src={previewUrl} alt={item?.objectName} />
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
						<iframe src={previewUrl} title={item?.objectName}></iframe>
					</div>
				{:else if fileType === 'text'}
					<div class="preview-container text">
						<iframe src={previewUrl} title={item?.objectName}></iframe>
					</div>
				{/if}
			{:else}
				<div class="no-preview">
					<p>Preview not available for this file type</p>
					<button class="modal-btn modal-btn-primary" on:click={handleDownload}>
						<Download size={16} />
						Download File
					</button>
				</div>
			{/if}
		</div>

		{#if item}
			<div class="file-info">
				<span>Size: {formatFileSize(item.size)}</span>
				<span>Modified: {formatDateShort(item.lastModified)}</span>
			</div>
		{/if}
	</svelte:fragment>
</Modal>

<style>
	.header-actions {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1rem;
	}

	.preview-body {
		background: #f9fafb;
		border-radius: 0.375rem;
		min-height: 300px;
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

	.file-info {
		display: flex;
		gap: 1.5rem;
		font-size: 0.875rem;
		color: #6b7280;
		margin-top: 1rem;
		padding-top: 1rem;
		border-top: 1px solid #e5e7eb;
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
