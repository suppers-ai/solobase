<script lang="ts">
	import { Database, Server, HardDrive } from 'lucide-svelte';
	import StatCard from '$lib/components/ui/StatCard.svelte';

	export let dbType = 'SQLite';
	export let dbVersion = '';
	export let dbSize = '';
	export let tableCount = 0;
</script>

<div class="stats-container">
	<StatCard
		icon={Database}
		value={dbType}
		label="Database Type"
		sublabel={dbVersion ? `Version ${dbVersion}` : ''}
		iconBg="bg-blue-100"
		iconColor="text-blue-600"
		size="lg"
	/>
	<StatCard
		icon={Server}
		value={tableCount}
		label="Tables"
		sublabel="Database objects"
		iconBg="bg-blue-100"
		iconColor="text-blue-600"
		size="lg"
	/>
	<StatCard
		icon={HardDrive}
		value={dbSize || 'N/A'}
		label="Size"
		sublabel="Storage used"
		iconBg="bg-blue-100"
		iconColor="text-blue-600"
		size="lg"
	/>
</div>

<style>
	.stats-container {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: 1rem;
		margin-bottom: 1.5rem;
	}
</style>
