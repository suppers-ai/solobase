<script lang="ts">
	import { onMount } from 'svelte';
	import { createEventDispatcher } from 'svelte';
	import { api, ErrorHandler } from '$lib/api';
	import { formatBytes } from '$lib/utils/formatters';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import Modal from '$lib/components/ui/Modal.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import TabNavigation from '$lib/components/ui/TabNavigation.svelte';

	const quotaTabs = [
		{ id: 'roles', label: 'Role Quotas' },
		{ id: 'overrides', label: 'User Overrides' }
	];

	interface Role {
		id: string;
		name: string;
	}

	interface RoleQuota {
		roleId: string;
		roleName: string;
		maxStorageBytes: number;
		maxBandwidthBytes: number;
		maxUploadSize: number;
		maxFilesCount: number;
		allowedExtensions?: string;
		blockedExtensions?: string;
	}

	interface UserOverride {
		id: string;
		userId: string;
		maxStorageBytes?: number | null;
		maxBandwidthBytes?: number | null;
		maxUploadSize?: number | null;
		maxFilesCount?: number | null;
		allowedExtensions?: string | null;
		blockedExtensions?: string | null;
		reason?: string;
		expiresAt?: string | null;
	}

	const dispatch = createEventDispatcher();

	export let roles: Role[] = [];

	let roleQuotas: RoleQuota[] = [];
	let userOverrides: UserOverride[] = [];
	let loading = true;
	let activeView = 'roles'; // 'roles' or 'overrides'
	let showEditModal = false;
	let showOverrideModal = false;
	let showDeleteConfirm = false;
	let selectedQuota: RoleQuota | null = null;
	let overrideToDelete: UserOverride | null = null;
	
	// Form data
	let quotaForm = {
		roleId: '',
		roleName: '',
		maxStorageBytes: 5368709120, // 5GB default
		maxBandwidthBytes: 10737418240, // 10GB default
		maxUploadSize: 104857600, // 100MB default
		maxFilesCount: 1000,
		allowedExtensions: '',
		blockedExtensions: ''
	};
	
	let overrideForm = {
		userId: '',
		maxStorageBytes: null,
		maxBandwidthBytes: null,
		maxUploadSize: null,
		maxFilesCount: null,
		allowedExtensions: null,
		blockedExtensions: null,
		reason: '',
		expiresAt: null
	};
	
	onMount(() => {
		loadQuotas();
	});
	
	async function loadQuotas() {
		loading = true;
		try {
			// Load role quotas
			try {
				const quotasResult = await api.get<RoleQuota[]>('/admin/ext/cloudstorage/quotas/roles');
				roleQuotas = Array.isArray(quotasResult) ? quotasResult : [];
			} catch {
				roleQuotas = [];
			}

			// Load user overrides
			try {
				const overridesResult = await api.get<UserOverride[]>('/admin/ext/cloudstorage/quotas/overrides');
				userOverrides = Array.isArray(overridesResult) ? overridesResult : [];
			} catch {
				userOverrides = [];
			}
		} finally {
			loading = false;
		}
	}

	function handleEditQuota(quota: RoleQuota) {
		selectedQuota = quota;
		quotaForm = {
			roleId: quota.roleId,
			roleName: quota.roleName,
			maxStorageBytes: quota.maxStorageBytes,
			maxBandwidthBytes: quota.maxBandwidthBytes,
			maxUploadSize: quota.maxUploadSize,
			maxFilesCount: quota.maxFilesCount,
			allowedExtensions: quota.allowedExtensions || '',
			blockedExtensions: quota.blockedExtensions || ''
		};
		showEditModal = true;
	}

	async function saveQuota() {
		try {
			await api.put(`/admin/ext/cloudstorage/quotas/roles/${quotaForm.roleId}`, {
				maxStorageBytes: quotaForm.maxStorageBytes,
				maxBandwidthBytes: quotaForm.maxBandwidthBytes,
				maxUploadSize: quotaForm.maxUploadSize,
				maxFilesCount: quotaForm.maxFilesCount,
				allowedExtensions: quotaForm.allowedExtensions,
				blockedExtensions: quotaForm.blockedExtensions
			});

			showEditModal = false;
			await loadQuotas();
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}
	
	async function createOverride() {
		try {
			await api.post('/admin/ext/cloudstorage/quotas/overrides', overrideForm);

			showOverrideModal = false;
			await loadQuotas();
			// Reset form
			overrideForm = {
				userId: '',
				maxStorageBytes: null,
				maxBandwidthBytes: null,
				maxUploadSize: null,
				maxFilesCount: null,
				allowedExtensions: null,
				blockedExtensions: null,
				reason: '',
				expiresAt: null
			};
		} catch (error) {
			ErrorHandler.handle(error);
		}
	}
	
	function deleteOverride(override: UserOverride) {
		overrideToDelete = override;
		showDeleteConfirm = true;
	}

	async function confirmDeleteOverride() {
		if (!overrideToDelete) return;
		showDeleteConfirm = false;

		try {
			await api.delete(`/admin/ext/cloudstorage/quotas/overrides/${overrideToDelete.id}`);
			await loadQuotas();
		} catch (error) {
			ErrorHandler.handle(error);
		}
		overrideToDelete = null;
	}

	function formatNumber(num: number | null | undefined): string {
		return num?.toLocaleString() || '0';
	}
</script>

<div class="quota-manager">
	<TabNavigation tabs={quotaTabs} bind:activeTab={activeView} />
	
	{#if loading}
		<div class="loading">Loading quotas...</div>
	{:else if activeView === 'roles'}
		<div class="section">
			<div class="section-header">
				<h3>Role-Based Quotas</h3>
				<p class="subtitle">Define storage limits and restrictions for each role</p>
			</div>
			
			<div class="quota-grid">
				{#each roleQuotas as quota}
					<div class="quota-card">
						<div class="quota-header">
							<h4>{quota.roleName}</h4>
							<button class="btn-edit" on:click={() => handleEditQuota(quota)}>
								Edit
							</button>
						</div>

						<div class="quota-details">
							<div class="quota-item">
								<span class="label">Storage:</span>
								<span class="value">{formatBytes(quota.maxStorageBytes)}</span>
							</div>
							<div class="quota-item">
								<span class="label">Bandwidth:</span>
								<span class="value">{formatBytes(quota.maxBandwidthBytes)}</span>
							</div>
							<div class="quota-item">
								<span class="label">Max Upload:</span>
								<span class="value">{formatBytes(quota.maxUploadSize)}</span>
							</div>
							<div class="quota-item">
								<span class="label">Max Files:</span>
								<span class="value">{formatNumber(quota.maxFilesCount)}</span>
							</div>

							{#if quota.allowedExtensions}
								<div class="quota-item full-width">
									<span class="label">Allowed:</span>
									<span class="value extensions">{quota.allowedExtensions}</span>
								</div>
							{/if}

							{#if quota.blockedExtensions}
								<div class="quota-item full-width">
									<span class="label">Blocked:</span>
									<span class="value extensions blocked">{quota.blockedExtensions}</span>
								</div>
							{/if}
						</div>
					</div>
				{/each}
				
				{#if roleQuotas.length === 0}
					<EmptyState
						title="No role quotas configured yet"
						message="Role quotas will be initialized when the CloudStorage extension starts."
						compact
					/>
				{/if}
			</div>
		</div>
	{:else if activeView === 'overrides'}
		<div class="section">
			<div class="section-header">
				<h3>User-Specific Overrides</h3>
				<button class="btn btn-primary" on:click={() => showOverrideModal = true}>
					Add Override
				</button>
			</div>
			
			{#if userOverrides.length > 0}
				<div class="overrides-table">
					<table>
						<thead>
							<tr>
								<th>User ID</th>
								<th>Storage</th>
								<th>Bandwidth</th>
								<th>Upload Size</th>
								<th>Max Files</th>
								<th>Reason</th>
								<th>Expires</th>
								<th>Actions</th>
							</tr>
						</thead>
						<tbody>
							{#each userOverrides as override}
								<tr>
									<td>{override.userId}</td>
									<td>{override.maxStorageBytes ? formatBytes(override.maxStorageBytes) : '-'}</td>
									<td>{override.maxBandwidthBytes ? formatBytes(override.maxBandwidthBytes) : '-'}</td>
									<td>{override.maxUploadSize ? formatBytes(override.maxUploadSize) : '-'}</td>
									<td>{override.maxFilesCount ? formatNumber(override.maxFilesCount) : '-'}</td>
									<td class="reason">{override.reason || '-'}</td>
									<td>{override.expiresAt ? new Date(override.expiresAt).toLocaleDateString() : 'Never'}</td>
									<td>
										<button class="btn-delete" on:click={() => deleteOverride(override)}>
											Delete
										</button>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{:else}
				<EmptyState
					title="No user overrides configured"
					message="User overrides allow you to set custom quotas for specific users."
					compact
				/>
			{/if}
		</div>
	{/if}
</div>

<!-- Edit Role Quota Modal -->
<Modal show={showEditModal} title="Edit Quota: {quotaForm.roleName}" maxWidth="600px" on:close={() => showEditModal = false}>
	<div class="form-grid">
		<div class="form-group">
			<label for="storage">Storage Limit (bytes)</label>
			<input
				id="storage"
				type="number"
				bind:value={quotaForm.maxStorageBytes}
				min="0"
			/>
			<small>{formatBytes(quotaForm.maxStorageBytes)}</small>
		</div>

		<div class="form-group">
			<label for="bandwidth">Bandwidth Limit (bytes)</label>
			<input
				id="bandwidth"
				type="number"
				bind:value={quotaForm.maxBandwidthBytes}
				min="0"
			/>
			<small>{formatBytes(quotaForm.maxBandwidthBytes)}</small>
		</div>

		<div class="form-group">
			<label for="upload">Max Upload Size (bytes)</label>
			<input
				id="upload"
				type="number"
				bind:value={quotaForm.maxUploadSize}
				min="0"
			/>
			<small>{formatBytes(quotaForm.maxUploadSize)}</small>
		</div>

		<div class="form-group">
			<label for="files">Max Files Count</label>
			<input
				id="files"
				type="number"
				bind:value={quotaForm.maxFilesCount}
				min="0"
			/>
		</div>

		<div class="form-group full-width">
			<label for="allowed">Allowed Extensions (comma-separated)</label>
			<input
				id="allowed"
				type="text"
				bind:value={quotaForm.allowedExtensions}
				placeholder="jpg,png,pdf,doc"
			/>
			<small>Leave empty to allow all</small>
		</div>

		<div class="form-group full-width">
			<label for="blocked">Blocked Extensions (comma-separated)</label>
			<input
				id="blocked"
				type="text"
				bind:value={quotaForm.blockedExtensions}
				placeholder="exe,bat,sh"
			/>
			<small>Leave empty to block none</small>
		</div>
	</div>
	<svelte:fragment slot="footer">
		<button class="btn" on:click={() => showEditModal = false}>Cancel</button>
		<button class="btn btn-primary" on:click={saveQuota}>Save Changes</button>
	</svelte:fragment>
</Modal>

<!-- Create User Override Modal -->
<Modal show={showOverrideModal} title="Create User Override" maxWidth="600px" on:close={() => showOverrideModal = false}>
	<div class="form-grid">
		<div class="form-group full-width">
			<label for="user-id">User ID *</label>
			<input
				id="user-id"
				type="text"
				bind:value={overrideForm.userId}
				placeholder="Enter user ID"
				required
			/>
		</div>

		<div class="form-group">
			<label for="override-storage">Storage Limit (bytes)</label>
			<input
				id="override-storage"
				type="number"
				bind:value={overrideForm.maxStorageBytes}
				min="0"
				placeholder="Leave empty for no override"
			/>
		</div>

		<div class="form-group">
			<label for="override-bandwidth">Bandwidth Limit (bytes)</label>
			<input
				id="override-bandwidth"
				type="number"
				bind:value={overrideForm.maxBandwidthBytes}
				min="0"
				placeholder="Leave empty for no override"
			/>
		</div>

		<div class="form-group">
			<label for="override-upload">Max Upload Size (bytes)</label>
			<input
				id="override-upload"
				type="number"
				bind:value={overrideForm.maxUploadSize}
				min="0"
				placeholder="Leave empty for no override"
			/>
		</div>

		<div class="form-group">
			<label for="override-files">Max Files Count</label>
			<input
				id="override-files"
				type="number"
				bind:value={overrideForm.maxFilesCount}
				min="0"
				placeholder="Leave empty for no override"
			/>
		</div>

		<div class="form-group full-width">
			<label for="override-reason">Reason *</label>
			<textarea
				id="override-reason"
				bind:value={overrideForm.reason}
				placeholder="Explain why this override is needed"
				rows="3"
				required
			/>
		</div>

		<div class="form-group">
			<label for="override-expires">Expires At</label>
			<input
				id="override-expires"
				type="datetime-local"
				bind:value={overrideForm.expiresAt}
			/>
			<small>Leave empty for permanent override</small>
		</div>
	</div>
	<svelte:fragment slot="footer">
		<button class="btn" on:click={() => showOverrideModal = false}>Cancel</button>
		<button
			class="btn btn-primary"
			on:click={createOverride}
			disabled={!overrideForm.userId || !overrideForm.reason}
		>
			Create Override
		</button>
	</svelte:fragment>
</Modal>

<ConfirmDialog
	bind:show={showDeleteConfirm}
	title="Delete Override"
	message="Are you sure you want to delete this user quota override?"
	confirmText="Delete"
	variant="danger"
	on:confirm={confirmDeleteOverride}
/>

<style>
	.quota-manager {
		padding: 1rem;
	}
	
	.section {
		background: white;
		border-radius: 8px;
		padding: 2rem;
		box-shadow: 0 2px 4px rgba(0,0,0,0.1);
	}
	
	.section-header {
		margin-bottom: 2rem;
		display: flex;
		justify-content: space-between;
		align-items: start;
	}
	
	.section-header h3 {
		margin: 0 0 0.5rem;
		color: #333;
	}
	
	.subtitle {
		margin: 0;
		color: #666;
		font-size: 0.9rem;
	}
	
	.quota-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
		gap: 1.5rem;
	}
	
	.quota-card {
		border: 1px solid #e0e0e0;
		border-radius: 8px;
		padding: 1.5rem;
		background: #f9f9f9;
	}
	
	.quota-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid #e0e0e0;
	}
	
	.quota-header h4 {
		margin: 0;
		text-transform: capitalize;
	}
	
	.quota-details {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.75rem;
	}
	
	.quota-item {
		display: flex;
		justify-content: space-between;
		font-size: 0.9rem;
	}
	
	.quota-item.full-width {
		grid-column: 1 / -1;
	}
	
	.quota-item .label {
		color: #666;
		font-weight: 500;
	}
	
	.quota-item .value {
		color: #333;
		font-weight: 600;
	}
	
	.quota-item .extensions {
		font-family: monospace;
		font-size: 0.85rem;
		color: #4CAF50;
	}
	
	.quota-item .extensions.blocked {
		color: #f44336;
	}
	
	.btn-edit {
		padding: 0.25rem 0.75rem;
		background: #4CAF50;
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.85rem;
	}
	
	.btn-edit:hover {
		background: #45a049;
	}
	
	.btn-delete {
		padding: 0.25rem 0.75rem;
		background: #f44336;
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.85rem;
	}
	
	.btn-delete:hover {
		background: #da190b;
	}
	
	.overrides-table {
		overflow-x: auto;
	}
	
	.overrides-table table {
		width: 100%;
		border-collapse: collapse;
	}
	
	.overrides-table th,
	.overrides-table td {
		padding: 0.75rem;
		text-align: left;
		border-bottom: 1px solid #e0e0e0;
	}
	
	.overrides-table th {
		background: #f5f5f5;
		font-weight: 600;
		color: #333;
	}
	
	.overrides-table .reason {
		max-width: 200px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	
	.loading {
		text-align: center;
		padding: 3rem;
		color: #666;
	}
	
	.form-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
		margin-bottom: 1.5rem;
	}
	
	.form-group {
		display: flex;
		flex-direction: column;
	}
	
	.form-group.full-width {
		grid-column: 1 / -1;
	}
	
	.form-group label {
		margin-bottom: 0.5rem;
		font-weight: 500;
		color: #333;
	}
	
	.form-group input,
	.form-group textarea {
		padding: 0.5rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		font-size: 1rem;
	}
	
	.form-group small {
		margin-top: 0.25rem;
		color: #666;
		font-size: 0.85rem;
	}
	
	.btn {
		padding: 0.5rem 1rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		background: white;
		color: #333;
		cursor: pointer;
		transition: all 0.3s;
	}
	
	.btn:hover:not(:disabled) {
		background: #f5f5f5;
	}
	
	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	
	.btn-primary {
		background: #4CAF50;
		color: white;
		border-color: #4CAF50;
	}
	
	.btn-primary:hover:not(:disabled) {
		background: #45a049;
	}
</style>