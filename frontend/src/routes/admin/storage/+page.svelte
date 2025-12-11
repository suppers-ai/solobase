<script lang="ts">
	import { onMount } from 'svelte';
	import { api, ErrorHandler } from '$lib/api';
	import { requireAdmin } from '$lib/utils/auth';
	import { formatBytes } from '$lib/utils/formatters';

	// Import all our new components
	import StorageStats from '$lib/components/storage/StorageStats.svelte';
	import BucketSelector from '$lib/components/storage/BucketSelector.svelte';
	import Breadcrumb from '$lib/components/storage/Breadcrumb.svelte';
	import Toolbar from '$lib/components/storage/Toolbar.svelte';
	import FileList from '$lib/components/storage/FileList.svelte';
	import ContextMenu from '$lib/components/storage/ContextMenu.svelte';

	// Import modal components
	import CreateBucketModal from '$lib/components/storage/modals/CreateBucketModal.svelte';
	import CreateFolderModal from '$lib/components/storage/modals/CreateFolderModal.svelte';
	import UploadModal from '$lib/components/storage/modals/UploadModal.svelte';
	import DeleteModal from '$lib/components/storage/modals/DeleteModal.svelte';
	import RenameModal from '$lib/components/storage/modals/RenameModal.svelte';
	import PreviewModal from '$lib/components/storage/modals/PreviewModal.svelte';

	// State management
	let viewMode: 'grid' | 'list' = 'grid';
	let selectedBucket: any = null;
	let currentPath = '';
	let currentFolderId: string | null = null;
	let selectedItems = new Set<string>();
	let loadingFiles = false;
	let refreshing = false;

	// Modal states
	let showCreateBucketModal = false;
	let showCreateFolderModal = false;
	let showUploadModal = false;
	let showDeleteModal = false;
	let showRenameModal = false;
	let showPreviewModal = false;
	let showContextMenu = false;

	// Context menu position
	let contextMenuX = 0;
	let contextMenuY = 0;
	let contextMenuItem: any = null;

	// Modal data
	let itemsToDelete: any[] = [];
	let itemToRename: any = null;
	let previewItem: any = null;
	let previewUrl = '';

	// Upload states
	let uploadingFiles = false;
	let uploadProgress = 0;
	let fileUploadProgress: Map<string, number> = new Map();

	// Data from API
	let buckets: any[] = [];
	let files: any[] = [];

	// Stats
	let totalStorage = '10 GB';
	let usedStorage = '0 B';
	let totalFiles = 0;
	let totalBuckets = 0;

	onMount(async () => {
		await requireAdmin();
		await fetchBuckets();
	});

	// API Functions
	async function fetchBuckets() {
		try {
			const response = await api.get('/storage/buckets');
			if (Array.isArray(response)) {
				buckets = response;
			} else {
				buckets = [];
			}
			totalBuckets = buckets.length;

			// Calculate stats
			let filesCount = 0;
			let storageUsed = 0;
			buckets.forEach(bucket => {
				filesCount += bucket.ObjectCount || bucket.files || 0;
				storageUsed += bucket.TotalSize || bucket.sizeBytes || 0;
			});
			totalFiles = filesCount;
			usedStorage = formatBytes(storageUsed);

			// Auto-select first bucket
			if (buckets.length > 0 && !selectedBucket) {
				await selectBucket(buckets[0]);
			}
		} catch (error) {
			console.error('Failed to fetch buckets:', error);
			buckets = [];
		}
	}

	interface StorageObjectsResponse {
		objects?: unknown[];
	}

	async function fetchBucketObjects(bucketName: string) {
		loadingFiles = true;
		files = [];

		try {
			const url = currentFolderId
				? `/storage/buckets/${bucketName}/objects?parent_folder_id=${encodeURIComponent(currentFolderId)}`
				: `/storage/buckets/${bucketName}/objects`;

			const response = await api.get<unknown[] | StorageObjectsResponse>(url);

			if (Array.isArray(response)) {
				files = processFiles(response);
			} else if (response && 'objects' in response && Array.isArray(response.objects)) {
				files = processFiles(response.objects);
			} else {
				files = [];
			}
		} catch (error) {
			console.error('Failed to fetch objects:', error);
			files = [];
		} finally {
			loadingFiles = false;
		}
	}

	function processFiles(rawFiles: any[]): any[] {
		return rawFiles.map(file => ({
			...file,
			id: file.id || file.objectId,
			isFolder: file.type === 'folder' || file.objectName?.endsWith('/'),
			objectName: file.objectName || file.name,
			size: file.sizeBytes || file.size || 0,
			lastModified: file.lastModified || file.updatedAt || new Date().toISOString()
		}));
	}

	async function selectBucket(bucket: any) {
		selectedBucket = bucket;
		currentPath = '';
		currentFolderId = null;
		selectedItems.clear();
		await fetchBucketObjects(bucket.name);
	}

	async function navigateToPath(path: string) {
		currentPath = path;
		// In a real implementation, would need to map path to folder ID
		await fetchBucketObjects(selectedBucket.name);
	}

	async function openFolder(folder: any) {
		if (!folder.isFolder) return;

		currentPath = currentPath ? `${currentPath}/${folder.objectName}` : folder.objectName;
		currentFolderId = folder.id;
		selectedItems.clear();
		await fetchBucketObjects(selectedBucket.name);
	}

	// Create operations
	async function createBucket(event: CustomEvent) {
		const { name, public: isPublic } = event.detail;

		try {
			await api.post('/storage/buckets', { name, public: isPublic });
			await fetchBuckets();
			showCreateBucketModal = false;
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}

	async function createFolder(event: CustomEvent) {
		const { name } = event.detail;

		try {
			const fullPath = currentPath ? `${currentPath}/${name}/` : `${name}/`;
			await api.post(`/storage/buckets/${selectedBucket.name}/objects`, {
				objectName: fullPath,
				type: 'folder',
				parentFolderId: currentFolderId
			});
			await fetchBucketObjects(selectedBucket.name);
			showCreateFolderModal = false;
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}

	// Upload operations
	async function handleUpload(event: CustomEvent) {
		const { files: filesToUpload } = event.detail;
		if (!filesToUpload || filesToUpload.length === 0) return;

		uploadingFiles = true;
		uploadProgress = 0;
		fileUploadProgress.clear();

		try {
			const totalSize = filesToUpload.reduce((sum: number, file: File) => sum + file.size, 0);
			let uploadedSize = 0;

			for (const file of filesToUpload) {
				const formData = new FormData();
				formData.append('file', file);
				if (currentFolderId) {
					formData.append('parentFolderId', currentFolderId);
				}

				// Track individual file progress
				fileUploadProgress.set(file.name, 0);
				fileUploadProgress = fileUploadProgress; // Trigger reactivity

				const xhr = new XMLHttpRequest();

				// Track upload progress
				xhr.upload.onprogress = (e) => {
					if (e.lengthComputable) {
						const fileProgress = (e.loaded / e.total) * 100;
						fileUploadProgress.set(file.name, fileProgress);
						fileUploadProgress = fileUploadProgress;

						uploadedSize += e.loaded;
						uploadProgress = Math.round((uploadedSize / totalSize) * 100);
					}
				};

				// Promisify XHR
				await new Promise((resolve, reject) => {
					xhr.onload = () => {
						if (xhr.status >= 200 && xhr.status < 300) {
							resolve(xhr.response);
						} else {
							reject(new Error(`Upload failed: ${xhr.statusText}`));
						}
					};
					xhr.onerror = () => reject(new Error('Upload failed'));

					// Token is stored in httpOnly cookie
					xhr.open('POST', `/api/storage/buckets/${selectedBucket.name}/upload`);
					xhr.send(formData);
				});

				fileUploadProgress.set(file.name, 100);
				fileUploadProgress = fileUploadProgress;
			}

			await fetchBucketObjects(selectedBucket.name);
			showUploadModal = false;
		} catch (error) {
			ErrorHandler.handle(error);
		} finally {
			uploadingFiles = false;
			uploadProgress = 0;
			fileUploadProgress.clear();
		}
	}

	// Delete operations
	async function deleteItems() {
		if (itemsToDelete.length === 0) return;

		try {
			for (const item of itemsToDelete) {
				await api.delete(`/storage/buckets/${selectedBucket.name}/objects/${item.id}`);
			}

			await fetchBucketObjects(selectedBucket.name);
			selectedItems.clear();
			showDeleteModal = false;
			itemsToDelete = [];
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}

	// Rename operations
	async function renameItem(event: CustomEvent) {
		const { newName } = event.detail;
		if (!itemToRename) return;

		try {
			await api.patch(`/storage/buckets/${selectedBucket.name}/objects/${itemToRename.id}/rename`, {
				newName
			});

			await fetchBucketObjects(selectedBucket.name);
			showRenameModal = false;
			itemToRename = null;
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}

	// Download operations
	async function downloadFile(file: any) {
		if (file.isFolder) return;

		try {
			// Token is stored in httpOnly cookie
			const response = await fetch(`/api/storage/buckets/${selectedBucket.name}/objects/${file.id}/download`);

			if (response.ok) {
				const blob = await response.blob();
				const url = window.URL.createObjectURL(blob);
				const a = document.createElement('a');
				a.href = url;
				a.download = file.objectName;
				document.body.appendChild(a);
				a.click();
				document.body.removeChild(a);
				window.URL.revokeObjectURL(url);
			}
		} catch (error) {
			console.error('Download failed:', error);
		}
	}

	async function downloadSelectedItems() {
		const itemsToDownload = files.filter(f => selectedItems.has(f.id) && !f.isFolder);
		for (const item of itemsToDownload) {
			await downloadFile(item);
		}
	}

	// Preview operations
	async function showPreview(file: any) {
		if (file.isFolder) return;

		previewItem = file;

		// Generate preview URL based on file type
		// Token is stored in httpOnly cookie
		previewUrl = `/api/storage/buckets/${selectedBucket.name}/objects/${file.id}/download`;

		showPreviewModal = true;
	}

	// Event handlers
	function handleToolbarAction(event: CustomEvent) {
		const action = event.detail;

		switch (action) {
			case 'upload':
				showUploadModal = true;
				break;
			case 'createFolder':
				showCreateFolderModal = true;
				break;
			case 'download':
				downloadSelectedItems();
				break;
			case 'delete':
				itemsToDelete = files.filter(f => selectedItems.has(f.id));
				showDeleteModal = true;
				break;
			case 'refresh':
				refreshFiles();
				break;
		}
	}

	async function refreshFiles() {
		refreshing = true;
		await fetchBucketObjects(selectedBucket.name);
		refreshing = false;
	}

	function handleFileAction(event: CustomEvent) {
		const { action, item } = event.detail;

		switch (action) {
			case 'preview':
				showPreview(item);
				break;
			case 'download':
				downloadFile(item);
				break;
			case 'rename':
				itemToRename = item;
				showRenameModal = true;
				break;
			case 'delete':
				itemsToDelete = [item];
				showDeleteModal = true;
				break;
		}
	}

	function handleFileOpen(event: CustomEvent) {
		const item = event.detail;
		if (item.isFolder) {
			openFolder(item);
		} else {
			showPreview(item);
		}
	}

	function handleFileMenu(event: CustomEvent) {
		const item = event.detail;
		const target = event.target as HTMLElement | null;
		const rect = target?.getBoundingClientRect() || { right: 0, bottom: 0 };

		contextMenuItem = item;
		contextMenuX = rect.right;
		contextMenuY = rect.bottom;
		showContextMenu = true;
	}

	function handleContextMenuAction(event: CustomEvent) {
		const { action, item } = event.detail;
		handleFileAction(new CustomEvent('action', { detail: { action, item } }));
		showContextMenu = false;
	}

	</script>

<div class="storage-container">
	<StorageStats
		{totalStorage}
		{usedStorage}
		{totalFiles}
		{totalBuckets}
	/>

	<div class="storage-content">
		<div class="sidebar">
			<BucketSelector
				{buckets}
				{selectedBucket}
				on:select={(e) => selectBucket(e.detail)}
				on:create={() => showCreateBucketModal = true}
			/>
		</div>

		<div class="main-content">
			{#if selectedBucket}
				<Breadcrumb
					{currentPath}
					on:navigate={(e) => navigateToPath(e.detail)}
				/>

				<Toolbar
					selectedCount={selectedItems.size}
					{viewMode}
					{refreshing}
					on:action={handleToolbarAction}
					on:viewChange={(e) => viewMode = e.detail}
				/>

				<FileList
					{files}
					{viewMode}
					{selectedItems}
					{loadingFiles}
					on:select={handleFileOpen}
					on:open={handleFileOpen}
					on:action={handleFileAction}
					on:menu={handleFileMenu}
					on:selectionChange={(e) => selectedItems = e.detail}
				/>
			{:else}
				<div class="no-bucket-selected">
					<p>Select a bucket to view files</p>
				</div>
			{/if}
		</div>
	</div>
</div>

<!-- Modals -->
<CreateBucketModal
	show={showCreateBucketModal}
	on:create={createBucket}
	on:close={() => showCreateBucketModal = false}
/>

<CreateFolderModal
	show={showCreateFolderModal}
	{currentPath}
	on:create={createFolder}
	on:close={() => showCreateFolderModal = false}
/>

<UploadModal
	show={showUploadModal}
	bucketName={selectedBucket?.name || ''}
	{currentPath}
	uploading={uploadingFiles}
	{uploadProgress}
	{fileUploadProgress}
	on:upload={handleUpload}
	on:close={() => showUploadModal = false}
/>

<DeleteModal
	show={showDeleteModal}
	items={itemsToDelete}
	deleting={false}
	on:confirm={deleteItems}
	on:close={() => {
		showDeleteModal = false;
		itemsToDelete = [];
	}}
/>

<RenameModal
	show={showRenameModal}
	item={itemToRename}
	renaming={false}
	on:rename={renameItem}
	on:close={() => {
		showRenameModal = false;
		itemToRename = null;
	}}
/>

<PreviewModal
	show={showPreviewModal}
	item={previewItem}
	{previewUrl}
	on:download={() => downloadFile(previewItem)}
	on:close={() => {
		showPreviewModal = false;
		previewItem = null;
		previewUrl = '';
	}}
/>

<ContextMenu
	show={showContextMenu}
	x={contextMenuX}
	y={contextMenuY}
	item={contextMenuItem}
	on:action={handleContextMenuAction}
	on:close={() => showContextMenu = false}
/>

<style>
	.storage-container {
		height: 100%;
		display: flex;
		flex-direction: column;
	}

	.storage-content {
		flex: 1;
		display: flex;
		gap: 1.5rem;
		overflow: hidden;
	}

	.sidebar {
		width: 300px;
		flex-shrink: 0;
	}

	.main-content {
		flex: 1;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}

	.no-bucket-selected {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		color: #6b7280;
	}
</style>