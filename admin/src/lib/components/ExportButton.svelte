<script lang="ts">
	import { Download, FileJson, FileSpreadsheet } from 'lucide-svelte';
	import { exportToCSV, exportToJSON, flattenObjectsForCSV } from '$lib/export';
	
	export let data: any[] = [];
	export let filename: string = 'export';
	export let flatten: boolean = false;
	export let disabled: boolean = false;
	
	let showDropdown = false;
	
	function handleExportCSV() {
		const timestamp = new Date().toISOString().split('T')[0];
		const dataToExport = flatten ? flattenObjectsForCSV(data) : data;
		exportToCSV(dataToExport, `${filename}_${timestamp}.csv`);
		showDropdown = false;
	}
	
	function handleExportJSON() {
		const timestamp = new Date().toISOString().split('T')[0];
		exportToJSON(data, `${filename}_${timestamp}.json`);
		showDropdown = false;
	}
	
	function toggleDropdown() {
		showDropdown = !showDropdown;
	}
	
	// Close dropdown when clicking outside
	function handleClickOutside(event: MouseEvent) {
		const target = event.target as HTMLElement;
		if (!target.closest('.export-dropdown')) {
			showDropdown = false;
		}
	}
</script>

<svelte:window on:click={handleClickOutside} />

<div class="export-dropdown">
	<button 
		class="export-btn"
		on:click|stopPropagation={toggleDropdown}
		{disabled}
		title="Export data"
	>
		<Download size={16} />
		Export
	</button>
	
	{#if showDropdown && !disabled}
		<div class="dropdown-menu">
			<button 
				class="dropdown-item"
				on:click|stopPropagation={handleExportCSV}
			>
				<FileSpreadsheet size={16} />
				Export as CSV
			</button>
			<button 
				class="dropdown-item"
				on:click|stopPropagation={handleExportJSON}
			>
				<FileJson size={16} />
				Export as JSON
			</button>
		</div>
	{/if}
</div>

<style>
	.export-dropdown {
		position: relative;
		display: inline-block;
	}
	
	.export-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		color: #374151;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		line-height: 1;
	}
	
	.export-btn:hover:not(:disabled) {
		background: #f9fafb;
		border-color: #d1d5db;
	}
	
	.export-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	
	.dropdown-menu {
		position: absolute;
		top: calc(100% + 4px);
		right: 0;
		min-width: 180px;
		background: white;
		border: 1px solid var(--border-color);
		border-radius: 6px;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
		z-index: 1000;
		overflow: hidden;
	}
	
	.dropdown-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		width: 100%;
		padding: 0.75rem 1rem;
		background: none;
		border: none;
		color: var(--text-primary);
		font-size: 0.875rem;
		text-align: left;
		cursor: pointer;
		transition: background 0.2s;
	}
	
	.dropdown-item:hover {
		background: var(--bg-hover);
	}
	
	.dropdown-item:not(:last-child) {
		border-bottom: 1px solid var(--border-color-light, #f0f0f0);
	}
</style>