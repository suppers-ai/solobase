<script lang="ts">
	import { onMount } from 'svelte';
	import {
		Cloud, Upload, Share2, Download, Users, Activity,
		HardDrive, Shield, Trash2, Eye, Link, UserPlus,
		Clock, AlertCircle, BarChart3, FileText, Settings,
		CheckCircle, XCircle, Info, TrendingUp, Database, Folder, BarChart,
		FolderOpen, File, Lock, Globe, Zap, AlertTriangle
	} from 'lucide-svelte';
	import { api, ErrorHandler, authFetch } from '$lib/api';
	import { requireAdmin } from '$lib/utils/auth';
	import { toasts } from '$lib/stores/toast';
	import { formatBytes } from '$lib/utils/formatters';
	import FileExplorer from '$lib/components/FileExplorer.svelte';
	import SearchInput from '$lib/components/SearchInput.svelte';
	import QuotaManager from '$lib/components/cloudstorage/QuotaManager.svelte';
	import LoadingSpinner from '$lib/components/ui/LoadingSpinner.svelte';
	import Modal from '$lib/components/ui/Modal.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import TabNavigation from '$lib/components/ui/TabNavigation.svelte';

	const cloudStorageTabs = [
		{ id: 'overview', label: 'Overview' },
		{ id: 'shares', label: 'Shares' },
		{ id: 'quotas', label: 'Quotas & Limits' },
		{ id: 'logs', label: 'Access Logs' },
		{ id: 'analytics', label: 'Analytics' },
		{ id: 'settings', label: 'Settings' }
	];

	function handleTabChange(event: CustomEvent<string>) {
		if (event.detail === 'logs') {
			loadAccessLogs();
		}
	}

	let loading = true;
	let activeTab = 'overview';
	
	// Type definitions
	interface User {
		id: string;
		email: string;
		name?: string;
	}
	
	interface FileTreeNode {
		id: string;
		name: string;
		path: string;
		type: 'file' | 'directory';
		size?: number;
		children?: FileTreeNode[];
	}
	
	interface Share {
		id: string;
		objectId: string;
		sharedWithEmail?: string;
		sharedWithUserId?: string;
		permissionLevel: 'view' | 'edit' | 'admin';
		isPublic: boolean;
		shareToken?: string;
		expiresAt?: string;
	}
	
	// Data states
	let stats: any = {};
	let shares: Share[] = [];
	let quotas: any[] = [];
	let accessLogs: any[] = [];
	let storageObjects: FileTreeNode[] = [];
	let roles: any[] = [];
	
	// Modals
	let showShareModal = false;
	let showQuotaModal = false;
	let showFileExplorer = false;
	let showDefaultQuotaModal = false;
	let selectedObject: any = null;
	let selectedUser: any = null;
	
	// User search
	let userSearchQuery = '';
	let searchResults: User[] = [];
	let searchingUsers = false;
	let searchDebounceTimer: NodeJS.Timeout;
	let showSearchDropdown = false;
	
	// Default quota settings
	let defaultQuotas = {
		storage: 5, // GB
		bandwidth: 10, // GB
		applyToExisting: false
	};
	
	// Extension settings
	let showUsageInProfile = true;
	
	// Forms
	let shareForm = {
		objectId: '',
		sharedWithEmail: '',
		permissionLevel: 'view',
		inheritToChildren: true,
		generateToken: false,
		isPublic: false,
		expiresIn: 24 // hours
	};
	
	let quotaForm = {
		userId: '',
		maxStorageGB: 5,
		maxBandwidthGB: 10
	};
	
	// Filters
	let logFilters = {
		objectId: '',
		userId: '',
		action: '',
		startDate: '',
		endDate: '',
		limit: 100
	};

	onMount(async () => {
		if (!requireAdmin()) return;
		await loadRoles();
		await loadData();
		await loadExtensionSettings();
	});

	async function loadRoles() {
		try {
			const response = await authFetch('/api/admin/iam/roles');
			if (response.ok) {
				roles = await response.json();
			}
		} catch (error) {
			console.error('Failed to load roles:', error);
		}
	}

	async function loadData() {
		try {
			loading = true;
			
			// Load CloudStorage extension data
			try {
				// Try to load CloudStorage data - endpoints may not be available yet
				const promises = [];
				
				promises.push(
					api.get('/ext/cloudstorage/stats')
						.catch(e => {
							console.log('Stats endpoint not available');
							return { totalFiles: 0, totalSize: 0, totalUsers: 0, totalShares: 0 };
						})
				);
				
				promises.push(
					api.get('/ext/cloudstorage/shares')
						.catch(e => {
							console.log('Shares endpoint not available');
							return [];
						})
				);
				
				promises.push(
					api.get('/ext/cloudstorage/quotas/user')
						.catch(e => {
							console.log('Quotas endpoint not available');
							return [];
						})
				);
				
				promises.push(
					api.get('/ext/cloudstorage/access-logs?limit=50')
						.catch(e => {
							console.log('Access logs endpoint not available');
							return [];
						})
				);
				
				const [statsRes, sharesRes, quotasRes, logsRes] = await Promise.all(promises);

				stats = statsRes || {};
				shares = (Array.isArray(sharesRes) ? sharesRes : []) as Share[];
				quotas = Array.isArray(quotasRes) ? quotasRes : [];
				accessLogs = Array.isArray(logsRes) ? logsRes : [];
			} catch (err) {
				// CloudStorage extension API might not be fully implemented yet
				// Continue with empty data
			}
			
			// Load storage buckets and files from main storage API
			try {
				const bucketsRes = await api.get('/storage/buckets');
				
				if (bucketsRes && Array.isArray(bucketsRes) && bucketsRes.length > 0) {
					// Load files for each bucket in parallel for better performance
					const bucketPromises = bucketsRes.map(async (bucket) => {
						try {
							const filesRes = await api.get(`/storage/buckets/${bucket.name}/objects`);
							return {
								...bucket,
								objects: filesRes || []
							};
						} catch (err) {
							// Continue with empty objects if one bucket fails
							return { ...bucket, objects: [] };
						}
					});
					
					const bucketsWithFiles = await Promise.all(bucketPromises);
					storageObjects = processFileTreeNodes(bucketsWithFiles);
				} else {
					storageObjects = [];
				}
			} catch (err) {
				// Silently fail - storage might not be configured
				storageObjects = [];
			}
			
		} catch (error) {
			// Handle any unexpected errors gracefully
		} finally {
			loading = false;
		}
	}
	
	async function loadExtensionSettings() {
		try {
			// Load the setting from API
			const response = await api.get<{ value?: string | boolean }>('/settings/ext_cloudstorage_profile_show_usage');
			if (response && response.value !== undefined) {
				showUsageInProfile = response.value === 'true' || response.value === true;
			}
		} catch {
			// Setting might not exist yet, use default value
			showUsageInProfile = true;
		}
	}
	
	async function updateProfileUsageSetting() {
		try {
			await api.post('/settings', {
				key: 'ext_cloudstorage_profile_show_usage',
				value: showUsageInProfile,
				type: 'bool'
			});
		} catch (err) {
			console.error('Failed to update profile usage setting:', err);
		}
	}
	
	function processFileTreeNodes(buckets: any[]): any[] {
		// Convert bucket and object data to file tree format
		const tree: any[] = [];
		
		buckets.forEach(bucket => {
			const bucketNode = {
				id: bucket.id || bucket.name,
				name: bucket.name,
				path: bucket.name,
				type: 'directory',
				children: []
			};
			
			// Add objects to bucket
			if (bucket.objects && Array.isArray(bucket.objects)) {
				bucket.objects.forEach((obj: { objectKey?: string; id?: string; contentType?: string; size?: number }) => {
					// Skip if no objectKey
					if (!obj.objectKey) return;

					const parts = obj.objectKey.split('/').filter((p: string) => p); // Remove empty parts
					if (parts.length === 0) return;

					let currentLevel: FileTreeNode[] = bucketNode.children;
					let currentPath = bucket.name;

					parts.forEach((part: string, index: number) => {
						currentPath += '/' + part;
						const isFile = index === parts.length - 1 && obj.contentType !== 'application/x-directory';

						let node: FileTreeNode | undefined = currentLevel.find((n: FileTreeNode) => n.name === part);
						if (!node) {
							node = {
								id: isFile ? (obj.id || currentPath) : `dir-${currentPath}`,
								name: part,
								path: currentPath,
								type: isFile ? 'file' : 'directory',
								size: isFile ? obj.size : undefined,
								children: isFile ? undefined : []
							};
							currentLevel.push(node);
						}

						if (!isFile && node.children) {
							currentLevel = node.children;
						}
					});
				});
			}
			
			// Only add bucket if it has content or is not empty
			if (bucketNode.children.length > 0 || !bucket.objects || bucket.objects.length === 0) {
				tree.push(bucketNode);
			}
		});
		
		return tree;
	}
	
	function handleFileSelect(event: CustomEvent) {
		const selected = event.detail;
		shareForm.objectId = selected.id;
		selectedObject = selected;
		showFileExplorer = false;
	}
	
	async function createShare() {
		try {
			const expiresAt = shareForm.expiresIn ? 
				new Date(Date.now() + shareForm.expiresIn * 3600000).toISOString() : 
				null;
			
			const response = await api.post('/ext/cloudstorage/shares', {
				objectId: shareForm.objectId,
				sharedWithEmail: shareForm.sharedWithEmail || undefined,
				permissionLevel: shareForm.permissionLevel,
				inheritToChildren: shareForm.inheritToChildren,
				generateToken: shareForm.generateToken,
				isPublic: shareForm.isPublic,
				expiresAt: expiresAt
			});
			
			if (response) {
				showShareModal = false;
				await loadData();

				// Show share link if token was generated
				const typedResponse = response as { shareToken?: string };
				if (typedResponse.shareToken) {
					toasts.success(`Share link created: ${window.location.origin}/share/${typedResponse.shareToken}`);
				}
			}
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}
	
	async function updateQuota() {
		try {
			const response = await api.put('/ext/cloudstorage/quota', {
				userId: quotaForm.userId,
				maxStorageBytes: quotaForm.maxStorageGB * 1024 * 1024 * 1024,
				maxBandwidthBytes: quotaForm.maxBandwidthGB * 1024 * 1024 * 1024
			});
			
			if (response) {
				showQuotaModal = false;
				quotaForm = {
					userId: '',
					maxStorageGB: 5,
					maxBandwidthGB: 10
				};
				searchResults = [];
				userSearchQuery = '';
				showSearchDropdown = false;
				await loadData();
			}
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}
	
	async function searchUsers() {
		// Clear previous timer
		if (searchDebounceTimer) {
			clearTimeout(searchDebounceTimer);
		}
		
		// Reset if query is too short
		if (!userSearchQuery || userSearchQuery.length < 2) {
			searchResults = [];
			searchingUsers = false;
			showSearchDropdown = false;
			return;
		}
		
		// Show loading state
		searchingUsers = true;
		showSearchDropdown = true;
		
		// Debounce the actual search
		searchDebounceTimer = setTimeout(async () => {
			try {
				// Use the search endpoint from cloudstorage extension
				// Extension routes are at /ext/ not /api/ext/
				searchResults = await api.get(`/ext/cloudstorage/users/search?q=${encodeURIComponent(userSearchQuery)}`) || [];
			} catch (error) {
				// If search fails, show no results
				searchResults = [];
			} finally {
				searchingUsers = false;
			}
		}, 300); // 300ms debounce
	}
	
	function selectUser(user: User) {
		quotaForm.userId = user.id;
		userSearchQuery = user.email || user.id;
		searchResults = [];
		showSearchDropdown = false;
	}
	
	async function updateDefaultQuotas() {
		try {
			const response = await api.put('/ext/cloudstorage/default-quotas', {
				defaultStorage: defaultQuotas.storage * 1024 * 1024 * 1024,
				defaultBandwidth: defaultQuotas.bandwidth * 1024 * 1024 * 1024
			});
			
			if (response) {
				toasts.success('Default quotas updated successfully');
				showDefaultQuotaModal = false;
				await loadData();
			}
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}
	
	async function loadAccessLogs() {
		try {
			const params = new URLSearchParams();
			Object.entries(logFilters).forEach(([key, value]) => {
				if (value) params.append(key, value.toString());
			});

			const response = await api.get(`/ext/cloudstorage/access-logs?${params}`);
			accessLogs = Array.isArray(response) ? response : [];
		} catch {
			// Silently handle error - logs might not be available
		}
	}
	
	function formatPercentage(value: number): string {
		return `${Math.round(value)}%`;
	}
	
	function getActionIcon(action: string) {
		switch (action) {
			case 'view': return Eye;
			case 'download': return Download;
			case 'upload': return Upload;
			case 'delete': return Trash2;
			case 'share': return Share2;
			case 'edit': return FileText;
			default: return Activity;
		}
	}
	
	function getActionColor(action: string) {
		switch (action) {
			case 'view': return 'text-blue-600';
			case 'download': return 'text-purple-600';
			case 'upload': return 'text-green-600';
			case 'delete': return 'text-red-600';
			case 'share': return 'text-cyan-600';
			case 'edit': return 'text-orange-600';
			default: return 'text-gray-600';
		}
	}
	
	function getPermissionColor(level: string) {
		switch (level) {
			case 'view': return 'bg-blue-100 text-blue-700';
			case 'edit': return 'bg-orange-100 text-orange-700';
			case 'admin': return 'bg-red-100 text-red-700';
			default: return 'bg-gray-100 text-gray-700';
		}
	}
</script>

<div class="page-container">
	<!-- Header -->
	<div class="page-header">
		<div class="header-content">
			<div class="header-left">
				<div class="header-title">
					<Cloud size={24} />
					<h1>CloudStorage Extension</h1>
				</div>
				<p class="header-subtitle">Extend your storage with advanced sharing, access tracking, bandwidth quotas, and detailed analytics</p>
			</div>
		</div>
	</div>

	<!-- Tabs -->
	<TabNavigation tabs={cloudStorageTabs} bind:activeTab on:change={handleTabChange} />

	{#if loading}
		<div class="loading">Loading...</div>
	{:else}
		{#if activeTab === 'overview'}
			<!-- Extension Description -->
			<div class="description-card">
				<h3>About CloudStorage Extension</h3>
				<p>CloudStorage extends your existing storage system with advanced features for enterprise-level file management and sharing.</p>
				
				<div class="features-grid">
					<div class="feature-item">
						<Share2 size={20} />
						<div>
							<h4>Advanced Sharing</h4>
							<p>Create public links, share with specific users, set expiration dates, and control permissions at a granular level.</p>
						</div>
					</div>
					<div class="feature-item">
						<Shield size={20} />
						<div>
							<h4>Access Control</h4>
							<p>Define view, edit, and admin permissions. Track who accesses your files and when, with detailed audit logs.</p>
						</div>
					</div>
					<div class="feature-item">
						<Database size={20} />
						<div>
							<h4>Storage Quotas</h4>
							<p>Set storage and bandwidth limits per user or organization. Monitor usage and prevent quota overruns.</p>
						</div>
					</div>
					<div class="feature-item">
						<BarChart size={20} />
						<div>
							<h4>Analytics & Insights</h4>
							<p>Get detailed analytics on file access patterns, popular content, and bandwidth consumption trends.</p>
						</div>
					</div>
				</div>
			</div>

			<!-- Overview Stats -->
			<div class="stats-grid">
				{#if stats.storage}
					<div class="stat-card">
						<div class="stat-icon bg-blue-100">
							<HardDrive size={20} class="text-blue-600" />
						</div>
						<div class="stat-content">
							<p class="stat-label">Total Storage</p>
							<p class="stat-value">{formatBytes(stats.storage?.totalSize || 0)}</p>
							<p class="stat-detail">{stats.storage?.totalObjects || 0} objects</p>
						</div>
					</div>
				{/if}
				
				{#if stats.quota}
					<div class="stat-card">
						<div class="stat-icon bg-green-100">
							<Database size={20} class="text-green-600" />
						</div>
						<div class="stat-content">
							<p class="stat-label">Storage Usage</p>
							<p class="stat-value">{formatPercentage(stats.quota?.storagePercentage || 0)}</p>
							<div class="progress-bar">
								<div class="progress-fill" style="width: {stats.quota?.storagePercentage || 0}%"></div>
							</div>
						</div>
					</div>
					
					<div class="stat-card">
						<div class="stat-icon bg-purple-100">
							<TrendingUp size={20} class="text-purple-600" />
						</div>
						<div class="stat-content">
							<p class="stat-label">Bandwidth Usage</p>
							<p class="stat-value">{formatPercentage(stats.quota?.bandwidthPercentage || 0)}</p>
							<div class="progress-bar">
								<div class="progress-fill bandwidth" style="width: {stats.quota?.bandwidthPercentage || 0}%"></div>
							</div>
						</div>
					</div>
				{/if}
				
				{#if stats.shares}
					<div class="stat-card">
						<div class="stat-icon bg-cyan-100">
							<Share2 size={20} class="text-cyan-600" />
						</div>
						<div class="stat-content">
							<p class="stat-label">Active Shares</p>
							<p class="stat-value">{stats.shares?.activeShares || 0}</p>
							<p class="stat-detail">Total: {stats.shares?.totalShares || 0}</p>
						</div>
					</div>
				{/if}
				
				{#if stats.access}
					<div class="stat-card">
						<div class="stat-icon bg-orange-100">
							<Activity size={20} class="text-orange-600" />
						</div>
						<div class="stat-content">
							<p class="stat-label">Total Access</p>
							<p class="stat-value">{stats.access?.totalAccess || 0}</p>
							<p class="stat-detail">{stats.access?.uniqueUsers || 0} unique users</p>
						</div>
					</div>
				{/if}
			</div>

			<!-- Recent Activity -->
			{#if accessLogs.length > 0}
				<div class="activity-card">
					<h3>Recent Activity</h3>
					<div class="activity-list">
						{#each accessLogs.slice(0, 5) as log}
							<div class="activity-item">
								<div class="activity-icon {getActionColor(log.action)}">
									<svelte:component this={getActionIcon(log.action)} size={16} />
								</div>
								<div class="activity-details">
									<p class="activity-description">
										<strong>{log.userId || 'Anonymous'}</strong>
										{log.action}
										<span class="file-name">Object {log.objectId.slice(0, 8)}...</span>
									</p>
									<div class="activity-meta">
										{#if log.metadata?.bytesSize}
											<span>{formatBytes(log.metadata.bytesSize)}</span>
											<span>•</span>
										{/if}
										<span>{new Date(log.createdAt).toLocaleString()}</span>
									</div>
								</div>
							</div>
						{/each}
					</div>
				</div>
			{/if}
		{/if}

		{#if activeTab === 'shares'}
			<!-- Shares Management -->
			<div class="section-card">
				<div class="section-header">
					<h3>Share Statistics</h3>
				</div>
				
				{#if !stats.shares || stats.shares.totalShares === 0}
					<!-- Empty State for Shares -->
					<EmptyState
						icon={Share2}
						title="No files are shared"
						message="Start sharing files and folders to collaborate with others. You can create public links or share with specific users."
					/>
				{:else}
					<div class="share-stats-grid">
						<div class="share-stat-card">
							<div class="share-stat-icon">
								<Share2 size={24} class="text-cyan-600" />
							</div>
							<div class="share-stat-content">
								<p class="share-stat-value">{stats.shares.totalShares || 0}</p>
								<p class="share-stat-label">Total Shares</p>
							</div>
						</div>
						
						<div class="share-stat-card">
							<div class="share-stat-icon">
								<Globe size={24} class="text-green-600" />
							</div>
							<div class="share-stat-content">
								<p class="share-stat-value">{stats.shares.publicShares || 0}</p>
								<p class="share-stat-label">Public Shares</p>
							</div>
						</div>
						
						<div class="share-stat-card">
							<div class="share-stat-icon">
								<Lock size={24} class="text-orange-600" />
							</div>
							<div class="share-stat-content">
								<p class="share-stat-value">{stats.shares.privateShares || 0}</p>
								<p class="share-stat-label">Private Shares</p>
							</div>
						</div>
						
						<div class="share-stat-card">
							<div class="share-stat-icon">
								<FolderOpen size={24} class="text-blue-600" />
							</div>
							<div class="share-stat-content">
								<p class="share-stat-value">{stats.shares.foldersShared || 0}</p>
								<p class="share-stat-label">Folders Shared</p>
							</div>
						</div>
						
						<div class="share-stat-card">
							<div class="share-stat-icon">
								<File size={24} class="text-purple-600" />
							</div>
							<div class="share-stat-content">
								<p class="share-stat-value">{stats.shares.filesShared || 0}</p>
								<p class="share-stat-label">Files Shared</p>
							</div>
						</div>
						
						<div class="share-stat-card">
							<div class="share-stat-icon">
								<Clock size={24} class="text-red-600" />
							</div>
							<div class="share-stat-content">
								<p class="share-stat-value">{stats.shares.expiredShares || 0}</p>
								<p class="share-stat-label">Expired Shares</p>
							</div>
						</div>
					</div>
				{/if}
				
				<div class="section-header" style="margin-top: 2rem;">
					<h3>Active Shares</h3>
					<button class="btn btn-sm btn-primary" on:click={() => showShareModal = true}>
						<Share2 size={14} />
						New Share
					</button>
				</div>
				
				{#if shares.length > 0}
					<div class="table-container">
						<table class="data-table">
							<thead>
								<tr>
									<th>Object</th>
									<th>Shared With</th>
									<th>Permission</th>
									<th>Type</th>
									<th>Expires</th>
									<th>Actions</th>
								</tr>
							</thead>
							<tbody>
								{#each shares as share}
									<tr>
										<td class="truncate">{share.objectId.slice(0, 8)}...</td>
										<td>
											{#if share.sharedWithEmail}
												{share.sharedWithEmail}
											{:else if share.sharedWithUserId}
												User: {share.sharedWithUserId.slice(0, 8)}...
											{:else if share.shareToken}
												<span class="text-cyan-600">Public Link</span>
											{:else}
												-
											{/if}
										</td>
										<td>
											<span class="badge {getPermissionColor(share.permissionLevel)}">
												{share.permissionLevel}
											</span>
										</td>
										<td>
											{#if share.isPublic}
												<span class="badge bg-green-100 text-green-700">Public</span>
											{:else}
												<span class="badge bg-gray-100 text-gray-700">Private</span>
											{/if}
										</td>
										<td>
											{#if share.expiresAt}
												{new Date(share.expiresAt).toLocaleDateString()}
											{:else}
												Never
											{/if}
										</td>
										<td>
											{#if share.shareToken}
												<button class="btn btn-xs" on:click={() => {
													navigator.clipboard.writeText(`${window.location.origin}/share/${share.shareToken}`);
													toasts.success('Share link copied!');
												}}>
													<Link size={12} />
													Copy Link
												</button>
											{/if}
											<button class="btn btn-xs btn-danger">
												<Trash2 size={12} />
												Revoke
											</button>
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{:else}
					<EmptyState
						icon={Share2}
						title="No active shares"
					>
						<button class="btn btn-primary" on:click={() => showShareModal = true}>
							Create First Share
						</button>
					</EmptyState>
				{/if}
			</div>
		{/if}

		{#if activeTab === 'quotas'}
			<!-- Quotas Management -->
			<div class="section-card">
				<QuotaManager {roles} />
			</div>
		{/if}

		{#if false && activeTab === 'never'}
			<div class="section-card">
					<!-- Quota Statistics Cards -->
					<div class="quota-stats-grid">
						<div class="quota-stat-card">
							<div class="quota-stat-icon">
								<Users size={24} class="text-blue-600" />
							</div>
							<div class="quota-stat-content">
								<p class="quota-stat-value">{stats.quota.totalUsers || 0}</p>
								<p class="quota-stat-label">Users with Quotas</p>
							</div>
						</div>
						
						<div class="quota-stat-card">
							<div class="quota-stat-icon">
								<HardDrive size={24} class="text-green-600" />
							</div>
							<div class="quota-stat-content">
								<p class="quota-stat-value">{formatBytes(stats.quota.storageUsed || 0)}</p>
								<p class="quota-stat-label">Total Storage Used</p>
							</div>
						</div>
						
						<div class="quota-stat-card">
							<div class="quota-stat-icon">
								<Activity size={24} class="text-purple-600" />
							</div>
							<div class="quota-stat-content">
								<p class="quota-stat-value">{formatBytes(stats.quota.bandwidthUsed || 0)}</p>
								<p class="quota-stat-label">Total Bandwidth Used</p>
							</div>
						</div>
						
						<div class="quota-stat-card">
							<div class="quota-stat-icon">
								<TrendingUp size={24} class="text-orange-600" />
							</div>
							<div class="quota-stat-content">
								<p class="quota-stat-value">{formatPercentage(stats.quota.storagePercentage || 0)}</p>
								<p class="quota-stat-label">Storage Utilization</p>
							</div>
						</div>
						
						<div class="quota-stat-card">
							<div class="quota-stat-icon">
								<Zap size={24} class="text-yellow-600" />
							</div>
							<div class="quota-stat-content">
								<p class="quota-stat-value">{formatPercentage(stats.quota.bandwidthPercentage || 0)}</p>
								<p class="quota-stat-label">Bandwidth Utilization</p>
							</div>
						</div>
						
						<div class="quota-stat-card">
							<div class="quota-stat-icon">
								<AlertTriangle size={24} class="text-red-600" />
							</div>
							<div class="quota-stat-content">
								<p class="quota-stat-value">{stats.quota.usersNearLimit || 0}</p>
								<p class="quota-stat-label">Users Near Limit</p>
							</div>
						</div>
					</div>
					
					<!-- Overall Usage Summary -->
					<div class="usage-summary">
						<h4>Overall Usage</h4>
						<div class="usage-bars">
							<div class="usage-bar-container">
								<div class="usage-bar-header">
									<span class="usage-bar-label">Total Storage Used</span>
									<span class="usage-bar-value">
										{formatBytes(stats.quota.storageUsed || 0)} / {formatBytes(stats.quota.storageLimit || 0)}
									</span>
								</div>
								<div class="usage-progress-bar">
									<div class="usage-progress-fill storage" style="width: {Math.min(stats.quota.storagePercentage || 0, 100)}%">
										<span class="usage-percentage">{formatPercentage(stats.quota.storagePercentage || 0)}</span>
									</div>
								</div>
							</div>
							
							<div class="usage-bar-container">
								<div class="usage-bar-header">
									<span class="usage-bar-label">Total Bandwidth Used</span>
									<span class="usage-bar-value">
										{formatBytes(stats.quota.bandwidthUsed || 0)} / {formatBytes(stats.quota.bandwidthLimit || 0)}
									</span>
								</div>
								<div class="usage-progress-bar">
									<div class="usage-progress-fill bandwidth" style="width: {Math.min(stats.quota.bandwidthPercentage || 0, 100)}%">
										<span class="usage-percentage">{formatPercentage(stats.quota.bandwidthPercentage || 0)}</span>
									</div>
								</div>
							</div>
						</div>
					</div>
			</div>
		{/if}

		{#if activeTab === 'logs'}
			<!-- Access Logs -->
			<div class="section-card">
				<div class="section-header">
					<h3>Access Logs</h3>
					<div class="filters">
						<select bind:value={logFilters.action} on:change={loadAccessLogs}>
							<option value="">All Actions</option>
							<option value="view">View</option>
							<option value="download">Download</option>
							<option value="upload">Upload</option>
							<option value="delete">Delete</option>
							<option value="share">Share</option>
							<option value="edit">Edit</option>
						</select>
						<input 
							type="number" 
							bind:value={logFilters.limit} 
							on:change={loadAccessLogs}
							placeholder="Limit"
							min="10"
							max="1000"
						/>
						<button class="btn btn-sm" on:click={loadAccessLogs}>
							<Activity size={14} />
							Refresh
						</button>
					</div>
				</div>
				
				{#if accessLogs && accessLogs.length > 0}
					<div class="logs-list">
						{#each accessLogs as log}
							<div class="log-item">
								<div class="log-icon {getActionColor(log.action)}">
									<svelte:component this={getActionIcon(log.action)} size={14} />
								</div>
								<div class="log-details">
									<div class="log-main">
										<span class="log-action">{log.action}</span>
										<span class="log-object">Object: {log.objectId.slice(0, 12)}...</span>
										{#if log.userId}
											<span class="log-user">User: {log.userId.slice(0, 8)}...</span>
										{/if}
									</div>
									<div class="log-meta">
										{#if log.ipAddress}
											<span>IP: {log.ipAddress}</span>
										{/if}
										{#if log.metadata?.success !== undefined}
											{#if log.metadata.success}
												<CheckCircle size={14} class="text-green-600" />
											{:else}
												<XCircle size={14} class="text-red-600" />
											{/if}
										{/if}
										{#if log.metadata?.bytesSize}
											<span>{formatBytes(log.metadata.bytesSize)}</span>
										{/if}
										{#if log.metadata?.durationMs}
											<span>{log.metadata.durationMs}ms</span>
										{/if}
										<span class="log-time">{new Date(log.createdAt).toLocaleString()}</span>
									</div>
								</div>
							</div>
						{/each}
					</div>
				{:else}
					<EmptyState
						icon={Activity}
						title="No access logs found"
						message="Access logs will appear here once users start interacting with files"
					/>
				{/if}
			</div>
		{/if}

		{#if activeTab === 'analytics'}
			<!-- Analytics Dashboard -->
			<div class="analytics-grid">
				{#if stats.access?.actionBreakdown}
					{@const breakdownValues = Object.values(stats.access.actionBreakdown)}
					{@const maxValue = Math.max(...breakdownValues.map(v => Number(v)))}
					<div class="analytics-card">
						<h3>Actions Breakdown</h3>
						<div class="breakdown-list">
							{#each Object.entries(stats.access.actionBreakdown) as [action, count]}
								<div class="breakdown-item">
									<div class="breakdown-label">
										<svelte:component this={getActionIcon(action)} size={14} class={getActionColor(action)} />
										<span>{action}</span>
									</div>
									<div class="breakdown-bar-container">
										<div class="breakdown-bar" style="width: {(Number(count) / maxValue) * 100}%"></div>
										<span class="breakdown-value">{count}</span>
									</div>
								</div>
							{/each}
						</div>
					</div>
				{/if}
				
				<div class="analytics-card">
					<h3>Storage Trends</h3>
					<div class="chart-placeholder">
						<BarChart3 size={48} class="text-cyan-600" />
						<p>Storage growth over time</p>
						<div class="mini-chart">
							<div class="chart-bar" style="height: 40%"></div>
							<div class="chart-bar" style="height: 55%"></div>
							<div class="chart-bar" style="height: 70%"></div>
							<div class="chart-bar" style="height: 85%"></div>
							<div class="chart-bar" style="height: 100%"></div>
						</div>
					</div>
				</div>
				
				<div class="analytics-card">
					<h3>Popular Files</h3>
					<div class="popular-files-list">
						{#if accessLogs && accessLogs.length > 0}
							{#each accessLogs.slice(0, 5) as log}
								<div class="popular-file-item">
									<File size={14} class="text-gray-500" />
									<span class="file-id">{log.objectId.slice(0, 8)}...</span>
									<span class="file-access-count">{log.action}</span>
								</div>
							{/each}
						{:else}
							<p class="text-gray-500">No file access data available</p>
						{/if}
					</div>
				</div>
				
				<div class="analytics-card">
					<h3>User Activity</h3>
					<div class="user-activity-summary">
						{#if stats.access}
							<div class="activity-metric">
								<span class="metric-label">Unique Users</span>
								<span class="metric-value">{stats.access.uniqueUsers || 0}</span>
							</div>
							<div class="activity-metric">
								<span class="metric-label">Total Actions</span>
								<span class="metric-value">{stats.access.totalAccess || 0}</span>
							</div>
							<div class="activity-metric">
								<span class="metric-label">Avg Actions/User</span>
								<span class="metric-value">
									{stats.access.uniqueUsers > 0 ?
										Math.round(stats.access.totalAccess / stats.access.uniqueUsers) : 0}
								</span>
							</div>
						{:else}
							<p class="text-gray-500">No activity data available</p>
						{/if}
					</div>
				</div>
			</div>
		{/if}
		
		{#if activeTab === 'settings'}
			<!-- Settings -->
			<div class="section-card">
				<div class="section-header">
					<h3>Extension Settings</h3>
				</div>
				
				<div class="settings-section">
					<h4>Profile Integration</h4>
					<div class="setting-item">
						<div class="setting-info">
							<label class="setting-label">Show Storage Usage in Profile</label>
							<p class="setting-description">
								When enabled, users will see a "Storage" option in their profile page that displays their storage usage statistics, quotas, and recent file activity.
							</p>
						</div>
						<div class="setting-control">
							<label class="toggle">
								<input 
									type="checkbox" 
									bind:checked={showUsageInProfile}
									on:change={updateProfileUsageSetting}
								/>
								<span class="toggle-slider"></span>
							</label>
						</div>
					</div>
				</div>
				
				<div class="settings-section">
					<h4>Extension Information</h4>
					<div class="info-grid">
						<div class="info-item">
							<span class="info-label">Version:</span>
							<span class="info-value">2.0.0</span>
						</div>
						<div class="info-item">
							<span class="info-label">Status:</span>
							<span class="badge bg-green-100 text-green-700">Active</span>
						</div>
						<div class="info-item">
							<span class="info-label">Database Schema:</span>
							<span class="info-value">ext_cloudstorage</span>
						</div>
					</div>
				</div>
			</div>
		{/if}
	{/if}
</div>

<!-- Share Modal -->
<Modal show={showShareModal} title="Create Share" on:close={() => showShareModal = false}>
	<div class="form-group">
		<label class="form-label">File or Folder</label>
		<div class="file-picker">
			<input
				type="text"
				class="form-input"
				value={selectedObject ? selectedObject.name : ''}
				placeholder="No file selected"
				readonly
			/>
			<button
				class="btn btn-secondary"
				on:click={() => showFileExplorer = true}
			>
				<Folder size={16} />
				Pick File/Folder
			</button>
		</div>
	</div>

	<div class="form-group">
		<label class="form-label">Share Type</label>
		<div class="toggle-switch">
			<button
				class="toggle-option {shareForm.generateToken ? 'active' : ''}"
				on:click={() => shareForm.generateToken = true}
			>
				Generate Share Link
			</button>
			<button
				class="toggle-option {!shareForm.generateToken ? 'active' : ''}"
				on:click={() => shareForm.generateToken = false}
			>
				Share with Email
			</button>
		</div>
	</div>

	{#if !shareForm.generateToken}
		<div class="form-group">
			<label class="form-label">Email Address</label>
			<input
				type="email"
				class="form-input"
				bind:value={shareForm.sharedWithEmail}
				placeholder="user@example.com"
			/>
		</div>
	{/if}

	<div class="form-group">
		<label class="form-label">Permission Level</label>
		<select class="form-select" bind:value={shareForm.permissionLevel}>
			<option value="view">View Only</option>
			<option value="edit">Edit</option>
			<option value="admin">Admin</option>
		</select>
	</div>

	<div class="checkbox-group">
		<label class="checkbox-label">
			<input
				type="checkbox"
				class="form-checkbox"
				bind:checked={shareForm.isPublic}
			/>
			<span>Make Public</span>
		</label>

		<label class="checkbox-label">
			<input
				type="checkbox"
				class="form-checkbox"
				bind:checked={shareForm.inheritToChildren}
			/>
			<span>Apply to Child Objects</span>
		</label>
	</div>

	<div class="form-group">
		<label class="form-label">Expires In (hours)</label>
		<input
			type="number"
			class="form-input"
			bind:value={shareForm.expiresIn}
			min="0"
			placeholder="24"
		/>
	</div>
	<svelte:fragment slot="footer">
		<button class="btn btn-secondary" on:click={() => showShareModal = false}>Cancel</button>
		<button class="btn btn-primary" on:click={createShare}>Create Share</button>
	</svelte:fragment>
</Modal>

<!-- Quota Modal -->
<Modal show={showQuotaModal} title="Set User Quota" on:close={() => { showQuotaModal = false; showSearchDropdown = false; }}>
	<div class="form-group">
		<label class="form-label">Search User</label>
		<div class="user-search">
			<SearchInput
				bind:value={userSearchQuery}
				placeholder="Search by email, name or ID..."
				on:input={searchUsers}
				on:focus={() => {if (searchResults.length > 0) showSearchDropdown = true}}
				maxWidth="100%"
			/>
			{#if searchingUsers}
				<div class="search-loading">
					<LoadingSpinner size="sm" />
				</div>
			{/if}
			{#if showSearchDropdown && searchResults.length > 0}
				<div class="search-results">
					{#each searchResults as user}
						<button
							class="search-result-item"
							on:click={() => selectUser(user)}
						>
							<div class="user-avatar">
								{user.email ? user.email[0].toUpperCase() : 'U'}
							</div>
							<div class="user-info">
								<span class="user-email">{user.email}</span>
								{#if user.name}
									<span class="user-name">{user.name}</span>
								{/if}
							</div>
							<span class="user-id">ID: {user.id.slice(0, 8)}...</span>
						</button>
					{/each}
				</div>
			{/if}
			{#if showSearchDropdown && !searchingUsers && searchResults.length === 0 && userSearchQuery.length >= 2}
				<div class="search-results">
					<div class="no-results">No users found</div>
				</div>
			{/if}
		</div>
		{#if quotaForm.userId}
			<div class="selected-user">
				<CheckCircle size={16} class="text-green-600" />
				Selected: <strong>{userSearchQuery}</strong> (ID: {quotaForm.userId.slice(0, 8)}...)
			</div>
		{/if}
	</div>

	<div class="form-group">
		<label class="form-label">Max Storage (GB)</label>
		<input
			type="number"
			class="form-input"
			bind:value={quotaForm.maxStorageGB}
			min="0.1"
			step="0.1"
		/>
		<div class="form-help">Current: {formatBytes(quotaForm.maxStorageGB * 1024 * 1024 * 1024)}</div>
	</div>

	<div class="form-group">
		<label class="form-label">Max Bandwidth (GB/month)</label>
		<input
			type="number"
			class="form-input"
			bind:value={quotaForm.maxBandwidthGB}
			min="0.1"
			step="0.1"
		/>
		<div class="form-help">Current: {formatBytes(quotaForm.maxBandwidthGB * 1024 * 1024 * 1024)}</div>
	</div>
	<svelte:fragment slot="footer">
		<button class="btn btn-secondary" on:click={() => {showQuotaModal = false; showSearchDropdown = false;}}>Cancel</button>
		<button class="btn btn-primary" on:click={updateQuota} disabled={!quotaForm.userId}>Update Quota</button>
	</svelte:fragment>
</Modal>

<!-- Default Quota Modal -->
<Modal show={showDefaultQuotaModal} title="Default Quota Settings" on:close={() => showDefaultQuotaModal = false}>
	<p class="modal-description">
		Configure default storage and bandwidth quotas for new users.
		These settings will be applied automatically when new users are created.
	</p>

	<div class="form-group">
		<label class="form-label">Default Storage Quota (GB)</label>
		<input
			type="number"
			class="form-input"
			bind:value={defaultQuotas.storage}
			min="0.1"
			step="0.1"
		/>
		<div class="form-help">Amount of storage allocated to each new user</div>
	</div>

	<div class="form-group">
		<label class="form-label">Default Bandwidth Quota (GB/month)</label>
		<input
			type="number"
			class="form-input"
			bind:value={defaultQuotas.bandwidth}
			min="0.1"
			step="0.1"
		/>
		<div class="form-help">Monthly bandwidth limit for each new user</div>
	</div>

	<div class="form-group">
		<label class="checkbox-label">
			<input
				type="checkbox"
				class="form-checkbox"
				bind:checked={defaultQuotas.applyToExisting}
			/>
			<span>Apply to existing users</span>
		</label>
		<div class="form-help">
			{#if defaultQuotas.applyToExisting}
				⚠️ This will update quotas for ALL existing users
			{:else}
				Only new users will receive these quotas
			{/if}
		</div>
	</div>
	<svelte:fragment slot="footer">
		<button class="btn btn-secondary" on:click={() => showDefaultQuotaModal = false}>Cancel</button>
		<button class="btn btn-primary" on:click={updateDefaultQuotas}>
			{defaultQuotas.applyToExisting ? 'Update All Quotas' : 'Save Defaults'}
		</button>
	</svelte:fragment>
</Modal>

<!-- File Explorer Modal -->
{#if showFileExplorer}
	<FileExplorer 
		files={storageObjects}
		showModal={true}
		mode="both"
		title="Select File or Folder to Share"
		on:confirm={handleFileSelect}
		on:cancel={() => showFileExplorer = false}
	/>
{/if}

<style>
	.page-container {
		padding: 1.5rem;
		max-width: 1400px;
		margin: 0 auto;
	}

	.page-header {
		background: white;
		border-radius: 0.5rem;
		padding: 1.5rem;
		margin-bottom: 1.5rem;
		border: 1px solid #e5e7eb;
	}

	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
	}

	.header-title {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 0.5rem;
	}

	.header-title h1 {
		font-size: 1.5rem;
		font-weight: 600;
		color: #111827;
		margin: 0;
	}

	.header-subtitle {
		color: #6b7280;
		font-size: 0.875rem;
		margin: 0;
	}

	.header-actions {
		display: flex;
		gap: 0.75rem;
	}

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: 1rem;
		margin-bottom: 1.5rem;
	}

	.stat-card {
		background: white;
		border-radius: 0.5rem;
		padding: 1.25rem;
		border: 1px solid #e5e7eb;
		display: flex;
		align-items: flex-start;
		gap: 1rem;
	}

	.stat-icon {
		width: 48px;
		height: 48px;
		border-radius: 0.5rem;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.stat-content {
		flex: 1;
	}

	.stat-label {
		font-size: 0.875rem;
		color: #6b7280;
		margin: 0 0 0.25rem 0;
	}

	.stat-value {
		font-size: 1.5rem;
		font-weight: 600;
		color: #111827;
		margin: 0;
	}

	.stat-detail {
		font-size: 0.75rem;
		color: #9ca3af;
		margin: 0.25rem 0 0 0;
	}

	.progress-bar {
		height: 6px;
		background: #e5e7eb;
		border-radius: 3px;
		margin-top: 0.5rem;
		overflow: hidden;
	}

	.progress-fill {
		height: 100%;
		background: linear-gradient(to right, #06b6d4, #0891b2);
		border-radius: 3px;
		transition: width 0.3s ease;
	}

	.progress-fill.bandwidth {
		background: linear-gradient(to right, #8b5cf6, #7c3aed);
	}

	.activity-card, .section-card {
		background: white;
		border-radius: 0.5rem;
		padding: 1.5rem;
		border: 1px solid #e5e7eb;
		margin-bottom: 1.5rem;
	}

	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.5rem;
	}

	.section-header h3,
	.activity-card h3 {
		margin: 0;
		font-size: 1.125rem;
		font-weight: 600;
		color: #111827;
	}

	.activity-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.activity-item {
		display: flex;
		gap: 1rem;
		padding: 0.75rem;
		border-radius: 0.375rem;
		transition: background 0.2s;
	}

	.activity-item:hover {
		background: #f9fafb;
	}

	.activity-icon {
		width: 32px;
		height: 32px;
		border-radius: 0.375rem;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #f3f4f6;
	}

	.activity-details {
		flex: 1;
	}

	.activity-description {
		font-size: 0.875rem;
		color: #374151;
		margin: 0 0 0.25rem 0;
	}

	.file-name {
		color: #06b6d4;
		font-weight: 500;
	}

	.activity-meta {
		display: flex;
		gap: 0.5rem;
		font-size: 0.75rem;
		color: #6b7280;
	}

	.table-container {
		overflow-x: auto;
	}

	.data-table {
		width: 100%;
		border-collapse: collapse;
	}

	.data-table th {
		text-align: left;
		padding: 0.75rem;
		font-size: 0.75rem;
		font-weight: 600;
		color: #6b7280;
		text-transform: uppercase;
		border-bottom: 1px solid #e5e7eb;
	}

	.data-table td {
		padding: 0.75rem;
		font-size: 0.875rem;
		color: #374151;
		border-bottom: 1px solid #f3f4f6;
	}

	.data-table tr:hover {
		background: #f9fafb;
	}

	.badge {
		display: inline-block;
		padding: 0.25rem 0.75rem;
		border-radius: 9999px;
		font-size: 0.75rem;
		font-weight: 500;
	}

	.logs-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.log-item {
		display: flex;
		gap: 0.75rem;
		padding: 0.75rem;
		border: 1px solid #f3f4f6;
		border-radius: 0.375rem;
		transition: all 0.2s;
	}

	.log-item:hover {
		background: #f9fafb;
		border-color: #e5e7eb;
	}

	.log-icon {
		width: 28px;
		height: 28px;
		border-radius: 0.25rem;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #f9fafb;
	}

	.log-details {
		flex: 1;
	}

	.log-main {
		display: flex;
		gap: 0.75rem;
		align-items: center;
		margin-bottom: 0.25rem;
	}

	.log-action {
		font-weight: 600;
		text-transform: uppercase;
		font-size: 0.75rem;
		color: #111827;
	}

	.log-object, .log-user {
		font-size: 0.813rem;
		color: #6b7280;
	}

	.log-meta {
		display: flex;
		gap: 0.75rem;
		align-items: center;
		font-size: 0.75rem;
		color: #9ca3af;
	}

	.log-time {
		margin-left: auto;
	}

	.filters {
		display: flex;
		gap: 0.75rem;
		align-items: center;
	}

	.filters select,
	.filters input {
		padding: 0.375rem 0.75rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		font-size: 0.813rem;
		background: white;
	}

	.analytics-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(350px, 1fr));
		gap: 1.5rem;
	}

	.analytics-card {
		background: white;
		border-radius: 0.5rem;
		padding: 1.5rem;
		border: 1px solid #e5e7eb;
	}

	.analytics-card h3 {
		margin: 0 0 1rem 0;
		font-size: 1.125rem;
		font-weight: 600;
		color: #111827;
	}

	.breakdown-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.breakdown-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.5rem;
		border-radius: 0.375rem;
		background: #f9fafb;
	}

	.breakdown-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.875rem;
		color: #374151;
		min-width: 120px;
	}

	.breakdown-bar-container {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		flex: 1;
	}

	.breakdown-bar {
		height: 8px;
		background: linear-gradient(to right, #06b6d4, #0891b2);
		border-radius: 4px;
		min-width: 20px;
	}

	.breakdown-value {
		font-weight: 600;
		color: #111827;
		min-width: 40px;
		text-align: right;
	}

	.btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		border: none;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-primary {
		background: #06b6d4;
		color: white;
	}

	.btn-primary:hover {
		background: #0891b2;
	}

	.btn-primary:disabled {
		background: #9ca3af;
		cursor: not-allowed;
	}

	.btn-secondary {
		background: white;
		color: #374151;
		border: 1px solid #e5e7eb;
	}

	.btn-secondary:hover {
		background: #f9fafb;
	}

	.btn-danger {
		background: #ef4444;
		color: white;
	}

	.btn-danger:hover {
		background: #dc2626;
	}

	.btn-sm {
		padding: 0.375rem 0.75rem;
		font-size: 0.813rem;
	}

	.btn-xs {
		padding: 0.25rem 0.5rem;
		font-size: 0.75rem;
	}

	.loading {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 4rem;
		color: #6b7280;
		font-size: 1rem;
	}

	.form-group {
		margin-bottom: 1.25rem;
	}

	.form-label {
		display: block;
		margin-bottom: 0.5rem;
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
	}

	.form-input,
	.form-select {
		width: 100%;
		padding: 0.625rem 0.875rem;
		border: 1px solid #d1d5db;
		border-radius: 0.5rem;
		font-size: 0.875rem;
		color: #111827;
		background: white;
		transition: all 0.2s;
	}

	.form-input:focus,
	.form-select:focus {
		outline: none;
		border-color: #06b6d4;
		box-shadow: 0 0 0 3px rgba(6, 182, 212, 0.1);
	}

	.form-input::placeholder {
		color: #9ca3af;
	}

	.toggle-switch {
		display: flex;
		background: #f3f4f6;
		border-radius: 0.5rem;
		padding: 0.125rem;
		gap: 0.125rem;
	}

	.toggle-option {
		flex: 1;
		padding: 0.625rem 1rem;
		border: none;
		background: transparent;
		color: #6b7280;
		font-size: 0.875rem;
		font-weight: 500;
		border-radius: 0.375rem;
		cursor: pointer;
		transition: all 0.2s;
		white-space: nowrap;
	}

	.toggle-option.active {
		background: #06b6d4;
		color: white;
		box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.1);
	}

	.toggle-option:not(.active):hover {
		color: #374151;
	}

	.checkbox-group {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		margin-bottom: 1.25rem;
	}

	.checkbox-label {
		display: flex;
		align-items: center;
		cursor: pointer;
		font-size: 0.875rem;
		color: #374151;
		user-select: none;
	}

	.form-checkbox {
		width: 18px;
		height: 18px;
		margin-right: 0.625rem;
		border: 1px solid #d1d5db;
		border-radius: 0.25rem;
		cursor: pointer;
		accent-color: #06b6d4;
	}

	.form-checkbox:checked {
		background-color: #06b6d4;
		border-color: #06b6d4;
	}
	
	.file-picker {
		display: flex;
		gap: 0.5rem;
	}
	
	.file-picker .form-input {
		flex: 1;
		background: var(--bg-secondary);
	}
	
	.file-picker .btn {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		white-space: nowrap;
	}

	.checkbox-label span {
		flex: 1;
	}

	.form-help {
		font-size: 0.75rem;
		color: #6b7280;
		margin-top: 0.25rem;
	}

	.truncate {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		max-width: 150px;
	}

	@media (max-width: 768px) {
		.stats-grid {
			grid-template-columns: 1fr;
		}
		
		.analytics-grid {
			grid-template-columns: 1fr;
		}
		
		.filters {
			flex-wrap: wrap;
		}
		
		.table-container {
			font-size: 0.75rem;
		}
	}

	/* Description Card Styles */
	.description-card {
		background: white;
		border-radius: 0.75rem;
		padding: 1.5rem;
		margin-bottom: 1.5rem;
		border: 1px solid #e5e7eb;
	}

	.description-card h3 {
		margin: 0 0 0.75rem 0;
		font-size: 1.125rem;
		font-weight: 600;
		color: #111827;
	}

	.description-card > p {
		color: #6b7280;
		margin: 0 0 1.5rem 0;
		line-height: 1.6;
	}

	.features-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
		gap: 1.5rem;
	}

	.feature-item {
		display: flex;
		gap: 1rem;
	}

	.feature-item > :global(svg) {
		flex-shrink: 0;
		color: #06b6d4;
		margin-top: 0.125rem;
	}

	.feature-item h4 {
		margin: 0 0 0.25rem 0;
		font-size: 0.875rem;
		font-weight: 600;
		color: #111827;
	}

	.feature-item p {
		margin: 0;
		font-size: 0.8125rem;
		color: #6b7280;
		line-height: 1.5;
	}

	/* User Search Styles */
	.user-search {
		position: relative;
	}

	.search-input-container {
		position: relative;
		display: flex;
		align-items: center;
	}

	.search-icon {
		position: absolute;
		left: 12px;
		color: #9ca3af;
		pointer-events: none;
	}

	.search-input {
		padding-left: 2.5rem !important;
	}

	.search-loading {
		position: absolute;
		right: 12px;
		top: 50%;
		transform: translateY(-50%);
	}


	.search-results {
		position: absolute;
		top: 100%;
		left: 0;
		right: 0;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		margin-top: 0.25rem;
		max-height: 240px;
		overflow-y: auto;
		box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1);
		z-index: 10;
	}

	.search-result-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem;
		border: none;
		background: none;
		width: 100%;
		text-align: left;
		cursor: pointer;
		transition: background-color 0.15s;
		border-bottom: 1px solid #f3f4f6;
	}

	.search-result-item:last-child {
		border-bottom: none;
	}

	.search-result-item:hover {
		background-color: #f9fafb;
	}

	.user-avatar {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
		color: white;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.875rem;
		font-weight: 600;
		flex-shrink: 0;
	}

	.user-info {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
		flex: 1;
	}

	.user-email {
		font-size: 0.875rem;
		color: #111827;
		font-weight: 500;
	}

	.user-name {
		font-size: 0.75rem;
		color: #6b7280;
	}

	.user-id {
		font-size: 0.75rem;
		color: #9ca3af;
	}

	.selected-user {
		margin-top: 0.5rem;
		padding: 0.5rem;
		background: #f0fdf4;
		border: 1px solid #86efac;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		color: #166534;
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.no-results {
		padding: 1rem;
		text-align: center;
		color: #6b7280;
		font-size: 0.875rem;
	}

	/* Default Quotas Styles */
	.default-quotas-info {
		background: #f9fafb;
		padding: 1rem;
		border-radius: 0.375rem;
		margin-bottom: 1.5rem;
	}

	.default-quotas-info h4 {
		margin: 0 0 0.75rem 0;
		font-size: 0.875rem;
		font-weight: 600;
		color: #374151;
	}

	.quota-defaults {
		display: flex;
		gap: 2rem;
	}

	.default-item {
		display: flex;
		gap: 0.5rem;
		align-items: center;
	}

	.default-item .label {
		font-size: 0.8125rem;
		color: #6b7280;
	}

	.default-item .value {
		font-size: 0.875rem;
		font-weight: 600;
		color: #111827;
	}

	.modal-description {
		margin: 0 0 1.5rem 0;
		color: #6b7280;
		font-size: 0.875rem;
		line-height: 1.5;
	}

	.header-actions {
		display: flex;
		gap: 0.5rem;
	}
	
	/* Settings Styles */
	.settings-section {
		margin-bottom: 2rem;
	}
	
	.settings-section:last-child {
		margin-bottom: 0;
	}
	
	.settings-section h4 {
		margin: 0 0 1rem 0;
		font-size: 0.9375rem;
		font-weight: 600;
		color: #374151;
	}
	
	.setting-item {
		display: flex;
		justify-content: space-between;
		align-items: start;
		padding: 1rem;
		background: #f9fafb;
		border-radius: 0.5rem;
		margin-bottom: 0.75rem;
	}
	
	.setting-info {
		flex: 1;
		margin-right: 2rem;
	}
	
	.setting-label {
		display: block;
		font-size: 0.875rem;
		font-weight: 600;
		color: #111827;
		margin-bottom: 0.25rem;
	}
	
	.setting-description {
		margin: 0;
		font-size: 0.813rem;
		color: #6b7280;
		line-height: 1.5;
	}
	
	.setting-control {
		display: flex;
		align-items: center;
	}
	
	.toggle {
		position: relative;
		display: inline-block;
		width: 48px;
		height: 24px;
		cursor: pointer;
	}
	
	.toggle input {
		opacity: 0;
		width: 0;
		height: 0;
	}
	
	.toggle-slider {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background-color: #d1d5db;
		transition: 0.3s;
		border-radius: 24px;
	}
	
	.toggle-slider:before {
		position: absolute;
		content: "";
		height: 18px;
		width: 18px;
		left: 3px;
		bottom: 3px;
		background-color: white;
		transition: 0.3s;
		border-radius: 50%;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12);
	}
	
	.toggle input:checked + .toggle-slider {
		background-color: #06b6d4;
	}
	
	.toggle input:checked + .toggle-slider:before {
		transform: translateX(24px);
	}
	
	.info-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1rem;
	}
	
	.info-item {
		display: flex;
		gap: 0.5rem;
		font-size: 0.875rem;
	}
	
	.info-label {
		color: #6b7280;
	}
	
	.info-value {
		color: #111827;
		font-weight: 500;
	}

	/* Share Statistics Grid */
	.share-stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1rem;
		margin-bottom: 2rem;
	}

	.share-stat-card {
		background: #f9fafb;
		border-radius: 0.5rem;
		padding: 1rem;
		display: flex;
		align-items: center;
		gap: 1rem;
		border: 1px solid #e5e7eb;
		transition: all 0.2s;
	}

	.share-stat-card:hover {
		border-color: #06b6d4;
		transform: translateY(-2px);
	}

	.share-stat-icon {
		width: 40px;
		height: 40px;
		border-radius: 0.375rem;
		background: white;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.share-stat-content {
		flex: 1;
	}

	.share-stat-value {
		font-size: 1.25rem;
		font-weight: 600;
		color: #111827;
		margin: 0 0 0.125rem 0;
	}

	.share-stat-label {
		font-size: 0.75rem;
		color: #6b7280;
		margin: 0;
	}

	/* Quota Statistics Grid */
	.quota-stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1rem;
		margin-bottom: 2rem;
	}

	.quota-stat-card {
		background: #f9fafb;
		border-radius: 0.5rem;
		padding: 1rem;
		display: flex;
		align-items: center;
		gap: 1rem;
		border: 1px solid #e5e7eb;
		transition: all 0.2s;
	}

	.quota-stat-card:hover {
		border-color: #06b6d4;
		transform: translateY(-2px);
	}

	.quota-stat-icon {
		width: 40px;
		height: 40px;
		border-radius: 0.375rem;
		background: white;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.quota-stat-content {
		flex: 1;
	}

	.quota-stat-value {
		font-size: 1.25rem;
		font-weight: 600;
		color: #111827;
		margin: 0 0 0.125rem 0;
	}

	.quota-stat-label {
		font-size: 0.75rem;
		color: #6b7280;
		margin: 0;
	}

	/* Usage Summary */
	.usage-summary {
		background: #f0fdf4;
		border: 1px solid #86efac;
		border-radius: 0.5rem;
		padding: 1.25rem;
		margin-bottom: 1.5rem;
	}

	.usage-summary h4 {
		margin: 0 0 1rem 0;
		font-size: 1rem;
		font-weight: 600;
		color: #166534;
	}

	.usage-bars {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.usage-bar-container {
		background: white;
		padding: 0.75rem;
		border-radius: 0.375rem;
	}

	.usage-bar-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.5rem;
	}

	.usage-bar-label {
		font-size: 0.813rem;
		font-weight: 500;
		color: #374151;
	}

	.usage-bar-value {
		font-size: 0.813rem;
		color: #6b7280;
	}

	.usage-progress-bar {
		height: 24px;
		background: #e5e7eb;
		border-radius: 12px;
		overflow: hidden;
		position: relative;
	}

	.usage-progress-fill {
		height: 100%;
		border-radius: 12px;
		display: flex;
		align-items: center;
		justify-content: flex-end;
		padding-right: 0.5rem;
		transition: width 0.3s ease;
		position: relative;
	}

	.usage-progress-fill.storage {
		background: linear-gradient(to right, #06b6d4, #0891b2);
	}

	.usage-progress-fill.bandwidth {
		background: linear-gradient(to right, #8b5cf6, #7c3aed);
	}

	.usage-percentage {
		color: white;
		font-size: 0.75rem;
		font-weight: 600;
		text-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
	}

	.usage-summary-detail {
		margin: 1rem 0 0 0;
		font-size: 0.813rem;
		color: #166534;
	}

	/* Quotas List */
	.quotas-list {
		margin-top: 1.5rem;
	}

	.quotas-list h4 {
		margin: 0 0 1rem 0;
		font-size: 1rem;
		font-weight: 600;
		color: #111827;
	}

	.quota-cell {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.mini-progress-bar {
		height: 4px;
		background: #e5e7eb;
		border-radius: 2px;
		overflow: hidden;
	}

	.mini-progress-fill {
		height: 100%;
		border-radius: 2px;
		transition: width 0.3s ease;
	}

	.mini-progress-fill.storage {
		background: #06b6d4;
	}

	.mini-progress-fill.bandwidth {
		background: #8b5cf6;
	}

	/* Analytics Enhancements */
	.chart-placeholder {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 2rem;
		text-align: center;
	}

	.chart-placeholder p {
		margin: 1rem 0;
		color: #6b7280;
		font-size: 0.875rem;
	}

	.mini-chart {
		display: flex;
		align-items: flex-end;
		gap: 0.5rem;
		height: 60px;
		margin-top: 1rem;
	}

	.chart-bar {
		width: 12px;
		background: linear-gradient(to top, #06b6d4, #0891b2);
		border-radius: 2px 2px 0 0;
		animation: growUp 0.5s ease-out;
	}

	@keyframes growUp {
		from {
			height: 0;
		}
	}

	.popular-files-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.popular-file-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem;
		background: #f9fafb;
		border-radius: 0.375rem;
	}

	.file-id {
		flex: 1;
		font-size: 0.813rem;
		color: #374151;
	}

	.file-access-count {
		font-size: 0.75rem;
		color: #6b7280;
		text-transform: uppercase;
	}

	.user-activity-summary {
		display: grid;
		grid-template-columns: 1fr;
		gap: 1rem;
	}

	.activity-metric {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem;
		background: #f9fafb;
		border-radius: 0.375rem;
	}

	.metric-label {
		font-size: 0.813rem;
		color: #6b7280;
	}

	.metric-value {
		font-size: 1.125rem;
		font-weight: 600;
		color: #111827;
	}
</style>