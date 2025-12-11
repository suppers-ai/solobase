<script lang="ts">
	import { Plus, Trash2, GripVertical, Edit2, ChevronUp, ChevronDown } from 'lucide-svelte';
	import { toasts } from '$lib/stores/toast';

	interface FieldConstraints {
		editableByUser: boolean;
		options?: string[];
		min?: number;
		max?: number;
		step?: number;
		minLength?: number;
		maxLength?: number;
		default?: string;
	}

	interface CustomField {
		id: string;
		name: string;
		type: string;
		required: boolean;
		description: string;
		section: string;
		order: number;
		constraints: FieldConstraints;
	}

	export let fields: CustomField[] = [];
	export let onFieldsChange: (fields: CustomField[]) => void;

	let showFieldModal = false;
	let editingIndex: number | null = null;
	let fieldForm: CustomField = {
		id: '',
		name: '',
		type: 'text',
		required: false,
		description: '',
		section: 'General',
		order: 0,
		constraints: {
			editableByUser: true
		}
	};

	const fieldTypes = [
		{ value: 'text', label: 'Text' },
		{ value: 'number', label: 'Number' },
		{ value: 'boolean', label: 'Boolean' },
		{ value: 'select', label: 'Select/Dropdown' },
		{ value: 'color', label: 'Color' },
		{ value: 'url', label: 'URL' },
		{ value: 'email', label: 'Email' },
		{ value: 'date', label: 'Date' },
		{ value: 'datetime', label: 'Date & Time' },
		{ value: 'json', label: 'JSON' },
		{ value: 'textarea', label: 'Long Text' },
		{ value: 'upload', label: 'File Upload' },
		{ value: 'range', label: 'Range/Slider' }
	];

	function addField() {
		resetFieldForm();
		showFieldModal = true;
		editingIndex = null;
	}

	function editField(index: number) {
		const field = fields[index];
		fieldForm = {
			...field,
			constraints: {
				...field.constraints,
				editableByUser: field.constraints?.editableByUser ?? true
			}
		};
		editingIndex = index;
		showFieldModal = true;
	}

	function saveField() {
		if (!fieldForm.name) {
			toasts.warning('Please provide a field name');
			return;
		}

		// Generate ID from name if not provided
		if (!fieldForm.id) {
			fieldForm.id = fieldForm.name.toLowerCase().replace(/\s+/g, '_').replace(/[^a-z0-9_]/g, '');
		}

		const newField = {
			...fieldForm,
			order: editingIndex !== null ? fields[editingIndex].order : fields.length
		};

		if (editingIndex !== null) {
			// Update existing field
			fields[editingIndex] = newField;
		} else {
			// Add new field
			fields = [...fields, newField];
		}

		onFieldsChange(fields);
		resetFieldForm();
		showFieldModal = false;
	}

	function removeField(index: number) {
		fields = fields.filter((_, i) => i !== index);
		onFieldsChange(fields);
	}

	function resetFieldForm() {
		fieldForm = {
			id: '',
			name: '',
			type: 'text',
			required: false,
			description: '',
			section: 'General',
			order: 0,
			constraints: {
				editableByUser: true,
				options: undefined,
				min: undefined,
				max: undefined,
				step: undefined,
				minLength: undefined,
				maxLength: undefined,
				default: undefined
			}
		};
	}

	function cancelEdit() {
		resetFieldForm();
		showFieldModal = false;
		editingIndex = null;
	}

	function moveField(index: number, direction: 'up' | 'down') {
		const newIndex = direction === 'up' ? index - 1 : index + 1;
		if (newIndex < 0 || newIndex >= fields.length) return;

		const newFields = [...fields];
		[newFields[index], newFields[newIndex]] = [newFields[newIndex], newFields[index]];

		// Update order values
		newFields.forEach((field, i) => {
			field.order = i;
		});

		fields = newFields;
		onFieldsChange(fields);
	}
</script>

