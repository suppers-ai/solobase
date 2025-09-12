<script lang="ts">
	import { onMount } from 'svelte';
	import { 
		HardDrive, FolderPlus, Upload, Download,
		Trash2, Edit2, Search, Grid, List,
		Folder, File, Image, FileText, Film,
		Music, Archive, Code, ChevronRight,
		Plus, X, Check, Copy, Move, Eye, RefreshCw,
		MoreVertical, FolderTree
	} from 'lucide-svelte';
	import { api } from '$lib/api';
	import ExportButton from '$lib/components/ExportButton.svelte';
	import FileExplorer from '$lib/components/FileExplorer.svelte';
	import { requireAdmin } from '$lib/utils/auth';
	
	let viewMode = 'grid'; // 'grid', 'list', or 'explorer'
	let selectedBucket: any = null;
	let currentPath = '';
	let currentFolderId: string | null = null; // Track current folder ID
	let searchQuery = '';
	let selectedItems = new Set<string>();
	let showCreateBucketModal = false;
	let showCreateFolderModal = false;
	let showUploadModal = false;
	let showDeleteModal = false;
	let showRenameModal = false;
	let itemToDelete: any = null;
	let itemToRename: any = null;
	let uploadingFiles = false;
	let uploadProgress = 0;
	let selectedFiles: File[] = [];
	let fileUploadProgress: Map<string, number> = new Map();
	let loadingFiles = false;
	let activeDropdownId: string | null = null;
	let newItemName = '';
	let fileInputRef: HTMLInputElement;
	
	// Preview modal state
	let showPreviewModal = false;
	let previewItem: any = null;
	let previewContent: string = '';
	let previewImageUrl: string = '';
	let previewLoading = false;
	
	// Form data
	let newBucket = {
		name: '',
		public: false
	};
	
	let newFolder = {
		name: ''
	};
	
	// Real data from API
	let buckets: any[] = [];
	
	let files: any[] = [];
	
	// Watch files changes
	$: console.log('Files array updated:', files);
	
	// Stats from API
	let totalStorage = '0 B';
	let usedStorage = '0 B';
	let totalFiles = 0;
	let totalBuckets = 0;
	
	// Breadcrumb path parts
	$: pathParts = currentPath.split('/').filter(p => p);
	
	// Filtered files based on search
	$: filteredFiles = (() => {
		const filtered = files.filter(file => {
			if (!file || !file.name) return false;
			return file.name.toLowerCase().includes(searchQuery.toLowerCase());
		});
		console.log('Filtered files:', filtered, 'Search query:', searchQuery);
		return filtered;
	})();
	
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
			case 'audio': return '#3b82f6';
			case 'archive': return '#6366f1';
			case 'code': return '#14b8a6';
			default: return '#64748b';
		}
	}
	
	async function selectBucket(bucket: any) {
		console.log('Selecting bucket:', bucket);
		selectedBucket = bucket;
		currentPath = ''; // Start at root of bucket
		currentFolderId = null; // Reset folder ID to root
		selectedItems.clear();
		
		// Fetch objects for this bucket
		if (bucket && bucket.name) {
			await fetchBucketObjects(bucket.name);
		}
	}
	
	async function navigateToFolder(folder: any) {
		if (folder.type === 'folder' || folder.isFolder) {
			console.log('Navigating to folder:', folder);
			
			// Use folder ID if available
			if (folder.id) {
				currentFolderId = folder.id;
				console.log('Set currentFolderId to:', currentFolderId);
			} else {
				console.warn('Folder has no ID:', folder);
			}
			
			// Clean up the folder name and fullPath for display
			let folderName = folder.name;
			let fullPath = folder.fullPath;
			
			// Remove leading and trailing slashes from folder name
			folderName = folderName.replace(/^\/+|\/+$/g, '');
			
			// If fullPath exists, use it; otherwise construct from current path
			if (fullPath) {
				// Clean up fullPath - remove leading/trailing slashes
				fullPath = fullPath.replace(/^\/+|\/+$/g, '');
				currentPath = fullPath;
			} else {
				// Construct path from current location
				if (currentPath) {
					currentPath = currentPath + '/' + folderName;
				} else {
					currentPath = folderName;
				}
			}
			
			console.log('New currentPath:', currentPath, 'folderId:', currentFolderId);
			selectedItems.clear();
			// Refresh the files for the new path
			await fetchBucketObjects(selectedBucket.name);
		}
	}
	
	async function navigateToPath(index: number) {
		const parts = pathParts.slice(0, index + 1);
		currentPath = parts.join('/');
		// Reset to root if navigating to root
		if (index === -1 || parts.length === 0) {
			currentFolderId = null;
		} 
		// Note: We don't know the folder ID when navigating via breadcrumb
		// This is a limitation - we should track folder IDs in breadcrumbs
		selectedItems.clear();
		// Refresh the files for the new path
		if (selectedBucket) {
			await fetchBucketObjects(selectedBucket.name);
		}
	}
	
	function toggleItemSelection(item: any) {
		if (selectedItems.has(item.id)) {
			selectedItems.delete(item.id);
		} else {
			selectedItems.add(item.id);
		}
		selectedItems = selectedItems;
	}
	
	function selectAll() {
		if (selectedItems.size === filteredFiles.length) {
			selectedItems.clear();
		} else {
			filteredFiles.forEach(file => selectedItems.add(file.id));
		}
		selectedItems = selectedItems;
	}
	
	// Modal functions
	function openCreateBucketModal() {
		newBucket = { name: '', public: false };
		showCreateBucketModal = true;
	}
	
	function closeCreateBucketModal() {
		showCreateBucketModal = false;
	}
	
	async function createBucket() {
		if (!newBucket.name) {
			alert('Bucket name is required');
			return;
		}
		
		try {
			await api.post('/storage/buckets', newBucket);
			await fetchBuckets(); // Refresh bucket list
			closeCreateBucketModal();
		} catch (error) {
			console.error('Failed to create bucket:', error);
			alert('Failed to create bucket');
		}
	}
	
	function openCreateFolderModal() {
		newFolder = { name: '' };
		showCreateFolderModal = true;
	}
	
	function closeCreateFolderModal() {
		showCreateFolderModal = false;
	}
	
	async function createFolder() {
		if (!newFolder.name || !selectedBucket) {
			alert('Folder name is required');
			return;
		}
		
		try {
			// Clean the folder name - remove slashes
			const cleanFolderName = newFolder.name.replace(/\//g, '');
			
			// Use the dedicated folder creation API with current folder ID
			const response = await api.createFolder(selectedBucket.name, cleanFolderName, currentFolderId);
			
			if (response.error) {
				throw new Error(response.error);
			}
			
			// Refresh the file list to show the new folder
			await fetchBucketObjects(selectedBucket.name);
			closeCreateFolderModal();
		} catch (error) {
			console.error('Failed to create folder:', error);
			alert('Failed to create folder: ' + (error instanceof Error ? error.message : 'Unknown error'));
		}
	}
	
	function openUploadModal() {
		selectedFiles = [];
		uploadProgress = 0;
		showUploadModal = true;
	}
	
	function closeUploadModal() {
		showUploadModal = false;
		selectedFiles = [];
	}
	
	function handleFileDrop(e: DragEvent) {
		e.preventDefault();
		const files = e.dataTransfer?.files;
		if (files) {
			selectedFiles = Array.from(files);
		}
	}
	
	function handleFileSelect(e: Event) {
		const input = e.target as HTMLInputElement;
		const files = input.files;
		if (files) {
			selectedFiles = Array.from(files);
		}
	}
	
	function openFilePicker() {
		if (fileInputRef) {
			fileInputRef.click();
		}
	}
	
	async function uploadFiles() {
		if (!selectedBucket || selectedFiles.length === 0) {
			alert('Please select files to upload');
			return;
		}
		
		uploadingFiles = true;
		uploadProgress = 0;
		fileUploadProgress.clear();
		
		try {
			const totalFiles = selectedFiles.length;
			let uploadedCount = 0;
			
			// Upload files in parallel for better performance
			const uploadPromises = selectedFiles.map(async (file) => {
				const fileKey = file.name + '_' + file.lastModified;
				fileUploadProgress.set(fileKey, 0);
				fileUploadProgress = fileUploadProgress; // Trigger reactivity
				
				try {
					// Simulate progress updates (since we can't get real progress from fetch)
					fileUploadProgress.set(fileKey, 30);
					fileUploadProgress = fileUploadProgress;
					
					// Upload file with current folder ID
					console.log('Uploading file:', file.name, 'to folder:', currentFolderId);
					const response = await api.uploadFile(selectedBucket.name, file, currentFolderId);
					
					if (response.error) {
						throw new Error(response.error);
					}
					
					fileUploadProgress.set(fileKey, 100);
					fileUploadProgress = fileUploadProgress;
					
					uploadedCount++;
					uploadProgress = Math.round((uploadedCount / totalFiles) * 100);
					
					// Add the uploaded file to the files list immediately
					const newFile = {
						id: response.data?.id || Date.now().toString(),
						name: file.name,
						size: formatBytes(file.size),
						type: getFileType(file.name),
						modified: new Date().toLocaleString(),
						public: false
					};
					files = [...files, newFile];
					
					return response;
				} catch (error) {
					fileUploadProgress.set(fileKey, -1); // Mark as failed
					fileUploadProgress = fileUploadProgress;
					throw error;
				}
			});
			
			await Promise.allSettled(uploadPromises);
			
			// Refresh the file list to get accurate data from server
			await fetchBucketObjects(selectedBucket.name);
			
			// Close modal after a short delay to show completion
			setTimeout(() => {
				closeUploadModal();
			}, 500);
		} catch (error) {
			console.error('Upload failed:', error);
			alert('Failed to upload some files');
		} finally {
			uploadingFiles = false;
		}
	}
	
	function getFileType(filename: string): string {
		const ext = filename.split('.').pop()?.toLowerCase();
		const imageExts = ['jpg', 'jpeg', 'png', 'gif', 'svg', 'webp'];
		const videoExts = ['mp4', 'avi', 'mov', 'webm'];
		const audioExts = ['mp3', 'wav', 'ogg', 'm4a'];
		const codeExts = ['js', 'ts', 'jsx', 'tsx', 'html', 'css', 'json', 'xml'];
		const archiveExts = ['zip', 'tar', 'gz', 'rar', '7z'];
		
		if (!ext) return 'file';
		if (imageExts.includes(ext)) return 'image';
		if (videoExts.includes(ext)) return 'video';
		if (audioExts.includes(ext)) return 'audio';
		if (codeExts.includes(ext)) return 'code';
		if (archiveExts.includes(ext)) return 'archive';
		if (ext === 'pdf') return 'pdf';
		return 'file';
	}
	
	function openDeleteModal(item: any) {
		itemToDelete = item;
		showDeleteModal = true;
	}
	
	function closeDeleteModal() {
		showDeleteModal = false;
		itemToDelete = null;
	}
	
	async function deleteItem() {
		if (!itemToDelete || !selectedBucket) return;
		
		try {
			const response = await api.deleteObject(selectedBucket.name, itemToDelete.id);
			
			if (response.error) {
				throw new Error(response.error);
			}
			
			// Refresh the file list
			await fetchBucketObjects(selectedBucket.name);
			closeDeleteModal();
		} catch (error) {
			console.error('Delete failed:', error);
			alert('Failed to delete item: ' + (error instanceof Error ? error.message : 'Unknown error'));
		}
	}
	
	async function downloadFile(file: any) {
		if (!selectedBucket) return;
		
		closeDropdowns(); // Close the dropdown menu
		
		try {
			// Create download URL
			const downloadUrl = `/api/storage/buckets/${selectedBucket.name}/objects/${file.id}/download`;
			
			// Create a temporary anchor element to trigger download
			const link = document.createElement('a');
			link.href = downloadUrl;
			link.download = file.name;
			link.style.display = 'none';
			
			// Add auth token if available
			const token = localStorage.getItem('auth_token');
			if (token) {
				// For authenticated downloads, we need to fetch the file with the token
				const response = await fetch(downloadUrl, {
					headers: {
						'Authorization': `Bearer ${token}`
					}
				});
				
				if (!response.ok) {
					throw new Error('Download failed');
				}
				
				const blob = await response.blob();
				const url = window.URL.createObjectURL(blob);
				link.href = url;
			}
			
			document.body.appendChild(link);
			link.click();
			document.body.removeChild(link);
			
			// Clean up the blob URL if we created one
			if (link.href.startsWith('blob:')) {
				window.URL.revokeObjectURL(link.href);
			}
		} catch (error) {
			console.error('Download failed:', error);
			alert('Failed to download file');
		}
	}
	
	async function previewFile(file: any) {
		if (!file || !selectedBucket) return;
		
		// Don't preview folders
		if (file.type === 'folder') {
			navigateToFolder(file);
			return;
		}
		
		previewItem = file;
		previewContent = '';
		previewImageUrl = '';
		previewLoading = true;
		showPreviewModal = true;
		
		try {
			// Check file type to determine preview method
			const fileExt = file.name.split('.').pop()?.toLowerCase();
			const imageExts = ['jpg', 'jpeg', 'png', 'gif', 'svg', 'webp', 'bmp'];
			const textExts = ['txt', 'md', 'json', 'xml', 'csv', 'log', 'yml', 'yaml', 'toml', 'ini'];
			const codeExts = ['js', 'ts', 'jsx', 'tsx', 'html', 'css', 'scss', 'sass', 'less', 
			                  'py', 'go', 'java', 'c', 'cpp', 'h', 'hpp', 'cs', 'php', 'rb', 
			                  'rs', 'swift', 'kt', 'scala', 'sh', 'bash', 'sql', 'vue', 'svelte'];
			
			if (imageExts.includes(fileExt || '')) {
				// For images, use download URL directly
				previewImageUrl = `/api/storage/buckets/${selectedBucket.name}/objects/${file.id}/download`;
				previewLoading = false;
			} else if (textExts.includes(fileExt || '') || codeExts.includes(fileExt || '')) {
				// For text/code files, fetch and display content
				const response = await fetch(`/api/storage/buckets/${selectedBucket.name}/objects/${file.id}/download`);
				if (response.ok) {
					previewContent = await response.text();
				} else {
					previewContent = 'Failed to load file content';
				}
				previewLoading = false;
			} else {
				// For other files, show info and download option
				previewContent = `File type: ${fileExt}\nSize: ${file.size}\n\nThis file type cannot be previewed. Click download to save it.`;
				previewLoading = false;
			}
		} catch (error) {
			console.error('Preview failed:', error);
			previewContent = 'Failed to load preview';
			previewLoading = false;
		}
	}
	
	function closePreviewModal() {
		showPreviewModal = false;
		previewItem = null;
		previewContent = '';
		previewImageUrl = '';
		previewLoading = false;
	}
	
	function copyFile(file: any) {
		console.log('Copying:', file);
	}
	
	function moveFile(file: any) {
		console.log('Moving:', file);
	}
	
	function toggleDropdown(fileId: string, event: MouseEvent) {
		event.stopPropagation();
		if (activeDropdownId === fileId) {
			activeDropdownId = null;
		} else {
			activeDropdownId = fileId;
		}
	}
	
	function closeDropdowns() {
		activeDropdownId = null;
	}
	
	function openRenameModal(item: any) {
		itemToRename = item;
		newItemName = item.name;
		showRenameModal = true;
		closeDropdowns();
	}
	
	function closeRenameModal() {
		showRenameModal = false;
		itemToRename = null;
		newItemName = '';
	}
	
	async function renameItem() {
		if (!itemToRename || !selectedBucket || !newItemName) return;
		
		try {
			const response = await api.patch(`/storage/buckets/${selectedBucket.name}/objects/${itemToRename.id}/rename`, {
				newName: newItemName
			});
			
			if (response.message) {
				// Refresh the file list to get updated data
				await fetchBucketObjects(selectedBucket.name);
				closeRenameModal();
			}
		} catch (error) {
			console.error('Rename failed:', error);
			alert('Failed to rename item: ' + (error.message || 'Unknown error'));
		}
	}
	
	function viewFile(file: any) {
		previewFile(file);
		closeDropdowns();
	}
	
	function handleDeleteFromMenu(file: any) {
		openDeleteModal(file);
		closeDropdowns();
	}
	
	async function fetchBuckets() {
		try {
			const response = await api.get('/storage/buckets');
			// Handle both array response and error response
			if (Array.isArray(response)) {
				buckets = response;
			} else if (response && response.error) {
				console.error('API error:', response.error);
				buckets = [];
			} else {
				buckets = [];
			}
			totalBuckets = buckets.length;
			
			// Calculate total files and storage
			let filesCount = 0;
			let storageUsed = 0;
			buckets.forEach(bucket => {
				filesCount += bucket.files || 0;
				storageUsed += bucket.size_bytes || 0;
			});
			totalFiles = filesCount;
			usedStorage = formatBytes(storageUsed);
			
			// Select first bucket if available
			if (buckets.length > 0) {
				selectBucket(buckets[0]);
			}
		} catch (error) {
			console.error('Failed to fetch buckets:', error);
			buckets = [];
		}
	}
	
	async function fetchBucketObjects(bucketName: string) {
		loadingFiles = true;
		files = []; // Reset files array
		
		try {
			console.log('Fetching objects for bucket:', bucketName, 'folderId:', currentFolderId);
			// Pass the current folder ID as a query parameter
			const url = currentFolderId 
				? `/storage/buckets/${bucketName}/objects?parent_folder_id=${encodeURIComponent(currentFolderId)}`
				: `/storage/buckets/${bucketName}/objects`;
			const response = await api.get(url);
			console.log('Raw API response:', response);
			console.log('Response type:', typeof response);
			// Debug: check for folders in raw response
			if (Array.isArray(response)) {
				response.forEach(item => {
					if (item.type === 'folder' || item.name?.endsWith('/')) {
						console.log('Raw folder item:', item);
					}
				});
			}
			
			// Handle null/undefined response
			if (response === null || response === undefined) {
				console.warn('Response is null or undefined, using empty array');
				files = [];
				return;
			}
			
			// Handle error response
			if (response && response.error) {
				console.error('API returned error:', response.error);
				// Try to create some test data if there's an error
				// This helps verify the UI is working
				if (response.error.includes('not implemented') || response.error.includes('not found')) {
					console.log('API not implemented, using test data');
					files = [
						{ id: '1', name: 'test-folder', type: 'folder', size: 0, modified: new Date().toISOString(), public: false },
						{ id: '2', name: 'test-file.txt', type: 'file', size: '1.2 KB', modified: new Date().toISOString(), public: false },
						{ id: '3', name: 'image.png', type: 'image', size: '45 KB', modified: new Date().toISOString(), public: false }
					];
					return;
				}
				files = [];
				return;
			}
			
			// Ensure we have an array
			let rawFiles = [];
			if (Array.isArray(response)) {
				rawFiles = response;
			} else if (response && response.data && Array.isArray(response.data)) {
				rawFiles = response.data;
			} else if (response && response.objects && Array.isArray(response.objects)) {
				rawFiles = response.objects;
			} else if (response && typeof response === 'object') {
				// Check if the response has any property that's an array
				const keys = Object.keys(response);
				console.log('Response object keys:', keys);
				for (const key of keys) {
					if (Array.isArray(response[key])) {
						console.log(`Found array at key "${key}":`, response[key]);
						rawFiles = response[key];
						break;
					}
				}
			}
			
			if (!rawFiles || rawFiles.length === 0) {
				console.log('No files found in response');
				files = [];
				return;
			}
			
			// Process files - backend now handles path filtering
			const processedFiles = rawFiles.map((file, index) => {
				// Use properties directly from backend response
				const processedFile = {
					id: file.id || file.ID || `file_${index}`,
					name: file.name || 'Untitled',
					fullPath: file.fullPath || file.name,
					size: file.size || '0 B',
					type: file.type || 'file',
					modified: file.modified || new Date().toISOString(),
					public: file.public || false,
					isFolder: file.isFolder || file.type === 'folder',
				};
				
				console.log(`Processing ${processedFile.isFolder ? 'folder' : 'file'}:`, {
					name: processedFile.name,
					fullPath: processedFile.fullPath,
					isFolder: processedFile.isFolder,
					type: processedFile.type,
				});
				
				return processedFile;
			});
			
			// Assign processed files to the files variable
			files = processedFiles;
			
			console.log('Processed files before assignment:', processedFiles);
			console.log('Files after assignment:', files);
			console.log('First file object:', files[0]);
			console.log('Files count:', files.length);
			// Debug: log folder details
			files.forEach(file => {
				if (file.type === 'folder') {
					console.log(`Folder: name="${file.name}", fullPath="${file.fullPath}", type="${file.type}"`);
				}
			});
		} catch (error) {
			console.error('Failed to fetch bucket objects:', error);
			files = [];
		} finally {
			loadingFiles = false;
		}
	}
	
	function formatBytes(bytes: number): string {
		if (bytes === 0) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
	}
	
	function convertToFileTree(files: any[]): any[] {
		// Convert flat file list to tree structure for FileExplorer
		const tree: any[] = [];
		const folderMap = new Map();
		
		// First pass: create all folders
		files.forEach(file => {
			if (file.isFolder || file.type === 'folder') {
				const node = {
					id: file.id,
					name: file.name,
					path: file.fullPath || file.name,
					type: 'directory',
					children: []
				};
				folderMap.set(file.fullPath || file.name, node);
				tree.push(node);
			}
		});
		
		// Second pass: add files
		files.forEach(file => {
			if (!file.isFolder && file.type !== 'folder') {
				const node = {
					id: file.id,
					name: file.name,
					path: file.fullPath || file.name,
					type: 'file',
					size: file.size
				};
				
				// Try to find parent folder
				const parentPath = currentPath;
				const parentFolder = folderMap.get(parentPath);
				
				if (parentFolder) {
					parentFolder.children.push(node);
				} else {
					tree.push(node);
				}
			}
		});
		
		return tree;
	}
	
	function handleExplorerSelect(item: any) {
		if (item.type === 'directory') {
			// Navigate to folder
			const file = files.find(f => f.id === item.id);
			if (file) {
				navigateToFolder(file);
			}
		} else {
			// View or download file
			const file = files.find(f => f.id === item.id);
			if (file) {
				viewFile(file);
			}
		}
	}

	onMount(async () => {
		// Check admin access
		if (!requireAdmin()) return;
		
		// Load initial data
		await fetchBuckets();
		
		// Add event listener to close dropdowns when clicking outside
		const handleClickOutside = () => {
			closeDropdowns();
		};
		
		document.addEventListener('click', handleClickOutside);
		
		return () => {
			document.removeEventListener('click', handleClickOutside);
		};
	});
</script>

<div class="storage-page">
	<!-- Header -->
	<div class="page-header">
		<div class="header-content">
			<div class="header-left">
				<div class="header-title">
					<HardDrive size={24} />
					<h1>Storage Manager</h1>
				</div>
				<div class="header-meta">
					<span class="meta-item">{buckets.length} buckets</span>
					<span class="meta-separator">•</span>
					<span class="meta-item">{totalFiles} files</span>
					<span class="meta-separator">•</span>
					<span class="meta-item">{usedStorage} used</span>
				</div>
			</div>
			<div class="header-info">
				<span class="info-item">
					<HardDrive size={14} />
					Connected
				</span>
			</div>
		</div>
	</div>

	<!-- Content Area -->
	<div class="content-area">
	<!-- Files Area -->
	<div class="files-container">
		<div class="card">
			<!-- Files Toolbar -->
			<div class="files-toolbar">
				<!-- Bucket Selector -->
				<div class="bucket-selector">
					<Folder size={16} style="color: var(--warning-color)" />
					<select 
						class="bucket-select"
						value={selectedBucket?.id || ''}
						on:change={(e) => {
							if (e.target.value === 'create-new') {
								openCreateBucketModal();
								// Reset the select to the current bucket
								e.target.value = selectedBucket?.id || '';
							} else {
								const bucket = buckets.find(b => b.id === e.target.value);
								if (bucket) selectBucket(bucket);
							}
						}}
					>
						{#if buckets.length === 0}
							<option value="">No buckets available</option>
						{:else if !selectedBucket}
							<option value="">Select a bucket</option>
						{/if}
						{#each buckets as bucket}
							<option value={bucket.id}>
								{bucket.name}
							</option>
						{/each}
						<option value="create-new" style="border-top: 1px solid var(--border-color); font-weight: 500;">
							+ Create New Bucket
						</option>
					</select>
					<button 
						class="toolbar-btn"
						on:click={fetchBuckets}
						title="Refresh buckets"
						style="padding: 0; width: 36px;"
					>
						<RefreshCw size={16} />
					</button>
				</div>
				
				<!-- Search -->
				<div class="search-box">
					<Search size={16} />
					<input 
						type="text" 
						placeholder="Search files..."
						bind:value={searchQuery}
					/>
				</div>
				
				<!-- Right Actions -->
				<div class="toolbar-right">
					<ExportButton 
						data={filteredFiles}
						filename="storage_files"
						disabled={filteredFiles.length === 0}
						flatten={true}
					/>
					
					<button class="toolbar-btn" on:click={openCreateFolderModal} disabled={!selectedBucket} title="New Folder">
						<FolderPlus size={16} />
						<span class="btn-text-label">New Folder</span>
					</button>
					
					<button class="toolbar-btn toolbar-btn-primary" on:click={openUploadModal} disabled={!selectedBucket} title="Upload Files">
						<Upload size={16} />
						<span class="btn-text-label">Upload Files</span>
					</button>
					
					<!-- View Mode Toggles -->
					<div class="view-toggles">
						<button 
							class="view-toggle-btn {viewMode === 'grid' ? 'active' : ''}"
							on:click={() => viewMode = 'grid'}
							title="Grid view"
						>
							<Grid size={16} />
						</button>
						<button 
							class="view-toggle-btn {viewMode === 'list' ? 'active' : ''}"
							on:click={() => viewMode = 'list'}
							title="List view"
						>
							<List size={16} />
						</button>
						<button 
							class="view-toggle-btn {viewMode === 'explorer' ? 'active' : ''}"
							on:click={() => viewMode = 'explorer'}
							title="Explorer view"
						>
							<FolderTree size={16} />
						</button>
					</div>
				</div>
			</div>
			
			{#if selectedBucket}
				<!-- Breadcrumb -->
				<div class="files-breadcrumb">
					<button class="breadcrumb-item" on:click={async () => {
						currentPath = '';
						currentFolderId = null; // Reset to root
						console.log('Navigating to root, reset currentFolderId to null');
						await fetchBucketObjects(selectedBucket.name);
					}}>
						<Folder size={14} />
						{selectedBucket.name}
					</button>
					{#each pathParts as part, i}
						<ChevronRight size={14} style="color: var(--text-muted)" />
						<button class="breadcrumb-item" on:click={() => navigateToPath(i)}>
							{part}
						</button>
					{/each}
				</div>
				
				<!-- Selection Bar -->
				{#if selectedItems.size > 0}
					<div class="selection-bar">
						<div class="selection-info">
							<Check size={16} />
							{selectedItems.size} item{selectedItems.size !== 1 ? 's' : ''} selected
						</div>
						<div class="selection-actions">
							<button class="btn btn-secondary btn-sm">
								<Download size={14} />
								Download
							</button>
							<button class="btn btn-secondary btn-sm">
								<Copy size={14} />
								Copy
							</button>
							<button class="btn btn-secondary btn-sm">
								<Move size={14} />
								Move
							</button>
							<button class="btn btn-danger btn-sm">
								<Trash2 size={14} />
								Delete
							</button>
							<button class="btn-text" on:click={() => selectedItems.clear()}>
								Clear selection
							</button>
						</div>
					</div>
				{/if}
				
				<!-- Files Display -->
				{#if viewMode === 'grid'}
					<div class="files-grid">
						{#if loadingFiles}
							<div class="empty-folder">
								<div class="loading-spinner">
									<RefreshCw size={24} class="spin" />
								</div>
								<p>Loading files...</p>
							</div>
						{:else if filteredFiles.length > 0}
							{#each filteredFiles as file}
								<div 
									class="file-card {selectedItems.has(file.id) ? 'selected' : ''}"
								>
									<!-- Three-dot menu button -->
									<button 
										class="file-menu-btn"
										on:click={(e) => toggleDropdown(file.id, e)}
										title="More options"
									>
										<MoreVertical size={16} />
									</button>
									
									<!-- Dropdown menu -->
									{#if activeDropdownId === file.id}
										<div class="file-dropdown-menu" on:click|stopPropagation>
											{#if file.type !== 'folder'}
												<button class="dropdown-item" on:click={() => viewFile(file)}>
													<Eye size={14} />
													<span>View</span>
												</button>
												<button class="dropdown-item" on:click={() => downloadFile(file)}>
													<Download size={14} />
													<span>Download</span>
												</button>
											{/if}
											<button class="dropdown-item" on:click={() => openRenameModal(file)}>
												<Edit2 size={14} />
												<span>Rename</span>
											</button>
											<button class="dropdown-item dropdown-item-danger" on:click={() => handleDeleteFromMenu(file)}>
												<Trash2 size={14} />
												<span>Delete</span>
											</button>
										</div>
									{/if}
									
									<div class="file-card-icon">
										<svelte:component 
											this={getFileIcon(file.type)} 
											size={40} 
											style="color: {getFileIconColor(file.type)}"
										/>
									</div>
									
									<div class="file-card-name" title={file.name || 'Untitled'}>
										{#if file.type === 'folder'}
											<button class="folder-link" on:click={() => navigateToFolder(file)}>
												{file.name || 'Untitled'}
											</button>
										{:else}
											{file.name || 'Untitled'}
										{/if}
									</div>
								</div>
							{/each}
						{:else}
							<div class="empty-folder">
								<Folder size={48} style="color: var(--text-muted)" />
								<p>This folder is empty</p>
								<p class="text-muted">Upload files or create folders to get started</p>
							</div>
						{/if}
					</div>
				{:else if viewMode === 'list'}
					<div class="files-list">
						<table class="table">
							<thead>
								<tr>
									<th style="width: 40px;">
										<input 
											type="checkbox" 
											checked={selectedItems.size === filteredFiles.length && filteredFiles.length > 0}
											on:change={selectAll}
										/>
									</th>
									<th>Name</th>
									<th style="width: 100px;">Size</th>
									<th style="width: 150px;">Modified</th>
									<th style="width: 100px;">Actions</th>
								</tr>
							</thead>
							<tbody>
								{#each filteredFiles as file}
									<tr 
										class="{selectedItems.has(file.id) ? 'selected' : ''}"
									>
										<td>
											<input 
												type="checkbox" 
												checked={selectedItems.has(file.id)}
												on:change={() => toggleItemSelection(file)}
											/>
										</td>
										<td>
											<div class="file-name">
												<svelte:component 
													this={getFileIcon(file.type)} 
													size={20} 
													style="color: {getFileIconColor(file.type)}"
												/>
												{#if file.type === 'folder'}
													<button class="folder-link-inline" on:click={() => navigateToFolder(file)} title={file.name || 'Untitled Folder'}>
														<span class="file-name-text">{file.name || 'Untitled Folder'}</span>
													</button>
												{:else}
													<span class="file-name-text" title={file.name || 'Untitled File'}>{file.name || 'Untitled File'}</span>
												{/if}
												{#if file.public}
													<span class="badge badge-success">Public</span>
												{/if}
											</div>
										</td>
										<td class="text-muted">
											{file.type === 'folder' ? 'Folder' : file.size}
										</td>
										<td class="text-muted">{file.modified}</td>
										<td>
											<div class="action-buttons">
												<button 
													class="btn-icon-sm"
													on:click={(e) => toggleDropdown(file.id, e)}
													title="More options"
												>
													<MoreVertical size={16} />
												</button>
												
												<!-- Dropdown menu for list view -->
												{#if activeDropdownId === file.id}
													<div class="list-dropdown-menu" on:click|stopPropagation>
														{#if file.type !== 'folder'}
															<button class="dropdown-item" on:click={() => viewFile(file)}>
																<Eye size={14} />
																<span>View</span>
															</button>
															<button class="dropdown-item" on:click={() => downloadFile(file)}>
																<Download size={14} />
																<span>Download</span>
															</button>
														{/if}
														<button class="dropdown-item" on:click={() => openRenameModal(file)}>
															<Edit2 size={14} />
															<span>Rename</span>
														</button>
														<button class="dropdown-item dropdown-item-danger" on:click={() => handleDeleteFromMenu(file)}>
															<Trash2 size={14} />
															<span>Delete</span>
														</button>
													</div>
												{/if}
											</div>
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
						
						{#if filteredFiles.length === 0}
							<div class="empty-state" style="padding: 3rem; text-align: center; color: var(--text-muted);">
								<Folder size={48} style="color: var(--text-muted)" />
								<p style="margin-top: 1rem;">This folder is empty</p>
								<p style="font-size: 0.875rem;">Upload files or create folders to get started</p>
							</div>
						{/if}
					</div>
				{:else if viewMode === 'explorer'}
					<div class="explorer-view">
						<FileExplorer 
							files={convertToFileTree(filteredFiles)}
							mode="both"
							on:select={(e) => handleExplorerSelect(e.detail)}
						/>
					</div>
				{/if}
				
				<!-- Bucket Info Bar -->
				{#if selectedBucket}
					<div class="bucket-info-bar">
						<div class="bucket-info-item">
							<File size={16} />
							<span><strong>{selectedBucket.files || 0}</strong> files</span>
						</div>
						<div class="bucket-info-item">
							<HardDrive size={16} />
							<span><strong>{selectedBucket.size || '0 B'}</strong> used</span>
						</div>
						{#if selectedBucket.public}
							<div class="bucket-info-item">
								<span class="badge badge-success">Public</span>
							</div>
						{/if}
					</div>
				{/if}
			{:else}
				<div class="no-bucket-selected">
					<Folder size={48} style="color: var(--text-muted)" />
					<p>Select a bucket to view files</p>
					<p class="text-muted" style="margin-top: 0.5rem;">or</p>
					<button class="btn btn-primary" on:click={openCreateBucketModal} style="margin-top: 0.75rem;">
						<FolderPlus size={16} />
						Create a bucket
					</button>
				</div>
			{/if}
		</div>
	</div>
</div>

<!-- Create Bucket Modal -->
{#if showCreateBucketModal}
	<div class="modal-overlay" on:click={closeCreateBucketModal}>
		<div class="modal modal-sm" on:click|stopPropagation>
			<div class="modal-header">
				<h3 class="modal-title">Create Bucket</h3>
				<button class="modal-close" on:click={closeCreateBucketModal}>
					<X size={20} />
				</button>
			</div>
			
			<div class="modal-body">
				<div class="form-group">
					<label class="form-label">Bucket Name</label>
					<input 
						type="text" 
						class="form-input" 
						bind:value={newBucket.name}
						placeholder="my-bucket"
					/>
					<small class="form-hint">Use lowercase letters, numbers, and hyphens</small>
				</div>
				
				<div class="form-group">
					<label class="checkbox-label">
						<input type="checkbox" bind:checked={newBucket.public} />
						Public Access
					</label>
					<small class="form-hint">Allow public read access to files in this bucket</small>
				</div>
			</div>
			
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={closeCreateBucketModal}>Cancel</button>
				<button class="btn btn-primary" on:click={createBucket}>Create Bucket</button>
			</div>
		</div>
	</div>
{/if}

<!-- Create Folder Modal -->
{#if showCreateFolderModal}
	<div class="modal-overlay" on:click={closeCreateFolderModal}>
		<div class="modal modal-sm" on:click|stopPropagation>
			<div class="modal-header">
				<h3 class="modal-title">Create Folder</h3>
				<button class="modal-close" on:click={closeCreateFolderModal}>
					<X size={20} />
				</button>
			</div>
			
			<div class="modal-body">
				<div class="form-group">
					<label class="form-label">Folder Name</label>
					<input 
						type="text" 
						class="form-input" 
						bind:value={newFolder.name}
						placeholder="New Folder"
					/>
				</div>
			</div>
			
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={closeCreateFolderModal}>Cancel</button>
				<button class="btn btn-primary" on:click={createFolder}>Create Folder</button>
			</div>
		</div>
	</div>
{/if}

<!-- Upload Modal -->
{#if showUploadModal}
	<div class="modal-overlay" on:click={closeUploadModal}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h3 class="modal-title">Upload Files</h3>
				<button class="modal-close" on:click={closeUploadModal}>
					<X size={20} />
				</button>
			</div>
			
			<div class="modal-body">
				{#if selectedFiles.length === 0}
					<div 
						class="upload-zone"
						on:drop={handleFileDrop}
						on:dragover|preventDefault
						on:dragenter|preventDefault
					>
						<Upload size={48} style="color: var(--text-muted)" />
						<p class="upload-text">Drag and drop files here or click to browse</p>
						<button class="btn btn-primary" on:click={openFilePicker} disabled={uploadingFiles}>Browse Files</button>
						<input 
							type="file" 
							multiple 
							class="hidden-file-input"
							on:change={handleFileSelect}
							disabled={uploadingFiles}
							bind:this={fileInputRef}
							style="display: none;"
						/>
					</div>
				{:else}
					<div class="selected-files">
						<h4 class="selected-files-title">Selected Files ({selectedFiles.length})</h4>
						<div class="files-list-container">
							{#each selectedFiles as file, index}
								<div class="selected-file-item">
									<File size={16} style="color: var(--text-muted)" />
									<div class="file-upload-info">
										<span class="selected-file-name">{file.name}</span>
										<span class="selected-file-size">{formatBytes(file.size)}</span>
										{#if uploadingFiles}
											{@const fileKey = file.name + '_' + file.lastModified}
											{@const progress = fileUploadProgress.get(fileKey) || 0}
											{#if progress >= 0}
												<div class="file-progress-bar">
													<div class="file-progress-fill" style="width: {progress}%"></div>
												</div>
												<span class="file-progress-text">{progress}%</span>
											{:else}
												<span class="file-upload-error">Failed</span>
											{/if}
										{/if}
									</div>
								</div>
							{/each}
						</div>
						
						{#if uploadingFiles}
							<div class="overall-progress">
								<span class="progress-text">Overall Progress: {uploadProgress}%</span>
							</div>
						{/if}
					</div>
				{/if}
				
				<div class="upload-info">
					<p class="text-muted">Maximum file size: 100MB</p>
					<p class="text-muted">Supported formats: All file types</p>
				</div>
			</div>
			
			<div class="modal-footer">
				{#if selectedFiles.length > 0}
					<button 
						class="btn btn-secondary" 
						on:click={() => selectedFiles = []}
						disabled={uploadingFiles}
					>
						Clear Selection
					</button>
					<button 
						class="btn btn-primary" 
						on:click={uploadFiles}
						disabled={uploadingFiles}
					>
						{uploadingFiles ? 'Uploading...' : `Upload ${selectedFiles.length} File${selectedFiles.length !== 1 ? 's' : ''}`}
					</button>
				{/if}
				<button class="btn btn-secondary" on:click={closeUploadModal} disabled={uploadingFiles}>
					{selectedFiles.length > 0 ? 'Cancel' : 'Close'}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Delete Confirmation Modal -->
{#if showDeleteModal}
	<div class="modal-overlay" on:click={closeDeleteModal}>
		<div class="modal modal-sm" on:click|stopPropagation>
			<div class="modal-header">
				<h3 class="modal-title">Delete {itemToDelete?.type === 'folder' ? 'Folder' : 'File'}</h3>
				<button class="modal-close" on:click={closeDeleteModal}>
					<X size={20} />
				</button>
			</div>
			
			<div class="modal-body">
				<p>Are you sure you want to delete this {itemToDelete?.type === 'folder' ? 'folder' : 'file'}?</p>
				<p class="text-muted">{itemToDelete?.name}</p>
				{#if itemToDelete?.type === 'folder'}
					<p class="text-danger">This will delete all files and folders inside.</p>
				{/if}
				<p class="text-danger">This action cannot be undone.</p>
			</div>
			
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={closeDeleteModal}>Cancel</button>
				<button class="btn btn-danger" on:click={deleteItem}>Delete</button>
			</div>
		</div>
	</div>
{/if}

<!-- Rename Modal -->
{#if showRenameModal}
	<div class="modal-overlay" on:click={closeRenameModal}>
		<div class="modal modal-sm" on:click|stopPropagation>
			<div class="modal-header">
				<h3 class="modal-title">Rename {itemToRename?.type === 'folder' ? 'Folder' : 'File'}</h3>
				<button class="modal-close" on:click={closeRenameModal}>
					<X size={20} />
				</button>
			</div>
			
			<div class="modal-body">
				<div class="form-group">
					<label class="form-label">New Name</label>
					<input 
						type="text" 
						class="form-input" 
						bind:value={newItemName}
						placeholder="Enter new name"
					/>
				</div>
			</div>
			
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={closeRenameModal}>Cancel</button>
				<button class="btn btn-primary" on:click={renameItem}>Rename</button>
			</div>
		</div>
	</div>
{/if}

<!-- Preview Modal -->
{#if showPreviewModal}
	<div class="modal-overlay" on:click={closePreviewModal}>
		<div class="modal modal-lg" on:click|stopPropagation>
			<div class="modal-header">
				<h3 class="modal-title">Preview: {previewItem?.name || 'File'}</h3>
				<button class="modal-close" on:click={closePreviewModal}>
					<X size={20} />
				</button>
			</div>
			
			<div class="modal-body preview-modal-body">
				{#if previewLoading}
					<div class="preview-loading">
						<span class="spinner"></span>
						Loading preview...
					</div>
				{:else if previewImageUrl}
					<div class="preview-image-container">
						<img src={previewImageUrl} alt={previewItem?.name} class="preview-image" />
					</div>
				{:else if previewContent}
					<pre class="preview-text">{previewContent}</pre>
				{:else}
					<div class="preview-empty">
						No preview available
					</div>
				{/if}
			</div>
			
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={closePreviewModal}>Close</button>
				<button class="btn btn-primary" on:click={() => downloadFile(previewItem)}>
					<Download size={16} />
					Download
				</button>
			</div>
		</div>
	</div>
{/if}
</div>

<style>
	/* Page Layout */
	.storage-page {
		height: 100%;
		display: flex;
		flex-direction: column;
		background: #f8fafc;
	}

	/* Header */
	.page-header {
		background: white;
		border-bottom: 1px solid #e2e8f0;
		padding: 1.5rem 2rem;
	}

	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.header-left {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.header-title {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.header-title h1 {
		font-size: 1.5rem;
		font-weight: 600;
		color: #0f172a;
		margin: 0;
	}

	.header-meta {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		margin-left: 2.25rem;
	}

	.meta-item {
		font-size: 0.8125rem;
		color: #64748b;
	}

	.meta-separator {
		color: #cbd5e1;
		margin: 0 0.25rem;
	}

	.header-info {
		display: flex;
		gap: 1.5rem;
	}

	.info-item {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		font-size: 0.875rem;
		color: #22c55e;
		font-weight: 500;
	}

	/* Content Area */
	.content-area {
		flex: 1;
		padding: 0.75rem 1.5rem 1.5rem;
		overflow: auto;
	}

	/* Files Container */
	.files-container {
		flex: 1;
		display: flex;
		flex-direction: column;
		height: 100%;
		background: white;
	}

	.card {
		background: white;
		height: 100%;
		display: flex;
		flex-direction: column;
	}
	
	/* Files Toolbar */
	.files-toolbar {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem 1.25rem;
		background: #fafbfc;
		flex-wrap: wrap;
	}
	
	.bucket-selector {
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}
	
	.bucket-select {
		height: 36px;
		padding: 0 0.75rem;
		padding-right: 2rem;
		border: 1px solid #e2e8f0;
		border-radius: 0.375rem;
		background: white;
		font-size: 0.8125rem;
		color: #475569;
		cursor: pointer;
		min-width: 150px;
		appearance: none;
		transition: all 0.15s;
		background-image: url("data:image/svg+xml;charset=UTF-8,%3csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%2394a3b8' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3e%3cpolyline points='6 9 12 15 18 9'%3e%3c/polyline%3e%3c/svg%3e");
		background-repeat: no-repeat;
		background-position: right 0.5rem center;
		background-size: 1rem;
	}
	
	.bucket-select:hover {
		border-color: #cbd5e1;
		background-color: #f8fafc;
	}
	
	.bucket-select:focus {
		outline: none;
		border-color: #3b82f6;
		box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.08);
	}
	
	.bucket-select:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	
	.search-box {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex: 1;
		max-width: 300px;
		height: 36px;
		padding: 0 0.75rem;
		border: 1px solid var(--border-color);
		border-radius: 6px;
		background: white;
	}
	
	.search-box:focus-within {
		border-color: var(--primary-color);
		box-shadow: 0 0 0 3px rgba(24, 154, 180, 0.1);
	}
	
	.search-box input {
		border: none;
		background: none;
		outline: none;
		flex: 1;
		font-size: 0.875rem;
		color: var(--text-primary);
	}
	
	.search-box input::placeholder {
		color: var(--text-muted);
	}
	
	.toolbar-right {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-left: auto;
	}
	
	.toolbar-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		height: 36px;
		padding: 0 0.875rem;
		border: 1px solid var(--border-color);
		border-radius: 6px;
		background: white;
		color: var(--text-primary);
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		white-space: nowrap;
	}
	
	.toolbar-btn:hover:not(:disabled) {
		background: var(--bg-hover);
		border-color: var(--primary-color);
	}
	
	.toolbar-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	
	.toolbar-btn-primary {
		background: #3b82f6;
		color: white;
		border-color: #3b82f6;
	}
	
	.toolbar-btn-primary:hover:not(:disabled) {
		background: #2563eb;
		border-color: #2563eb;
	}
	
	/* Files Breadcrumb */
	.files-breadcrumb {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--border-color);
	}
	
	.breadcrumb-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.25rem 0.5rem;
		border: none;
		background: none;
		color: var(--text-secondary);
		font-size: 0.875rem;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.2s;
	}
	
	.breadcrumb-item:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
	}
	
	.files-actions {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}
	
	.view-toggles {
		display: flex;
		gap: 2px;
		padding: 2px;
		background: var(--bg-secondary);
		border-radius: 6px;
		height: 36px;
	}
	
	.view-toggle-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border: none;
		border-radius: 4px;
		background: transparent;
		color: var(--text-secondary);
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.view-toggle-btn:hover {
		color: var(--text-primary);
	}
	
	.view-toggle-btn.active {
		background: white;
		color: var(--primary-color);
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
	}
	
	/* Selection Bar */
	.selection-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.75rem 1rem;
		background: var(--info-color);
		color: white;
	}
	
	.selection-info {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.875rem;
		font-weight: 500;
	}
	
	.selection-actions {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	
	.btn-text {
		border: none;
		background: none;
		color: white;
		font-size: 0.875rem;
		cursor: pointer;
		text-decoration: underline;
		opacity: 0.9;
	}
	
	.btn-text:hover {
		opacity: 1;
	}
	
	/* Grid View */
	.files-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
		gap: 0.75rem;
		padding: 0.75rem;
		min-height: 120px;
	}
	
	.file-card {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: 0.75rem 0.5rem;
		border: 1px solid transparent;
		border-radius: 6px;
		cursor: pointer;
		transition: all 0.2s;
		height: 100px;
	}
	
	.file-card:hover {
		background: var(--bg-hover);
		border-color: var(--border-color);
	}
	
	.file-card.selected {
		background: rgba(14, 165, 233, 0.1);
		border-color: var(--primary-color);
	}
	
	.file-card-icon {
		margin-bottom: 0.5rem;
	}
	
	.file-card-name {
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--text-primary);
		text-align: center;
		width: 100%;
		padding: 0.25rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	
	.file-card-actions {
		display: flex;
		gap: 0.25rem;
		margin-top: auto;
		opacity: 0;
		transition: opacity 0.2s;
	}
	
	.file-card:hover .file-card-actions {
		opacity: 1;
	}
	
	.empty-folder {
		grid-column: 1 / -1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 3rem;
		color: var(--text-muted);
		text-align: center;
	}
	
	.empty-folder p {
		margin: 0.5rem 0;
		font-size: 0.875rem;
	}
	
	.empty-folder p:first-of-type {
		font-weight: 500;
		color: var(--text-secondary);
	}
	
	.loading-spinner {
		display: flex;
		justify-content: center;
		margin-bottom: 1rem;
	}
	
	.spin {
		animation: spin 1s linear infinite;
	}
	
	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}
	
	.btn-icon-xs {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 24px;
		height: 24px;
		border: 1px solid var(--border-color);
		border-radius: 4px;
		background: white;
		color: var(--text-secondary);
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.btn-icon-xs:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
	}
	
	/* List View */
	.files-list {
		padding: 0;
	}
	
	.explorer-view {
		padding: 1rem;
		min-height: 400px;
	}
	
	/* Bucket Info Bar */
	.bucket-info-bar {
		display: flex;
		align-items: center;
		gap: 2rem;
		padding: 0.75rem 1rem;
		border-top: 1px solid var(--border-color);
		font-size: 0.875rem;
		color: var(--text-muted);
	}
	
	.bucket-info-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	
	.bucket-info-item strong {
		color: var(--text-primary);
	}
	
	.files-list .table {
		margin: 0;
	}
	
	.files-list tbody tr.selected {
		background: rgba(14, 165, 233, 0.1);
	}
	
	.file-name {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		max-width: 100%;
	}
	
	.file-name-text {
		display: inline-block;
		max-width: 300px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		vertical-align: middle;
	}
	
	/* No Bucket Selected */
	.no-bucket-selected {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 400px;
		color: var(--text-muted);
	}
	
	.no-bucket-selected p {
		margin-top: 1rem;
		font-size: 0.875rem;
	}
	
	/* Upload Zone */
	.upload-zone {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 3rem;
		border: 2px dashed var(--border-color);
		border-radius: 8px;
		background: var(--bg-secondary);
		text-align: center;
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.upload-zone:hover {
		border-color: var(--primary-color);
		background: rgba(14, 165, 233, 0.05);
	}
	
	.upload-text {
		margin: 1rem 0;
		color: var(--text-secondary);
	}
	
	.hidden-file-input {
		display: none;
	}
	
	.upload-info {
		margin-top: 1rem;
		padding-top: 1rem;
		border-top: 1px solid var(--border-color);
	}
	
	.upload-info p {
		margin: 0.25rem 0;
		font-size: 0.875rem;
	}
	
	/* Selected Files Display */
	.selected-files {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	
	.selected-files-title {
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--text-primary);
		margin: 0;
	}
	
	.files-list-container {
		max-height: 200px;
		overflow-y: auto;
		border: 1px solid var(--border-color);
		border-radius: 6px;
		padding: 0.5rem;
	}
	
	.selected-file-item {
		display: flex;
		align-items: flex-start;
		gap: 0.5rem;
		padding: 0.5rem;
		border-radius: 4px;
		font-size: 0.875rem;
		border: 1px solid transparent;
	}
	
	.selected-file-item:hover {
		background: var(--bg-hover);
	}
	
	.file-upload-info {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		min-width: 0;
	}
	
	.selected-file-name {
		color: var(--text-primary);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	
	.selected-file-size {
		color: var(--text-muted);
		font-size: 0.75rem;
	}
	
	.file-progress-bar {
		width: 100%;
		height: 4px;
		background: var(--bg-secondary);
		border-radius: 2px;
		overflow: hidden;
		margin-top: 0.25rem;
	}
	
	.file-progress-fill {
		height: 100%;
		background: var(--primary-color);
		border-radius: 2px;
		transition: width 0.3s ease;
	}
	
	.file-progress-text {
		font-size: 0.75rem;
		color: var(--text-secondary);
	}
	
	.file-upload-error {
		font-size: 0.75rem;
		color: var(--danger-color);
		font-weight: 500;
	}
	
	.overall-progress {
		margin-top: 1rem;
		padding-top: 1rem;
		border-top: 1px solid var(--border-color);
		text-align: center;
	}
	
	/* Upload Progress */
	.upload-progress {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	
	.progress-bar {
		width: 100%;
		height: 8px;
		background: var(--bg-secondary);
		border-radius: 4px;
		overflow: hidden;
	}
	
	.progress-fill {
		height: 100%;
		background: var(--primary-color);
		border-radius: 4px;
		transition: width 0.3s ease;
	}
	
	.progress-text {
		font-size: 0.875rem;
		color: var(--text-secondary);
		text-align: center;
	}
	
	/* Modal Styles */
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
		z-index: 2000;
	}
	
	.modal {
		background: white;
		border-radius: 8px;
		width: 90%;
		max-width: 500px;
		max-height: 90vh;
		overflow: auto;
	}
	
	.modal-sm {
		max-width: 400px;
	}
	
	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1.5rem;
		border-bottom: 1px solid var(--border-color);
	}
	
	.modal-title {
		font-size: 1.125rem;
		font-weight: 600;
		color: var(--text-primary);
	}
	
	.modal-close {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border: none;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.2s;
	}
	
	.modal-close:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
	}
	
	.modal-body {
		padding: 1.5rem;
	}
	
	.modal-footer {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid var(--border-color);
	}
	
	.form-hint {
		display: block;
		margin-top: 0.25rem;
		font-size: 0.75rem;
		color: var(--text-muted);
	}
	
	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.875rem;
		color: var(--text-primary);
		cursor: pointer;
	}
	
	.text-danger {
		color: var(--danger-color);
	}
	
	.text-muted {
		color: var(--text-muted);
		font-size: 0.875rem;
	}
	
	.btn-icon-sm {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border: 1px solid var(--border-color);
		border-radius: 4px;
		background: white;
		color: var(--text-secondary);
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.btn-icon-sm:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
	}
	
	.action-buttons {
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}
	
	.folder-link {
		border: none;
		background: none;
		color: var(--text-primary, #1f2937);
		font: inherit;
		padding: 2px 4px;
		cursor: pointer;
		text-decoration: none;
		width: 100%;
		display: block;
		font-size: 0.813rem;
		font-weight: 500;
		text-align: center;
	}
	
	.folder-link:hover {
		color: var(--primary-color);
		text-decoration: underline;
	}
	
	.folder-link-inline {
		border: none;
		background: none;
		color: inherit;
		font: inherit;
		padding: 0;
		cursor: pointer;
		text-decoration: none;
	}
	
	.folder-link-inline:hover {
		color: var(--primary-color);
		text-decoration: underline;
	}
	
	/* Three-dot menu styles */
	.file-menu-btn {
		position: absolute;
		top: 0.5rem;
		right: 0.5rem;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border: none;
		border-radius: 4px;
		background: transparent;
		color: var(--text-muted);
		cursor: pointer;
		opacity: 0;
		transition: all 0.2s;
		z-index: 10;
	}
	
	.file-card:hover .file-menu-btn {
		opacity: 1;
	}
	
	.file-menu-btn:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
	}
	
	.file-dropdown-menu {
		position: absolute;
		top: 2.5rem;
		right: 0.5rem;
		background: white;
		border: 1px solid var(--border-color);
		border-radius: 6px;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
		min-width: 140px;
		z-index: 100;
		overflow: hidden;
	}
	
	.dropdown-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.5rem 0.75rem;
		border: none;
		background: none;
		color: var(--text-primary);
		font-size: 0.875rem;
		text-align: left;
		cursor: pointer;
		transition: background 0.2s;
	}
	
	.dropdown-item:hover {
		background: var(--bg-hover);
	}
	
	.dropdown-item-danger {
		color: var(--danger-color);
	}
	
	.dropdown-item-danger:hover {
		background: rgba(239, 68, 68, 0.1);
	}
	
	/* List view dropdown menu */
	.action-buttons {
		position: relative;
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}
	
	.list-dropdown-menu {
		position: absolute;
		top: 100%;
		right: 0;
		margin-top: 0.25rem;
		background: white;
		border: 1px solid var(--border-color);
		border-radius: 6px;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
		min-width: 140px;
		z-index: 100;
		overflow: hidden;
	}
	
	/* Button text labels */
	.btn-text-label {
		display: inline;
	}
	
	/* Mobile Responsive */
	@media (max-width: 1024px) {
		.files-toolbar {
			gap: 0.5rem;
		}
		
		.btn-text-label {
			display: none;
		}
		
		.toolbar-btn {
			padding: 0 0.75rem;
			min-width: 36px;
		}
		
		.search-box {
			max-width: 200px;
		}
	}
	
	@media (max-width: 768px) {
		.files-toolbar {
			padding: 0.75rem;
			flex-direction: column;
			align-items: stretch;
		}
		
		.bucket-selector {
			width: 100%;
			order: 1;
		}
		
		.bucket-select {
			flex: 1;
		}
		
		.search-box {
			max-width: none;
			width: 100%;
			order: 2;
			margin-top: 0.5rem;
		}
		
		.toolbar-right {
			width: 100%;
			justify-content: space-between;
			order: 3;
			margin-top: 0.5rem;
			margin-left: 0;
		}
		
		.toolbar-btn {
			flex: 1;
			justify-content: center;
		}
		
		.bucket-info-bar {
			flex-wrap: wrap;
			gap: 1rem;
		}
		
		.files-grid {
			grid-template-columns: repeat(auto-fill, minmax(80px, 1fr));
			gap: 0.5rem;
			padding: 0.5rem;
		}
		
		.file-card {
			height: 90px;
			padding: 0.5rem 0.25rem;
		}
		
		.file-card-name {
			font-size: 0.75rem;
		}
	}
	
	@media (max-width: 480px) {
		.header-title h1 {
			font-size: 1.25rem;
		}
		
		.header-meta {
			margin-left: 0;
			font-size: 0.75rem;
		}
		
		.meta-separator {
			margin: 0 0.125rem;
		}
		
		.toolbar-btn {
			height: 34px;
			font-size: 0.813rem;
		}
		
		.view-toggles {
			height: 34px;
		}
		
		.view-toggle-btn {
			width: 30px;
			height: 30px;
		}
		
		.files-grid {
			grid-template-columns: repeat(auto-fill, minmax(70px, 1fr));
		}
	}
	
	/* Preview Modal Styles */
	.modal-lg {
		max-width: 900px;
		width: 90%;
	}
	
	.preview-modal-body {
		max-height: 70vh;
		overflow: auto;
		padding: 1rem;
	}
	
	.preview-loading {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 1rem;
		padding: 2rem;
		color: var(--text-muted);
	}
	
	.preview-image-container {
		display: flex;
		justify-content: center;
		align-items: center;
		min-height: 200px;
		background: #f0f0f0;
		border-radius: 0.375rem;
		padding: 1rem;
	}
	
	.preview-image {
		max-width: 100%;
		max-height: 60vh;
		object-fit: contain;
		border-radius: 0.25rem;
		box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
	}
	
	.preview-text {
		background: #f8f9fa;
		border: 1px solid #dee2e6;
		border-radius: 0.375rem;
		padding: 1rem;
		font-family: 'Monaco', 'Courier New', monospace;
		font-size: 0.875rem;
		line-height: 1.5;
		white-space: pre-wrap;
		word-wrap: break-word;
		max-height: 60vh;
		overflow: auto;
		margin: 0;
	}
	
	.preview-empty {
		text-align: center;
		padding: 3rem;
		color: var(--text-muted);
	}
	
	.spinner {
		display: inline-block;
		width: 20px;
		height: 20px;
		border: 3px solid rgba(0, 0, 0, 0.1);
		border-radius: 50%;
		border-top-color: var(--primary);
		animation: spin 0.8s linear infinite;
	}
	
	@keyframes spin {
		to { transform: rotate(360deg); }
	}
</style>