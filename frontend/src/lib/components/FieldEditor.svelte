<script lang="ts">
	import { Plus, Trash2, Edit2, ChevronUp, ChevronDown, GripVertical } from 'lucide-svelte';

	export let fields: any[] = [];
	export let onFieldsChange: (fields: any[]) => void;

	let showFieldModal = false;
	let showTypeSelector = false;
	let editingIndex: number | null = null;
	let selectedType: string = '';

	let fieldForm = {
		id: '',
		name: '',
		type: '',
		required: false,
		description: '',
		constraints: {
			editable_by_user: true
		}
	};

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

	function openAddFieldModal(type: string) {
		const id = getNextFilterId(type);
		if (!id) {
			alert(`Maximum of 5 ${type} fields reached`);
			return;
		}

		selectedType = type;
		fieldForm = {
			id: id,
			name: '',
			type: type,
			required: false,
			description: '',
			constraints: {
				editable_by_user: true
			}
		};

		editingIndex = null;
		showTypeSelector = false;
		showFieldModal = true;
	}

	function editField(index: number) {
		const field = fields[index];
		fieldForm = {
			...field,
			constraints: {
				editable_by_user: true,
				...(field.constraints || {})
			}
		};
		selectedType = field.type;
		editingIndex = index;
		showFieldModal = true;
	}

	function saveField() {
		if (!fieldForm.name) {
			alert('Please provide a field name');
			return;
		}

		if (editingIndex !== null) {
			// Update existing field
			fields[editingIndex] = { ...fieldForm };
		} else {
			// Add new field
			fields = [...fields, fieldForm];
		}

		onFieldsChange(fields);
		closeModal();
	}

	function removeField(index: number) {
		fields = fields.filter((_, i) => i !== index);
		onFieldsChange(fields);
	}

	function closeModal() {
		showFieldModal = false;
		editingIndex = null;
		fieldForm = {
			id: '',
			name: '',
			type: '',
			required: false,
			description: '',
			constraints: {
				editable_by_user: true
			}
		};
	}

	function moveField(index: number, direction: 'up' | 'down') {
		const newIndex = direction === 'up' ? index - 1 : index + 1;
		if (newIndex < 0 || newIndex >= fields.length) return;

		const newFields = [...fields];
		[newFields[index], newFields[newIndex]] = [newFields[newIndex], newFields[index]];
		fields = newFields;
		onFieldsChange(fields);
	}

	const fieldTypes = [
		{ value: 'text', label: 'Text', description: 'Short text input' },
		{ value: 'numeric', label: 'Numeric', description: 'Number values' },
		{ value: 'boolean', label: 'Boolean', description: 'Yes/No selection' },
		{ value: 'enum', label: 'Enum/Select', description: 'Dropdown options' },
		{ value: 'select', label: 'Select', description: 'Dropdown selection' },
		{ value: 'location', label: 'Location', description: 'Address or coordinates' }
	];
</script>

