<script lang="ts">
	import { HardDrive } from 'lucide-svelte';

	export let totalStorage = '0 B';
	export let usedStorage = '0 B';
	export let totalFiles = 0;
	export let totalBuckets = 0;

	$: storagePercentage = calculateStoragePercentage(usedStorage, totalStorage);

	function calculateStoragePercentage(used: string, total: string): number {
		// Parse storage values
		const parseSize = (str: string) => {
			const match = str.match(/([\d.]+)\s*([KMGT]?B)/i);
			if (!match) return 0;

			const value = parseFloat(match[1]);
			const unit = match[2].toUpperCase();

			const units: Record<string, number> = {
				'B': 1,
				'KB': 1024,
				'MB': 1024 * 1024,
				'GB': 1024 * 1024 * 1024,
				'TB': 1024 * 1024 * 1024 * 1024
			};

			return value * (units[unit] || 1);
		};

		const usedBytes = parseSize(used);
		const totalBytes = parseSize(total);

		return totalBytes > 0 ? (usedBytes / totalBytes) * 100 : 0;
	}
</script>

<div class="stats-container">
	<div class="storage-header">
		<HardDrive size={24} />
		<div>
			<h1>Storage Management</h1>
			<p class="storage-info">{usedStorage} / {totalStorage} used</p>
		</div>
	</div>

	<div class="storage-progress">
		<div class="progress-bar">
			<div class="progress-fill" style="width: {storagePercentage}%"></div>
		</div>
		<div class="storage-stats">
			<span>{totalFiles} files</span>
			<span>{totalBuckets} buckets</span>
		</div>
	</div>
</div>

<style>
	.stats-container {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		padding: 1.5rem;
		margin-bottom: 1.5rem;
	}

	.storage-header {
		display: flex;
		align-items: flex-start;
		gap: 1rem;
		margin-bottom: 1rem;
	}

	.storage-header h1 {
		margin: 0;
		font-size: 1.5rem;
		font-weight: 600;
	}

	.storage-info {
		margin: 0.25rem 0 0 0;
		color: #6b7280;
		font-size: 0.875rem;
	}

	.storage-progress {
		margin-top: 1rem;
	}

	.progress-bar {
		width: 100%;
		height: 8px;
		background: #e5e7eb;
		border-radius: 4px;
		overflow: hidden;
	}

	.progress-fill {
		height: 100%;
		background: #189AB4;
		transition: width 0.3s ease;
	}

	.storage-stats {
		display: flex;
		justify-content: space-between;
		margin-top: 0.5rem;
		font-size: 0.875rem;
		color: #6b7280;
	}
</style>