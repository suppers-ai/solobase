<script lang="ts">
	import { FolderPlus, Folder } from 'lucide-svelte';
	import { createEventDispatcher } from 'svelte';

	export let buckets: any[] = [];
	export let selectedBucket: any = null;
	export let showCreateButton = true;

	const dispatch = createEventDispatcher();

	function handleSelectBucket(bucket: any) {
		dispatch('select', bucket);
	}

	function handleCreateBucket() {
		dispatch('create');
	}
</script>

<div class="bucket-selector">
	<div class="bucket-header">
		<h3>Buckets</h3>
		{#if showCreateButton}
			<button class="btn btn-sm btn-secondary" on:click={handleCreateBucket}>
				<FolderPlus size={16} />
				New Bucket
			</button>
		{/if}
	</div>

	<div class="bucket-list">
		{#each buckets as bucket}
			<button
				class="bucket-item"
				class:active={selectedBucket?.name === bucket.name}
				on:click={() => handleSelectBucket(bucket)}
			>
				<Folder size={18} />
				<span class="bucket-name">{bucket.name}</span>
				{#if bucket.public}
					<span class="bucket-badge public">Public</span>
				{:else}
					<span class="bucket-badge private">Private</span>
				{/if}
			</button>
		{/each}

		{#if buckets.length === 0}
			<div class="no-buckets">
				<p>No buckets yet</p>
				<button class="btn btn-primary" on:click={handleCreateBucket}>
					Create your first bucket
				</button>
			</div>
		{/if}
	</div>
</div>

<style>
	.bucket-selector {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		padding: 1rem;
		height: fit-content;
		min-width: 250px;
	}

	.bucket-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
		padding-bottom: 0.75rem;
		border-bottom: 1px solid #e5e7eb;
	}

	.bucket-header h3 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
	}

	.bucket-list {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.bucket-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.75rem;
		background: transparent;
		border: 1px solid transparent;
		border-radius: 0.375rem;
		cursor: pointer;
		text-align: left;
		transition: all 0.2s;
	}

	.bucket-item:hover {
		background: #f3f4f6;
	}

	.bucket-item.active {
		background: #e0f2fe;
		border-color: #189AB4;
	}

	.bucket-name {
		flex: 1;
		font-size: 0.875rem;
		font-weight: 500;
	}

	.bucket-badge {
		font-size: 0.75rem;
		padding: 0.125rem 0.375rem;
		border-radius: 0.25rem;
	}

	.bucket-badge.public {
		background: #dbeafe;
		color: #1e40af;
	}

	.bucket-badge.private {
		background: #f3f4f6;
		color: #6b7280;
	}

	.no-buckets {
		padding: 2rem 1rem;
		text-align: center;
	}

	.no-buckets p {
		margin: 0 0 1rem 0;
		color: #6b7280;
		font-size: 0.875rem;
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

	.btn-secondary {
		background: #f3f4f6;
		color: #374151;
	}

	.btn-secondary:hover {
		background: #e5e7eb;
	}

	.btn-sm {
		padding: 0.375rem 0.75rem;
		font-size: 0.75rem;
	}
</style>