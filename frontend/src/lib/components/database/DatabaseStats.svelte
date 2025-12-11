<script lang="ts">
	import { Database, Server, HardDrive } from 'lucide-svelte';

	export let dbType = 'SQLite';
	export let dbVersion = '';
	export let dbSize = '';
	export let tableCount = 0;
</script>

<div class="stats-container">
	<div class="stat-card">
		<div class="stat-icon">
			<Database size={24} />
		</div>
		<div class="stat-content">
			<h3>Database Type</h3>
			<p class="stat-value">{dbType}</p>
			{#if dbVersion}
				<p class="stat-meta">Version {dbVersion}</p>
			{/if}
		</div>
	</div>

	<div class="stat-card">
		<div class="stat-icon">
			<Server size={24} />
		</div>
		<div class="stat-content">
			<h3>Tables</h3>
			<p class="stat-value">{tableCount}</p>
			<p class="stat-meta">Database objects</p>
		</div>
	</div>

	<div class="stat-card">
		<div class="stat-icon">
			<HardDrive size={24} />
		</div>
		<div class="stat-content">
			<h3>Size</h3>
			<p class="stat-value">{dbSize || 'N/A'}</p>
			<p class="stat-meta">Storage used</p>
		</div>
	</div>
</div>

<style>
	.stats-container {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: 1rem;
		margin-bottom: 1.5rem;
	}

	.stat-card {
		display: flex;
		align-items: flex-start;
		gap: 1rem;
		padding: 1.5rem;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		transition: all 0.2s;
	}

	.stat-card:hover {
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
	}

	.stat-icon {
		padding: 0.75rem;
		background: #e0f2fe;
		border-radius: 0.5rem;
		color: #0369a1;
	}

	.stat-content {
		flex: 1;
	}

	.stat-content h3 {
		margin: 0 0 0.5rem 0;
		font-size: 0.875rem;
		font-weight: 500;
		color: #6b7280;
	}

	.stat-value {
		margin: 0 0 0.25rem 0;
		font-size: 1.5rem;
		font-weight: 600;
		color: #111827;
	}

	.stat-meta {
		margin: 0;
		font-size: 0.75rem;
		color: #9ca3af;
	}
</style>