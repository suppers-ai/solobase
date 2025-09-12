<script lang="ts">
	import { Plus, Trash2 } from 'lucide-svelte';
	
	export let fields: any[] = [];
	export let onFieldsChange: (fields: any[]) => void;
	
	let showTypeSelector = false;
	
	// Track used filter IDs
	function getUsedFilterCount(type: string): number {
		return fields.filter(f => {
			if (f.id) {
				const parts = f.id.split('_');
				return parts[0] === 'filter' && parts[1] === type;
			}
			return false;
		}).length;
	}
	
	// Get next available filter ID for a type
	function getNextFilterId(type: string): string | null {
		const count = getUsedFilterCount(type);
		if (count >= 5) return null;
		
		// Find the next available number
		for (let i = 1; i <= 5; i++) {
			const id = `filter_${type}_${i}`;
			if (!fields.some(f => f.id === id)) {
				return id;
			}
		}
		return null;
	}
	
	// Check if a field type is available
	function isTypeAvailable(type: string): boolean {
		return getNextFilterId(type) !== null;
	}
	
	function addField(type: string) {
		const id = getNextFilterId(type);
		if (!id) {
			alert(`Maximum of 5 ${type} fields reached`);
			return;
		}
		
		const newField = {
			id: id,
			name: '',
			type: type,
			required: false,
			description: '',
			constraints: {}
		};
		
		fields = [...fields, newField];
		onFieldsChange(fields);
		showTypeSelector = false;
	}
	
	function removeField(index: number) {
		fields = fields.filter((_, i) => i !== index);
		onFieldsChange(fields);
	}
	
	function updateField(index: number, field: any) {
		fields[index] = field;
		onFieldsChange(fields);
	}
</script>

