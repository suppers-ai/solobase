<script lang="ts">
	import { Settings } from 'lucide-svelte';
	import { createEventDispatcher } from 'svelte';
	
	export let template = {
		name: '',
		display_name: '',
		description: '',
		category: 'standard',
		price_formula: '',
		condition_formula: '',
		is_active: true
	};
	
	export let templateCategories = [
		{ value: 'time-based', label: 'Time-Based' },
		{ value: 'distance-based', label: 'Distance-Based' },
		{ value: 'usage-based', label: 'Usage-Based' },
		{ value: 'flat-rate', label: 'Flat Rate' },
		{ value: 'tiered', label: 'Tiered' },
		{ value: 'standard', label: 'Standard' }
	];
	
	const dispatch = createEventDispatcher();
	
	function editFormula(type: 'price' | 'condition') {
		dispatch('editFormula', {
			type,
			currentValue: type === 'price' ? template.price_formula : template.condition_formula
		});
	}
</script>

<div class="pricing-template-form">
	<div class="form-row">
		<div class="form-group">
			<label for="name">Template Name</label>
			<input 
				type="text" 
				id="name" 
				bind:value={template.name} 
				placeholder="e.g., time_based_pricing" 
			/>
		</div>
		<div class="form-group">
			<label for="display_name">Display Name</label>
			<input 
				type="text" 
				id="display_name" 
				bind:value={template.display_name} 
				placeholder="e.g., Time-Based Pricing" 
			/>
		</div>
	</div>
	
	<div class="form-group">
		<label for="description">Description</label>
		<textarea 
			id="description" 
			bind:value={template.description} 
			rows="2"
			placeholder="Describe when and how this pricing template is used..."
		></textarea>
	</div>
	
	<div class="form-group">
		<label for="category">Category</label>
		<select id="category" bind:value={template.category}>
			{#each templateCategories as cat}
				<option value={cat.value}>{cat.label}</option>
			{/each}
		</select>
	</div>
	
	<div class="form-group">
		<label>Pricing Formula</label>
		<div class="formula-display">
			{#if template.price_formula}
				<code class="formula-code">{template.price_formula}</code>
			{:else}
				<span class="formula-placeholder">No formula defined</span>
			{/if}
			<button 
				type="button" 
				class="btn-edit-formula" 
				on:click={() => editFormula('price')}
			>
				<Settings size={16} />
				Edit Formula
			</button>
		</div>
	</div>
	
	<div class="form-group">
		<label>Condition Formula (Optional)</label>
		<div class="formula-display">
			{#if template.condition_formula}
				<code class="formula-code condition">{template.condition_formula}</code>
			{:else}
				<span class="formula-placeholder">No condition defined</span>
			{/if}
			<button 
				type="button" 
				class="btn-edit-formula" 
				on:click={() => editFormula('condition')}
			>
				<Settings size={16} />
				Edit Condition
			</button>
		</div>
	</div>
	
	<div class="form-group">
		<label for="is_active">Status</label>
		<select id="is_active" bind:value={template.is_active}>
			<option value={true}>Active</option>
			<option value={false}>Inactive</option>
		</select>
	</div>
</div>

<style>
	.pricing-template-form {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	
	.form-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
	}
	
	.form-group {
		display: flex;
		flex-direction: column;
	}
	
	.form-group label {
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
		margin-bottom: 0.375rem;
	}
	
	.form-group input[type="text"],
	.form-group select,
	.form-group textarea {
		padding: 0.5rem 0.75rem;
		border: 1px solid #d1d5db;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		transition: all 0.2s;
	}
	
	.form-group input[type="text"]:focus,
	.form-group select:focus,
	.form-group textarea:focus {
		outline: none;
		border-color: #3b82f6;
		box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
	}
	
	.form-group textarea {
		resize: vertical;
		min-height: 60px;
	}
	
	.formula-display {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0.75rem;
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
	}
	
	.formula-code {
		font-family: 'Courier New', monospace;
		font-size: 0.875rem;
		color: #1f2937;
		background: white;
		padding: 0.5rem;
		border-radius: 0.25rem;
		border: 1px solid #d1d5db;
		word-break: break-all;
	}
	
	.formula-code.condition {
		color: #059669;
	}
	
	.formula-placeholder {
		color: #9ca3af;
		font-style: italic;
		font-size: 0.875rem;
		padding: 0.5rem;
	}
	
	.btn-edit-formula {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		background: white;
		border: 1px solid #d1d5db;
		border-radius: 0.375rem;
		color: #4b5563;
		font-size: 0.875rem;
		cursor: pointer;
		transition: all 0.2s;
		align-self: flex-start;
	}
	
	.btn-edit-formula:hover {
		background: #f3f4f6;
		border-color: #9ca3af;
		color: #1f2937;
	}
</style>