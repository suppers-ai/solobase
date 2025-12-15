<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { HardDrive, TrendingUp, Download, Upload, Share2 } from 'lucide-svelte';
	import Modal from '$lib/components/ui/Modal.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import { formatBytes } from '$lib/utils/formatters';

	export let show = false;
	export let storageStats: any = null;
	export let storageQuota: any = null;
	export let recentActivity: any[] = [];
	export let isAdmin = false;

	const dispatch = createEventDispatcher();

	function handleClose() {
		dispatch('close');
	}

	function getActionIcon(action: string) {
		switch (action) {
			case 'upload': return Upload;
			case 'download': return Download;
			case 'share': return Share2;
			default: return HardDrive;
		}
	}

	function getActionColor(action: string) {
		switch (action) {
			case 'upload': return 'upload';
			case 'download': return 'download';
			case 'share': return 'share';
			default: return '';
		}
	}
</script>

<Modal {show} title="Storage Usage" maxWidth="600px" on:close={handleClose}>
	<!-- Storage Overview -->
	<div class="storage-overview">
		<div class="storage-stat-card">
			<div class="stat-icon storage-icon">
				<HardDrive size={20} />
			</div>
			<div class="stat-details">
				<span class="stat-label">Storage Used</span>
				<span class="stat-value">
					{#if storageQuota}
						{formatBytes(storageQuota.storageUsed || 0)}
					{:else}
						Loading...
					{/if}
				</span>
				{#if storageQuota && storageQuota.maxStorageBytes}
					<div class="progress-bar">
						<div class="progress-fill" style="width: {Math.min((storageQuota.storageUsed / storageQuota.maxStorageBytes) * 100, 100)}%"></div>
					</div>
					<span class="stat-detail">
						of {formatBytes(storageQuota.maxStorageBytes)} available
					</span>
				{/if}
			</div>
		</div>

		<div class="storage-stat-card">
			<div class="stat-icon bandwidth-icon">
				<TrendingUp size={20} />
			</div>
			<div class="stat-details">
				<span class="stat-label">Bandwidth Used</span>
				<span class="stat-value">
					{#if storageQuota}
						{formatBytes(storageQuota.bandwidthUsed || 0)}
					{:else}
						Loading...
					{/if}
				</span>
				{#if storageQuota && storageQuota.maxBandwidthBytes}
					<div class="progress-bar">
						<div class="progress-fill bandwidth" style="width: {Math.min((storageQuota.bandwidthUsed / storageQuota.maxBandwidthBytes) * 100, 100)}%"></div>
					</div>
					<span class="stat-detail">
						of {formatBytes(storageQuota.maxBandwidthBytes)} this month
					</span>
				{/if}
			</div>
		</div>
	</div>

	<!-- Storage Details -->
	{#if storageStats || storageQuota}
		<div class="storage-details">
			<h4>Storage Details</h4>
			<div class="detail-grid">
				<div class="detail-item">
					<span class="detail-label">Total Files:</span>
					<span class="detail-value">{storageStats?.storage?.totalObjects || 0}</span>
				</div>
				<div class="detail-item">
					<span class="detail-label">Shared Files:</span>
					<span class="detail-value">{storageStats?.shares?.totalShares || 0}</span>
				</div>
				{#if storageQuota?.resetBandwidthAt}
					<div class="detail-item">
						<span class="detail-label">Bandwidth Resets:</span>
						<span class="detail-value">
							{new Date(storageQuota.resetBandwidthAt).toLocaleDateString()}
						</span>
					</div>
				{/if}
				{#if storageQuota && storageQuota.storageUsed > storageQuota.maxStorageBytes * 0.9}
					<div class="detail-item warning">
						<span class="detail-label">Storage Warning:</span>
						<span class="detail-value">Over 90% used</span>
					</div>
				{/if}
			</div>
		</div>
	{/if}

	<!-- Storage Info -->
	<div class="storage-tips">
		<h4>Storage Information</h4>
		<ul>
			<li>Your storage quota is managed by your administrator</li>
			<li>Contact your admin if you need more storage space</li>
			{#if storageQuota && storageQuota.storageUsed > storageQuota.maxStorageBytes * 0.75}
				<li class="warning">Your storage is almost full - please contact your administrator</li>
			{/if}
		</ul>
	</div>

	<!-- Recent Activity -->
	{#if recentActivity && recentActivity.length > 0}
		<div class="recent-activity">
			<h4>Recent Activity</h4>
			<div class="activity-list">
				{#each recentActivity.slice(0, 5) as activity}
					<div class="activity-item">
						<div class="activity-icon {getActionColor(activity.action)}">
							<svelte:component this={getActionIcon(activity.action)} size={14} />
						</div>
						<div class="activity-details">
							<span class="activity-action">{activity.action}</span>
							<span class="activity-time">{new Date(activity.createdAt).toLocaleString()}</span>
						</div>
					</div>
				{/each}
			</div>
		</div>
	{/if}

	<svelte:fragment slot="footer">
		{#if isAdmin}
			<a href="/admin/extensions/cloudstorage" class="btn btn-secondary">
				Extension Settings
			</a>
			<a href="/admin/storage" class="btn btn-secondary">
				Manage Files
			</a>
		{/if}
		<Button on:click={handleClose}>Close</Button>
	</svelte:fragment>
</Modal>

<style>
	.storage-overview {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
		margin-bottom: 1.5rem;
	}

	.storage-stat-card {
		display: flex;
		gap: 1rem;
		padding: 1rem;
		background: #f9fafb;
		border-radius: 0.5rem;
		border: 1px solid #e5e7eb;
	}

	.stat-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 40px;
		height: 40px;
		border-radius: 0.5rem;
		flex-shrink: 0;
	}

	.storage-icon {
		background: #dbeafe;
		color: #2563eb;
	}

	.bandwidth-icon {
		background: #d1fae5;
		color: #059669;
	}

	.stat-details {
		flex: 1;
		min-width: 0;
	}

	.stat-label {
		display: block;
		font-size: 0.75rem;
		font-weight: 500;
		color: #6b7280;
		text-transform: uppercase;
		margin-bottom: 0.25rem;
	}

	.stat-value {
		display: block;
		font-size: 1.25rem;
		font-weight: 600;
		color: #111827;
		margin-bottom: 0.5rem;
	}

	.progress-bar {
		height: 6px;
		background: #e5e7eb;
		border-radius: 3px;
		overflow: hidden;
		margin-bottom: 0.25rem;
	}

	.progress-fill {
		height: 100%;
		background: #2563eb;
		border-radius: 3px;
		transition: width 0.3s ease;
	}

	.progress-fill.bandwidth {
		background: #059669;
	}

	.stat-detail {
		font-size: 0.75rem;
		color: #6b7280;
	}

	.storage-details, .storage-tips, .recent-activity {
		margin-bottom: 1.5rem;
	}

	.storage-details h4, .storage-tips h4, .recent-activity h4 {
		font-size: 0.875rem;
		font-weight: 600;
		color: #374151;
		margin: 0 0 0.75rem 0;
	}

	.detail-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.5rem;
	}

	.detail-item {
		display: flex;
		justify-content: space-between;
		padding: 0.5rem 0.75rem;
		background: #f9fafb;
		border-radius: 0.375rem;
		font-size: 0.875rem;
	}

	.detail-item.warning {
		background: #fef3c7;
		color: #92400e;
	}

	.detail-label {
		color: #6b7280;
	}

	.detail-value {
		font-weight: 500;
		color: #111827;
	}

	.storage-tips ul {
		margin: 0;
		padding-left: 1.25rem;
	}

	.storage-tips li {
		font-size: 0.875rem;
		color: #6b7280;
		margin-bottom: 0.5rem;
	}

	.storage-tips li.warning {
		color: #d97706;
		font-weight: 500;
	}

	.activity-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.activity-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.5rem;
		background: #f9fafb;
		border-radius: 0.375rem;
	}

	.activity-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border-radius: 0.25rem;
		background: #e5e7eb;
		color: #6b7280;
	}

	.activity-icon.upload {
		background: #dbeafe;
		color: #2563eb;
	}

	.activity-icon.download {
		background: #d1fae5;
		color: #059669;
	}

	.activity-icon.share {
		background: #fef3c7;
		color: #d97706;
	}

	.activity-details {
		display: flex;
		flex-direction: column;
	}

	.activity-action {
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
		text-transform: capitalize;
	}

	.activity-time {
		font-size: 0.75rem;
		color: #6b7280;
	}

	.btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		font-size: 0.875rem;
		font-weight: 500;
		border-radius: 0.375rem;
		text-decoration: none;
		transition: all 0.15s;
	}

	.btn-secondary {
		background: white;
		color: #374151;
		border: 1px solid #d1d5db;
	}

	.btn-secondary:hover {
		background: #f9fafb;
	}

	@media (max-width: 640px) {
		.storage-overview {
			grid-template-columns: 1fr;
		}

		.detail-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
