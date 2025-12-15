<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { Upload, X, File as FileIcon } from 'lucide-svelte';
	import { formatFileSize } from '$lib/utils/formatters';
	import Modal from '$lib/components/ui/Modal.svelte';

	export let show = false;
	export let bucketName = '';
	export let currentPath = '';
	export let uploading = false;
	export let uploadProgress = 0;
	export let fileUploadProgress: Map<string, number> = new Map();

	const dispatch = createEventDispatcher();

	let selectedFiles: File[] = [];
	let fileInputRef: HTMLInputElement;

	function handleFileSelect(e: Event) {
		const target = e.target as HTMLInputElement;
		if (target.files) {
			selectedFiles = Array.from(target.files);
		}
	}

	function removeFile(index: number) {
		selectedFiles = selectedFiles.filter((_, i) => i !== index);
	}

	function handleUpload() {
		if (selectedFiles.length > 0) {
			dispatch('upload', { files: selectedFiles });
		}
	}

	function handleClose() {
		selectedFiles = [];
		if (fileInputRef) {
			fileInputRef.value = '';
		}
		dispatch('close');
	}
</script>

<Modal {show} title="Upload Files" maxWidth="600px" on:close={handleClose}>
	<p class="upload-info">
		Uploading to: <strong>{bucketName}/{currentPath || 'root'}</strong>
	</p>

	<div class="upload-area">
		<input
			type="file"
			multiple
			on:change={handleFileSelect}
			bind:this={fileInputRef}
			disabled={uploading}
			id="file-upload"
			class="file-input"
		/>
		<label for="file-upload" class="upload-label">
			<Upload size={48} />
			<h4>Click to select files</h4>
			<p>or drag and drop files here</p>
		</label>
	</div>

	{#if selectedFiles.length > 0}
		<div class="selected-files">
			<h4>Selected Files ({selectedFiles.length})</h4>
			<div class="file-list">
				{#each selectedFiles as file, index}
					<div class="file-item">
						<div class="file-info">
							<FileIcon size={16} />
							<span class="file-name">{file.name}</span>
							<span class="file-size">({formatFileSize(file.size)})</span>
						</div>
						{#if fileUploadProgress.has(file.name)}
							<div class="progress-bar">
								<div
									class="progress-fill"
									style="width: {fileUploadProgress.get(file.name)}%"
								></div>
							</div>
						{:else if !uploading}
							<button
								class="remove-button"
								on:click={() => removeFile(index)}
							>
								<X size={16} />
							</button>
						{/if}
					</div>
				{/each}
			</div>
		</div>
	{/if}

	{#if uploading}
		<div class="upload-progress">
			<p>Uploading... {uploadProgress}%</p>
			<div class="progress-bar">
				<div class="progress-fill" style="width: {uploadProgress}%"></div>
			</div>
		</div>
	{/if}

	<svelte:fragment slot="footer">
		<button class="modal-btn modal-btn-secondary" on:click={handleClose} disabled={uploading}>
			Cancel
		</button>
		<button
			class="modal-btn modal-btn-primary"
			on:click={handleUpload}
			disabled={selectedFiles.length === 0 || uploading}
		>
			{uploading ? 'Uploading...' : `Upload ${selectedFiles.length} file(s)`}
		</button>
	</svelte:fragment>
</Modal>

<style>
	.upload-info {
		margin: 0 0 1rem 0;
		color: #6b7280;
		font-size: 0.875rem;
	}

	.upload-area {
		position: relative;
		margin-bottom: 1.5rem;
	}

	.file-input {
		position: absolute;
		width: 0.1px;
		height: 0.1px;
		opacity: 0;
		overflow: hidden;
		z-index: -1;
	}

	.upload-label {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 3rem 2rem;
		border: 2px dashed #d1d5db;
		border-radius: 0.5rem;
		cursor: pointer;
		transition: all 0.2s;
		text-align: center;
	}

	.upload-label:hover {
		border-color: #189AB4;
		background: #f0f9ff;
	}

	.upload-label h4 {
		margin: 1rem 0 0.5rem 0;
		font-size: 1rem;
		font-weight: 600;
		color: #374151;
	}

	.upload-label p {
		margin: 0;
		color: #6b7280;
		font-size: 0.875rem;
	}

	.selected-files {
		margin-bottom: 1rem;
	}

	.selected-files h4 {
		margin: 0 0 0.75rem 0;
		font-size: 0.875rem;
		font-weight: 600;
		color: #374151;
	}

	.file-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.file-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem;
		background: #f9fafb;
		border-radius: 0.375rem;
		font-size: 0.875rem;
	}

	.file-info {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex: 1;
		min-width: 0;
	}

	.file-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.file-size {
		color: #6b7280;
		font-size: 0.75rem;
		flex-shrink: 0;
	}

	.progress-bar {
		width: 100px;
		height: 4px;
		background: #e5e7eb;
		border-radius: 2px;
		overflow: hidden;
	}

	.progress-fill {
		height: 100%;
		background: #189AB4;
		transition: width 0.3s ease;
	}

	.remove-button {
		padding: 0.25rem;
		background: transparent;
		border: none;
		border-radius: 0.25rem;
		cursor: pointer;
		color: #6b7280;
		transition: all 0.2s;
	}

	.remove-button:hover {
		background: #fee2e2;
		color: #dc2626;
	}

	.upload-progress {
		margin-top: 1rem;
	}

	.upload-progress p {
		margin: 0 0 0.5rem 0;
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
	}
</style>
