<script lang="ts">
	import { Settings } from 'lucide-svelte';
	import { createEventDispatcher } from 'svelte';
	import Modal from '$lib/components/ui/Modal.svelte';

	export let show = false;
	export let mode: 'create' | 'edit' = 'create';
	export let template = {
		name: '',
		displayName: '',
		description: '',
		category: 'standard',
		priceFormula: '',
		conditionFormula: '',
		isActive: true
	};
	export let variables: any[] = [];

	const dispatch = createEventDispatcher();

	let showFormulaModal = false;
	let editingFormulaField: 'price' | 'condition' = 'price';
	let tempFormula = '';

	const templateCategories = [
		{ value: 'time-based', label: 'Time-Based' },
		{ value: 'distance-based', label: 'Distance-Based' },
		{ value: 'usage-based', label: 'Usage-Based' },
		{ value: 'flat-rate', label: 'Flat Rate' },
		{ value: 'tiered', label: 'Tiered' },
		{ value: 'standard', label: 'Standard' }
	];

	function handleClose() {
		show = false;
		dispatch('close');
	}

	function handleSave() {
		dispatch('save', { template });
	}

	function openFormulaEditor(type: 'price' | 'condition') {
		editingFormulaField = type;
		const currentValue = type === 'price' ? template.priceFormula : template.conditionFormula;
		tempFormula = currentValue || '';
		showFormulaModal = true;
	}

	function saveFormula() {
		if (editingFormulaField === 'price') {
			template.priceFormula = tempFormula;
		} else {
			template.conditionFormula = tempFormula;
		}
		template = template;
		showFormulaModal = false;
	}

	function insertVariable(varName: string) {
		tempFormula = tempFormula ? `${tempFormula} ${varName}` : varName;
	}
</script>

<Modal {show} title="{mode === 'create' ? 'Create' : 'Edit'} Pricing Template" maxWidth="600px" on:close={handleClose}>
	<div class="form-row">
		<div class="modal-form-group">
			<label for="name">Template Name</label>
			<input
				type="text"
				id="name"
				bind:value={template.name}
				placeholder="e.g., time_based_pricing"
			/>
		</div>
		<div class="modal-form-group">
			<label for="displayName">Display Name</label>
			<input
				type="text"
				id="displayName"
				bind:value={template.displayName}
				placeholder="e.g., Time-Based Pricing"
			/>
		</div>
	</div>

	<div class="modal-form-group">
		<label for="description">Description</label>
		<textarea
			id="description"
			bind:value={template.description}
			rows="2"
			placeholder="Describe when and how this pricing template is used..."
		></textarea>
	</div>

	<div class="modal-form-group">
		<label for="category">Category</label>
		<select id="category" bind:value={template.category}>
			{#each templateCategories as cat}
				<option value={cat.value}>{cat.label}</option>
			{/each}
		</select>
	</div>

	<div class="modal-form-group">
		<label>Pricing Formula</label>
		<div class="formula-display">
			{#if template.priceFormula}
				<code class="formula-code">{template.priceFormula}</code>
			{:else}
				<span class="formula-placeholder">No formula defined</span>
			{/if}
			<button
				type="button"
				class="btn-edit-formula"
				on:click={() => openFormulaEditor('price')}
			>
				<Settings size={16} />
				Edit Formula
			</button>
		</div>
	</div>

	<div class="modal-form-group">
		<label>Condition Formula (Optional)</label>
		<div class="formula-display">
			{#if template.conditionFormula}
				<code class="formula-code condition">{template.conditionFormula}</code>
			{:else}
				<span class="formula-placeholder">No condition defined</span>
			{/if}
			<button
				type="button"
				class="btn-edit-formula"
				on:click={() => openFormulaEditor('condition')}
			>
				<Settings size={16} />
				Edit Condition
			</button>
		</div>
	</div>

	<div class="modal-form-group">
		<label for="isActive">Status</label>
		<select id="isActive" bind:value={template.isActive}>
			<option value={true}>Active</option>
			<option value={false}>Inactive</option>
		</select>
	</div>

	<svelte:fragment slot="footer">
		<button class="modal-btn modal-btn-secondary" on:click={handleClose}>Cancel</button>
		<button class="modal-btn modal-btn-primary" on:click={handleSave}>
			{mode === 'create' ? 'Create' : 'Save'} Template
		</button>
	</svelte:fragment>
</Modal>

<!-- Formula Editor Modal -->
<Modal show={showFormulaModal} title="Edit {editingFormulaField === 'price' ? 'Pricing' : 'Condition'} Formula" maxWidth="600px" on:close={() => showFormulaModal = false}>
	<div class="formula-editor-content">
		<div class="formula-input-section">
			<label for="formula-input">Formula</label>
			<textarea
				id="formula-input"
				bind:value={tempFormula}
				placeholder={editingFormulaField === 'condition'
					? "Enter a condition that evaluates to true/false (e.g., quantity >= 10)"
					: "Enter your pricing formula (e.g., base_price * quantity * (1 - discount_rate / 100))"}
				rows="6"
				class="formula-textarea"
			/>
		</div>

		{#if variables && variables.length > 0}
			<div class="variables-section">
				<h4>Available Variables</h4>
				<div class="variables-list">
					{#each variables as variable}
						<button class="variable-item" on:click={() => insertVariable(variable.name)}>
							<span class="variable-name">{variable.name}</span>
							{#if variable.description}
								<span class="variable-description">{variable.description}</span>
							{/if}
						</button>
					{/each}
				</div>
			</div>
		{/if}
	</div>

	<svelte:fragment slot="footer">
		<button class="modal-btn modal-btn-secondary" on:click={() => showFormulaModal = false}>Cancel</button>
		<button class="modal-btn modal-btn-primary" on:click={saveFormula}>Save Formula</button>
	</svelte:fragment>
</Modal>

<style>
	.form-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
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

	/* Formula Editor Styles */
	.formula-editor-content {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.formula-input-section label {
		display: block;
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
		margin-bottom: 0.375rem;
	}

	.formula-textarea {
		width: 100%;
		padding: 0.75rem;
		border: 1px solid #d1d5db;
		border-radius: 0.375rem;
		font-family: 'Courier New', monospace;
		font-size: 0.875rem;
		resize: vertical;
		min-height: 120px;
	}

	.formula-textarea:focus {
		outline: none;
		border-color: #189AB4;
		box-shadow: 0 0 0 3px rgba(24, 154, 180, 0.1);
	}

	.variables-section {
		margin-top: 1rem;
	}

	.variables-section h4 {
		font-size: 0.875rem;
		font-weight: 600;
		color: #374151;
		margin: 0 0 0.5rem 0;
	}

	.variables-list {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.variable-item {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.25rem 0.5rem;
		background: #f3f4f6;
		border: 1px solid #d1d5db;
		border-radius: 0.25rem;
		cursor: pointer;
		transition: all 0.2s;
		font-size: 0.75rem;
	}

	.variable-item:hover {
		background: #e5e7eb;
		border-color: #9ca3af;
	}

	.variable-name {
		font-family: 'Courier New', monospace;
		color: #1f2937;
		font-weight: 500;
	}

	.variable-description {
		color: #6b7280;
		font-size: 0.75rem;
	}
</style>