<div class="field-editor">
	{#each fields as field, index}
		<div class="field-card">
			<div class="field-header">
				<div class="field-header-left">
					<h4>Field {index + 1}</h4>
					<span class="field-type-badge">{field.type}</span>
					<span class="field-id-badge">{field.id}</span>
				</div>
				<button class="btn-remove" on:click={() => removeField(index)} title="Remove field">
					<Trash2 size={14} />
				</button>
			</div>
			
			<div class="field-grid">
				<div class="field-col">
					<label>Display Name</label>
					<input 
						type="text" 
						placeholder="e.g., Company Size" 
						value={field.name}
						on:input={(e) => updateField(index, {...field, name: e.target.value})}
					/>
				</div>
				
				<div class="field-col">
					<label>Description</label>
					<input 
						type="text" 
						placeholder="Help text for this field" 
						value={field.description}
						on:input={(e) => updateField(index, {...field, description: e.target.value})}
					/>
				</div>
				
				<div class="field-col">
					<label class="checkbox-label">
						<input 
							type="checkbox" 
							checked={field.required}
							on:change={(e) => updateField(index, {...field, required: e.target.checked})}
						/>
						Required field
					</label>
				</div>
				
				<!-- Type-specific constraints -->
				{#if field.type === 'numeric'}
					<div class="field-col">
						<label>Min Value</label>
						<input 
							type="number" 
							value={field.constraints.min || ''}
							on:input={(e) => updateField(index, {...field, constraints: {...field.constraints, min: e.target.value ? parseFloat(e.target.value) : undefined}})}
						/>
					</div>
					<div class="field-col">
						<label>Max Value</label>
						<input 
							type="number" 
							value={field.constraints.max || ''}
							on:input={(e) => updateField(index, {...field, constraints: {...field.constraints, max: e.target.value ? parseFloat(e.target.value) : undefined}})}
						/>
					</div>
				{:else if field.type === 'text'}
					<div class="field-col">
						<label>Min Length</label>
						<input 
							type="number" 
							min="0"
							value={field.constraints.min_length || ''}
							on:input={(e) => updateField(index, {...field, constraints: {...field.constraints, min_length: e.target.value ? parseInt(e.target.value) : undefined}})}
						/>
					</div>
					<div class="field-col">
						<label>Max Length</label>
						<input 
							type="number" 
							min="0"
							value={field.constraints.max_length || ''}
							on:input={(e) => updateField(index, {...field, constraints: {...field.constraints, max_length: e.target.value ? parseInt(e.target.value) : undefined}})}
						/>
					</div>
				{:else if field.type === 'enum'}
					<div class="field-col full-width">
						<label>Options (comma-separated)</label>
						<input 
							type="text" 
							placeholder="e.g., Small, Medium, Large"
							value={field.constraints.options?.join(', ') || ''}
							on:input={(e) => updateField(index, {...field, constraints: {...field.constraints, options: e.target.value.split(',').map(s => s.trim()).filter(s => s)}})}
						/>
					</div>
				{:else if field.type === 'location'}
					<div class="field-col full-width">
						<label>Location Format</label>
						<select 
							value={field.constraints.format || 'address'}
							on:change={(e) => updateField(index, {...field, constraints: {...field.constraints, format: e.target.value}})}>
							<option value="address">Street Address</option>
							<option value="coordinates">Lat/Long Coordinates</option>
							<option value="city">City Only</option>
							<option value="region">Region/State</option>
						</select>
					</div>
				{/if}
			</div>
		</div>
	{/each}
	
	{#if !showTypeSelector}
		<button class="btn-add-field" on:click={() => showTypeSelector = true}>
			<Plus size={14} />
			Add Field
		</button>
	{:else}
		<div class="type-selector">
			<h4>Select Field Type</h4>
			<div class="type-options">
				<button 
					class="type-option" 
					class:disabled={!isTypeAvailable('text')}
					on:click={() => addField('text')}
					disabled={!isTypeAvailable('text')}>
					<span class="type-name">Text</span>
					<span class="type-count">{getUsedFilterCount('text')}/5</span>
				</button>
				<button 
					class="type-option" 
					class:disabled={!isTypeAvailable('numeric')}
					on:click={() => addField('numeric')}
					disabled={!isTypeAvailable('numeric')}>
					<span class="type-name">Numeric</span>
					<span class="type-count">{getUsedFilterCount('numeric')}/5</span>
				</button>
				<button 
					class="type-option" 
					class:disabled={!isTypeAvailable('boolean')}
					on:click={() => addField('boolean')}
					disabled={!isTypeAvailable('boolean')}>
					<span class="type-name">Boolean</span>
					<span class="type-count">{getUsedFilterCount('boolean')}/5</span>
				</button>
				<button 
					class="type-option" 
					class:disabled={!isTypeAvailable('enum')}
					on:click={() => addField('enum')}
					disabled={!isTypeAvailable('enum')}>
					<span class="type-name">Enum/Select</span>
					<span class="type-count">{getUsedFilterCount('enum')}/5</span>
				</button>
				<button 
					class="type-option" 
					class:disabled={!isTypeAvailable('location')}
					on:click={() => addField('location')}
					disabled={!isTypeAvailable('location')}>
					<span class="type-name">Location</span>
					<span class="type-count">{getUsedFilterCount('location')}/5</span>
				</button>
			</div>
			<button class="btn-cancel" on:click={() => showTypeSelector = false}>Cancel</button>
		</div>
	{/if}
</div>

<style>
	.field-editor {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	
	.field-card {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		padding: 1rem;
	}
	
	.field-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
		padding-bottom: 0.75rem;
		border-bottom: 1px solid #f3f4f6;
	}
	
	.field-header-left {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	
	.field-header h4 {
		margin: 0;
		font-size: 0.875rem;
		font-weight: 600;
		color: #374151;
	}
	
	.field-type-badge {
		padding: 0.125rem 0.5rem;
		background: #dbeafe;
		color: #1e40af;
		border-radius: 0.25rem;
		font-size: 0.75rem;
		font-weight: 500;
	}
	
	.field-id-badge {
		padding: 0.125rem 0.5rem;
		background: #f3f4f6;
		color: #6b7280;
		border-radius: 0.25rem;
		font-size: 0.75rem;
		font-family: monospace;
	}
	
	.btn-remove {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border: 1px solid #fee2e2;
		border-radius: 0.375rem;
		background: white;
		color: #ef4444;
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.btn-remove:hover {
		background: #fee2e2;
		border-color: #fca5a5;
	}
	
	.field-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: 1rem;
	}
	
	.field-col {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	
	.field-col.full-width {
		grid-column: 1 / -1;
	}
	
	.field-col label {
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
	}
	
	.field-col input,
	.field-col select {
		padding: 0.5rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		font-size: 0.875rem;
	}
	
	.field-col input:focus,
	.field-col select:focus {
		outline: none;
		border-color: #06b6d4;
		box-shadow: 0 0 0 3px rgba(6, 182, 212, 0.1);
	}
	
	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 0;
		cursor: pointer;
	}
	
	.checkbox-label input[type="checkbox"] {
		width: 16px;
		height: 16px;
		cursor: pointer;
	}
	
	.btn-add-field {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		padding: 0.75rem;
		border: 2px dashed #e5e7eb;
		border-radius: 0.5rem;
		background: white;
		color: #6b7280;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.btn-add-field:hover {
		border-color: #06b6d4;
		color: #06b6d4;
		background: #f0fdfa;
	}
	
	.type-selector {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		padding: 1.5rem;
	}
	
	.type-selector h4 {
		margin: 0 0 1rem 0;
		font-size: 1rem;
		font-weight: 600;
		color: #111827;
	}
	
	.type-options {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
		gap: 0.75rem;
		margin-bottom: 1rem;
	}
	
	.type-option {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.25rem;
		padding: 1rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		background: white;
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.type-option:hover:not(.disabled) {
		border-color: #06b6d4;
		background: #f0fdfa;
	}
	
	.type-option.disabled {
		opacity: 0.5;
		cursor: not-allowed;
		background: #f9fafb;
	}
	
	.type-name {
		font-size: 0.875rem;
		font-weight: 500;
		color: #111827;
	}
	
	.type-count {
		font-size: 0.75rem;
		color: #6b7280;
	}
	
	.btn-cancel {
		padding: 0.5rem 1rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		background: white;
		color: #374151;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.btn-cancel:hover {
		background: #f9fafb;
	}
</style>