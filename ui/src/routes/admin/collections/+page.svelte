<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { Collection } from '$lib/types';
	import { Layers, Plus, Edit, Trash2 } from 'lucide-svelte';
	import { requireAdmin } from '$lib/utils/auth';
	
	let collections: Collection[] = [];
	let loading = true;
	
	onMount(async () => {
		// Check admin access
		if (!requireAdmin()) return;
		
		const response = await api.getCollections();
		if (response.data) {
			collections = response.data;
		}
		loading = false;
	});
</script>

<div class="space-y-6">
	<div class="flex justify-between items-center">
		<h1 class="h1">Collections</h1>
		<button class="btn variant-filled-primary">
			<Plus size={16} />
			<span>New Collection</span>
		</button>
	</div>
	
	{#if loading}
		<div class="placeholder animate-pulse h-64"></div>
	{:else if collections.length > 0}
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
			{#each collections as collection}
				<div class="card p-6">
					<div class="flex items-start justify-between mb-4">
						<div class="rounded-full bg-secondary-500/20 p-3">
							<Layers size={24} class="text-secondary-500" />
						</div>
						<div class="flex gap-2">
							<button class="btn btn-sm variant-ghost-surface">
								<Edit size={14} />
							</button>
							<button class="btn btn-sm variant-ghost-error">
								<Trash2 size={14} />
							</button>
						</div>
					</div>
					<h3 class="h4 mb-2">{collection.name}</h3>
					<p class="text-sm text-surface-500 mb-3">
						{collection.records_count} records
					</p>
					<p class="text-sm text-surface-500">
						{collection.schema.fields.length} fields
					</p>
				</div>
			{/each}
		</div>
	{:else}
		<div class="card p-12 text-center">
			<Layers size={48} class="mx-auto mb-4 text-surface-500 opacity-50" />
			<h3 class="h3 mb-2">No Collections Yet</h3>
			<p class="text-surface-500 mb-4">Create your first collection to get started</p>
			<button class="btn variant-filled-primary mx-auto">
				<Plus size={16} />
				<span>Create Collection</span>
			</button>
		</div>
	{/if}
</div>