<div class="custom-field-editor">
	{#if fields.length > 0}
		<div class="fields-list">
			{#each fields as field, index}
				<div class="field-card">
					<div class="field-handle">
						<GripVertical size={16} />
					</div>
					<div class="field-info">
						<div class="field-header">
							<h4>{field.name}</h4>
							<span class="field-type-badge">{field.type}</span>
							{#if field.required}
								<span class="required-badge">Required</span>
							{/if}
							{#if field.constraints?.editableByUser !== true}
								<span class="readonly-badge">Read-only</span>
							{/if}
							{#if field.section && field.section !== 'General'}
								<span class="section-badge">{field.section}</span>
							{/if}
						</div>
						{#if field.description}
							<p class="field-description">{field.description}</p>
						{/if}
						<div class="field-meta">
							<span class="field-id">ID: {field.id}</span>
							{#if field.type === 'select' && field.constraints?.options}
								<span class="field-options">Options: {field.constraints.options.join(', ')}</span>
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

	<button class="btn-add-field" on:click={addField}>
		<Plus size={14} />
		Add Custom Field
	</button>
</div>

<!-- Field Modal -->
{#if showFieldModal}
	<div class="modal-overlay" on:click={cancelEdit}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h2>{editingIndex !== null ? 'Edit' : 'Add'} Custom Field</h2>
				<button class="btn-close" on:click={cancelEdit}>
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
					<select bind:value={fieldForm.type}>
						{#each fieldTypes as type}
							<option value={type.value}>{type.label}</option>
						{/each}
					</select>
				</div>

				<div class="form-group">
					<label>Field Name</label>
					<input
						type="text"
						bind:value={fieldForm.name}
						placeholder="e.g., Background Color"
						required
					/>
				</div>

				<div class="form-group">
					<label>Field ID (optional)</label>
					<input
						type="text"
						bind:value={fieldForm.id}
						placeholder="Auto-generated from name"
					/>
				</div>

				<div class="form-group">
					<label>Section</label>
					<input
						type="text"
						bind:value={fieldForm.section}
						placeholder="e.g., Colors, Typography, Advanced"
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
							bind:checked={fieldForm.constraints.editableByUser}
						/>
						Editable by user
					</label>
				</div>

				<!-- Type-specific options -->
				{#if fieldForm.type === 'select'}
					<div class="form-group full-width">
						<label>Options (comma-separated)</label>
						<input
							type="text"
							value={fieldForm.constraints.options?.join(', ') || ''}
							on:input={(e) => {
								fieldForm.constraints.options = e.currentTarget.value
									.split(',')
									.map(s => s.trim())
									.filter(s => s);
							}}
							placeholder="e.g., Option 1, Option 2, Option 3"
						/>
					</div>
				{:else if fieldForm.type === 'number' || fieldForm.type === 'range'}
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
					{#if fieldForm.type === 'range'}
						<div class="form-group">
							<label>Step</label>
							<input
								type="number"
								bind:value={fieldForm.constraints.step}
								placeholder="1"
							/>
						</div>
					{/if}
				{:else if fieldForm.type === 'text' || fieldForm.type === 'textarea'}
					<div class="form-group">
						<label>Min Length</label>
						<input
							type="number"
							bind:value={fieldForm.constraints.minLength}
							placeholder="Optional"
							min="0"
						/>
					</div>
					<div class="form-group">
						<label>Max Length</label>
						<input
							type="number"
							bind:value={fieldForm.constraints.maxLength}
							placeholder="Optional"
							min="0"
						/>
					</div>
				{:else if fieldForm.type === 'color'}
					<div class="form-group">
						<label>Default Color</label>
						<input
							type="text"
							bind:value={fieldForm.constraints.default}
							placeholder="#000000"
						/>
					</div>
				{/if}
				</div>
			</div>

			<div class="modal-footer">
				<button class="btn-secondary" on:click={cancelEdit}>Cancel</button>
				<button class="btn-primary" on:click={saveField}>
					{editingIndex !== null ? 'Update' : 'Add'} Field
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.custom-field-editor {
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
	.required-badge,
	.section-badge {
		padding: 0.125rem 0.5rem;
		border-radius: 0.25rem;
		font-size: 0.75rem;
		font-weight: 500;
	}

	.field-type-badge {
		background: #dbeafe;
		color: #1e40af;
	}

	.required-badge {
		background: #fee2e2;
		color: #b91c1c;
	}

	.section-badge {
		background: #f3e8ff;
		color: #6b21a8;
	}

	.readonly-badge {
		background: #fef3c7;
		color: #92400e;
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

	.field-id,
	.field-options {
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
		margin-bottom: 1rem;
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

	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
		font-size: 0.875rem;
		color: #374151;
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