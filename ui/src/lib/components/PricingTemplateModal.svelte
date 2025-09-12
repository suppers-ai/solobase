<script lang="ts">
	import { Settings } from 'lucide-svelte';
	import { createEventDispatcher } from 'svelte';
	
	export let show = false;
	export let mode: 'create' | 'edit' = 'create';
	export let template = {
		name: '',
		display_name: '',
		description: '',
		category: 'standard',
		price_formula: '',
		condition_formula: '',
		is_active: true
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
		// Get the current value from the template
		const currentValue = type === 'price' ? template.price_formula : template.condition_formula;
		tempFormula = currentValue || '';
		showFormulaModal = true;
	}
	
	function saveFormula() {
		if (editingFormulaField === 'price') {
			template.price_formula = tempFormula;
		} else {
			template.condition_formula = tempFormula;
		}
		// Trigger reactivity
		template = template;
		showFormulaModal = false;
	}
	
	function handleCreateVariable() {
		dispatch('createVariable');
	}
	
	function insertVariable(varName: string) {
		// Insert variable at cursor position or at end
		tempFormula = tempFormula ? `${tempFormula} ${varName}` : varName;
	}
</script>

{#if show}
	<div class="modal-overlay" on:click={handleClose}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h2>{mode === 'create' ? 'Create' : 'Edit'} Pricing Template</h2>
				<button class="btn-icon" on:click={handleClose}>
					×
				</button>
			</div>
			<div class="modal-body">
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
							on:click={() => openFormulaEditor('price')}
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
							on:click={() => openFormulaEditor('condition')}
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
			<div class="modal-footer">
				<div class="modal-footer-left"></div>
				<div class="modal-footer-right">
					<button class="btn btn-secondary" on:click={handleClose}>Cancel</button>
					<button class="btn btn-primary" on:click={handleSave}>
						{mode === 'create' ? 'Create' : 'Save'} Template
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}

<!-- Formula Editor Modal -->
{#if showFormulaModal}
	<div class="modal-overlay formula-modal-overlay" on:click={() => showFormulaModal = false}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h2>Edit {editingFormulaField === 'price' ? 'Pricing' : 'Condition'} Formula</h2>
				<button class="btn-icon" on:click={() => showFormulaModal = false}>
					×
				</button>
			</div>
			<div class="modal-body">
				<div class="formula-editor-content">
					<div class="formula-input-section">
						<label>Formula</label>
						<textarea
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
									<div class="variable-item" on:click={() => insertVariable(variable.name)}>
										<span class="variable-name">{variable.name}</span>
										{#if variable.description}
											<span class="variable-description">{variable.description}</span>
										{/if}
									</div>
								{/each}
							</div>
						</div>
					{/if}
				</div>
			</div>
			<div class="modal-footer">
				<div class="modal-footer-left"></div>
				<div class="modal-footer-right">
					<button class="btn btn-secondary" on:click={() => showFormulaModal = false}>Cancel</button>
					<button class="btn btn-primary" on:click={saveFormula}>Save Formula</button>
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 9999;
	}
	
	.formula-modal-overlay {
		z-index: 10000 !important;
	}
	
	.modal {
		background: white;
		border-radius: 0.5rem;
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1);
		max-width: 600px;
		width: 90%;
		max-height: 90vh;
		display: flex;
		flex-direction: column;
	}
	
	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-bottom: 1px solid #e5e7eb;
	}
	
	.modal-header h2 {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 600;
		color: #111827;
	}
	
	.btn-icon {
		background: transparent;
		border: none;
		font-size: 1.5rem;
		color: #6b7280;
		cursor: pointer;
		padding: 0.25rem;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2rem;
		height: 2rem;
		border-radius: 0.25rem;
		transition: all 0.2s;
	}
	
	.btn-icon:hover {
		background: #f3f4f6;
		color: #111827;
	}
	
	.modal-body {
		padding: 1.5rem;
		overflow-y: auto;
		flex: 1;
	}
	
	.modal-footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-top: 1px solid #e5e7eb;
	}
	
	.modal-footer-left {
		display: flex;
		gap: 0.5rem;
	}
	
	.modal-footer-right {
		display: flex;
		gap: 0.5rem;
	}
	
	.form-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
		margin-bottom: 1rem;
	}
	
	.form-group {
		display: flex;
		flex-direction: column;
		margin-bottom: 1rem;
	}
	
	.form-group:last-child {
		margin-bottom: 0;
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
	
	.btn {
		padding: 0.5rem 1rem;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		border: none;
	}
	
	.btn-primary {
		background: #3b82f6;
		color: white;
	}
	
	.btn-primary:hover {
		background: #2563eb;
	}
	
	.btn-secondary {
		background: #6b7280;
		color: white;
	}
	
	.btn-secondary:hover {
		background: #4b5563;
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
		border-color: #3b82f6;
		box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
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