<div class="field-editor">
	{#if fields.length > 0}
		<div class="fields-list">
			{#each fields as field, index}
				<div class="field-card">
					<div class="field-handle">
						<GripVertical size={16} />
					</div>
					<div class="field-info">
						<div class="field-header">
							<h4>{field.name || 'Unnamed Field'}</h4>
							<span class="field-type-badge">{field.type}</span>
							<span class="field-id-badge">{field.id}</span>
							{#if field.required}
								<span class="required-badge">Required</span>
							{/if}
						</div>
						{#if field.description}
							<p class="field-description">{field.description}</p>
						{/if}
						<div class="field-meta">
							{#if (field.type === 'enum' || field.type === 'select') && field.constraints?.options}
								<span class="field-options">Options: {field.constraints.options.join(', ')}</span>
							{:else if field.type === 'numeric' && (field.constraints?.min !== undefined || field.constraints?.max !== undefined)}
								<span class="field-range">
									Range: {field.constraints.min ?? '∞'} - {field.constraints.max ?? '∞'}
								</span>
							{:else if field.type === 'text' && (field.constraints?.min_length !== undefined || field.constraints?.max_length !== undefined)}
								<span class="field-length">
									Length: {field.constraints.min_length ?? 0} - {field.constraints.max_length ?? '∞'}
								</span>
							{:else if field.type === 'location' && field.constraints?.format}
								<span class="field-format">Format: {field.constraints.format}</span>
							{/if}
						</div>
					</div>
					<div class="field-actions">
						<button
							class="btn-icon"
							on:click={() => moveField(index, 'up')}
							disabled={index === 0}
							title="Move up"
						>
							<ChevronUp size={14} />
						</button>
						<button
							class="btn-icon"
							on:click={() => moveField(index, 'down')}
							disabled={index === fields.length - 1}
							title="Move down"
						>
							<ChevronDown size={14} />
						</button>
						<button class="btn-icon" on:click={() => editField(index)} title="Edit">
							<Edit2 size={14} />
						</button>
						<button class="btn-icon btn-remove" on:click={() => removeField(index)} title="Remove">
							<Trash2 size={14} />
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}

	{#if !showTypeSelector}
		<button class="btn-add-field" on:click={() => showTypeSelector = true}>
			<Plus size={14} />
			Add Filter Field
		</button>
	{:else}
		<div class="type-selector">
			<h4>Select Field Type</h4>
			<p class="type-selector-description">Filter fields are mapped to database columns and can be used for searching and filtering.</p>
			<div class="type-options">
				{#each fieldTypes as type}
					<button
						class="type-option"
						class:disabled={!isTypeAvailable(type.value)}
						on:click={() => openAddFieldModal(type.value)}
						disabled={!isTypeAvailable(type.value)}>
						<span class="type-name">{type.label}</span>
						<span class="type-description">{type.description}</span>
						<span class="type-count">{getUsedFilterCount(type.value)}/5 used</span>
					</button>
				{/each}
			</div>
			<button class="btn-cancel" on:click={() => showTypeSelector = false}>Cancel</button>
		</div>
	{/if}
</div>

<!-- Field Modal -->
{#if showFieldModal}
	<div class="modal-overlay" on:click={closeModal}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h2>{editingIndex !== null ? 'Edit' : 'Add'} Filter Field</h2>
				<button class="btn-close" on:click={closeModal}>
					<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<line x1="18" y1="6" x2="6" y2="18"></line>
						<line x1="6" y1="6" x2="18" y2="18"></line>
					</svg>
				</button>
			</div>

			<div class="modal-body">
				<div class="form-grid">
					<div class="form-group">
						<label>Field Type</label>
						<input type="text" value={selectedType} disabled class="disabled-input" />
					</div>

					<div class="form-group">
						<label>Field ID</label>
						<input type="text" value={fieldForm.id} disabled class="disabled-input" />
					</div>

					<div class="form-group full-width">
						<label>Display Name <span class="required">*</span></label>
						<input
							type="text"
							bind:value={fieldForm.name}
							placeholder="e.g., Company Size"
							required
						/>
					</div>

					<div class="form-group full-width">
						<label>Description</label>
						<input
							type="text"
							bind:value={fieldForm.description}
							placeholder="Help text for this field"
						/>
					</div>

					<div class="form-group">
						<label class="checkbox-label">
							<input
								type="checkbox"
								bind:checked={fieldForm.required}
							/>
							Required field
						</label>
					</div>

					<div class="form-group">
						<label class="checkbox-label">
							<input
								type="checkbox"
								bind:checked={fieldForm.constraints.editable_by_user}
							/>
							Editable by user
						</label>
					</div>

					<!-- Type-specific constraints -->
					{#if selectedType === 'numeric'}
						<div class="form-group">
							<label>Min Value</label>
							<input
								type="number"
								bind:value={fieldForm.constraints.min}
								placeholder="Optional"
							/>
						</div>
						<div class="form-group">
							<label>Max Value</label>
							<input
								type="number"
								bind:value={fieldForm.constraints.max}
								placeholder="Optional"
							/>
						</div>
					{:else if selectedType === 'text'}
						<div class="form-group">
							<label>Min Length</label>
							<input
								type="number"
								min="0"
								bind:value={fieldForm.constraints.min_length}
								placeholder="Optional"
							/>
						</div>
						<div class="form-group">
							<label>Max Length</label>
							<input
								type="number"
								min="0"
								bind:value={fieldForm.constraints.max_length}
								placeholder="Optional"
							/>
						</div>
					{:else if selectedType === 'enum' || selectedType === 'select'}
						<div class="form-group full-width">
							<label>Options (comma-separated)</label>
							<input
								type="text"
								placeholder="e.g., Small, Medium, Large"
								value={fieldForm.constraints.options?.join(', ') || ''}
								on:input={(e) => {
									fieldForm.constraints.options = e.currentTarget.value
										.split(',')
										.map(s => s.trim())
										.filter(s => s);
								}}
							/>
						</div>
					{:else if selectedType === 'location'}
						<div class="form-group full-width">
							<label>Location Format</label>
							<select bind:value={fieldForm.constraints.format}>
								<option value="address">Street Address</option>
								<option value="coordinates">Lat/Long Coordinates</option>
								<option value="city">City Only</option>
								<option value="region">Region/State</option>
							</select>
						</div>
					{/if}
				</div>
			</div>

			<div class="modal-footer">
				<button class="btn-secondary" on:click={closeModal}>Cancel</button>
				<button class="btn-primary" on:click={saveField}>
					{editingIndex !== null ? 'Update' : 'Add'} Field
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.field-editor {
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		padding: 1rem;
		background: #f9fafb;
	}

	.fields-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin-bottom: 1rem;
	}

	.field-card {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
	}

	.field-handle {
		color: #9ca3af;
		cursor: move;
	}

	.field-info {
		flex: 1;
	}

	.field-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 0.25rem;
	}

	.field-header h4 {
		margin: 0;
		font-size: 0.875rem;
		font-weight: 600;
		color: #111827;
	}

	.field-type-badge,
	.field-id-badge,
	.required-badge {
		padding: 0.125rem 0.5rem;
		border-radius: 0.25rem;
		font-size: 0.75rem;
		font-weight: 500;
	}

	.field-type-badge {
		background: #dbeafe;
		color: #1e40af;
	}

	.field-id-badge {
		background: #f3f4f6;
		color: #6b7280;
		font-family: monospace;
	}

	.required-badge {
		background: #fee2e2;
		color: #b91c1c;
	}

	.field-description {
		margin: 0.25rem 0;
		font-size: 0.8125rem;
		color: #6b7280;
	}

	.field-meta {
		display: flex;
		gap: 1rem;
		margin-top: 0.25rem;
	}

	.field-options,
	.field-range,
	.field-length,
	.field-format {
		font-size: 0.75rem;
		color: #9ca3af;
	}

	.field-actions {
		display: flex;
		gap: 0.25rem;
	}

	.btn-icon {
		padding: 0.25rem 0.5rem;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.25rem;
		cursor: pointer;
		transition: all 0.2s;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.btn-icon:hover:not(:disabled) {
		background: #f3f4f6;
		border-color: #d1d5db;
	}

	.btn-icon:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-remove {
		color: #ef4444;
	}

	.btn-remove:hover {
		background: #fee2e2;
		border-color: #f87171;
	}

	.btn-add-field {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		background: white;
		border: 1px dashed #d1d5db;
		border-radius: 0.375rem;
		color: #6b7280;
		cursor: pointer;
		transition: all 0.2s;
		width: 100%;
		justify-content: center;
	}

	.btn-add-field:hover {
		border-color: #9ca3af;
		background: #f9fafb;
		color: #374151;
	}

	.type-selector {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		padding: 1.5rem;
	}

	.type-selector h4 {
		margin: 0 0 0.5rem 0;
		font-size: 1rem;
		font-weight: 600;
		color: #111827;
	}

	.type-selector-description {
		margin: 0 0 1rem 0;
		font-size: 0.875rem;
		color: #6b7280;
	}

	.type-options {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
		gap: 0.75rem;
		margin-bottom: 1rem;
	}

	.type-option {
		display: flex;
		flex-direction: column;
		padding: 1rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		background: white;
		cursor: pointer;
		transition: all 0.2s;
		text-align: left;
	}

	.type-option:hover:not(.disabled) {
		border-color: #3b82f6;
		background: #eff6ff;
	}

	.type-option.disabled {
		opacity: 0.5;
		cursor: not-allowed;
		background: #f9fafb;
	}

	.type-name {
		font-size: 0.875rem;
		font-weight: 600;
		color: #111827;
		margin-bottom: 0.25rem;
	}

	.type-description {
		font-size: 0.75rem;
		color: #6b7280;
		margin-bottom: 0.5rem;
	}

	.type-count {
		font-size: 0.75rem;
		color: #9ca3af;
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
		width: 100%;
	}

	.btn-cancel:hover {
		background: #f9fafb;
	}

	/* Modal Styles */
	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal {
		background: white;
		border-radius: 0.5rem;
		width: 90%;
		max-width: 600px;
		max-height: 90vh;
		display: flex;
		flex-direction: column;
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1.5rem;
		border-bottom: 1px solid #e5e7eb;
	}

	.modal-header h2 {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 600;
		color: #111827;
	}

	.btn-close {
		background: none;
		border: none;
		padding: 0.25rem;
		cursor: pointer;
		color: #6b7280;
		transition: color 0.2s;
	}

	.btn-close:hover {
		color: #111827;
	}

	.modal-body {
		padding: 1.5rem;
		overflow-y: auto;
		flex: 1;
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid #e5e7eb;
		background: #f9fafb;
	}

	.form-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
	}

	.form-group {
		display: flex;
		flex-direction: column;
	}

	.form-group.full-width {
		grid-column: 1 / -1;
	}

	.form-group label {
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
		margin-bottom: 0.375rem;
	}

	.form-group input,
	.form-group select {
		padding: 0.5rem;
		border: 1px solid #d1d5db;
		border-radius: 0.375rem;
		font-size: 0.875rem;
	}

	.form-group input:focus,
	.form-group select:focus {
		outline: none;
		border-color: #3b82f6;
		box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
	}

	.disabled-input {
		background: #f3f4f6;
		color: #6b7280;
		cursor: not-allowed;
	}

	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
		font-size: 0.875rem;
		color: #374151;
	}

	.required {
		color: #ef4444;
	}

	.btn-primary,
	.btn-secondary {
		padding: 0.5rem 1rem;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		border: 1px solid transparent;
	}

	.btn-primary {
		background: #3b82f6;
		color: white;
	}

	.btn-primary:hover {
		background: #2563eb;
	}

	.btn-secondary {
		background: white;
		color: #374151;
		border-color: #d1d5db;
	}

	.btn-secondary:hover {
		background: #f3f4f6;
	}
</style>