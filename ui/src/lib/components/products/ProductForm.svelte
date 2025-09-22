<script context="module" lang="ts">
	export interface ProductFormProps {
		mode?: 'create' | 'edit';
		product?: any;
		productTemplates?: any[];
		groups?: any[];
		customFieldsConfig?: any;
		submitButtonText?: string;
		onSubmit?: (data: any) => void;
		onCancel?: () => void;
	}
</script>

<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let mode: 'create' | 'edit' = 'create';
	export let product: any = null;
	export let productTemplates: any[] = [];
	export let groups: any[] = [];
	export let customFieldsConfig: any = null;
	export let initialGroupId: number | string | null = null;
	export let submitButtonText = mode === 'create' ? 'Create' : 'Save';
	export let onSubmit: ((data: any) => void) | undefined = undefined;
	export let onCancel: (() => void) | undefined = undefined;

	const dispatch = createEventDispatcher();

	let formData: any = {
		group_id: mode === 'create' && initialGroupId ? initialGroupId : '',
		product_template_id: '',
		name: '',
		description: '',
		base_price: 0,
		currency: 'USD',
		active: true,
		filter_fields: {},
		custom_fields: {}
	};

	// Store template separately to avoid circular dependency
	let selectedTemplate: any = null;
	let templateFields: any[] = [];
	let customFields: any[] = [];

	// Initialize form data from existing product
	function initializeFromProduct() {
		if (mode === 'edit' && product) {
			const newFormData = {
				...product,
				filter_fields: {},
				custom_fields: product.custom_fields || {}
			};

			// Find template and map filter fields
			const template = productTemplates.find(t => t.id === product.product_template_id);
			if (template?.fields) {
				template.fields.forEach((field: any) => {
					if (field.id && product[field.id] !== undefined) {
						newFormData.filter_fields[field.id] = product[field.id];
					}
				});
			}

			formData = newFormData;
			updateSelectedTemplate();
		}
	}

	// Update selected template when product_template_id changes
	function updateSelectedTemplate() {
		// Convert to number for comparison since select values are strings
		const templateId = typeof formData.product_template_id === 'string'
			? parseInt(formData.product_template_id)
			: formData.product_template_id;

		selectedTemplate = productTemplates.find(t => t.id === templateId) || null;
		templateFields = selectedTemplate?.fields || [];
		customFields = customFieldsConfig || selectedTemplate?.custom_field_definitions || [];
	}

	// React to product changes
	$: if (mode === 'edit' && product) {
		initializeFromProduct();
	}

	// React to template ID changes
	$: if (formData.product_template_id) {
		updateSelectedTemplate();
	}

	function updateField(fieldPath: string, value: any) {
		const keys = fieldPath.split('.');

		if (keys.length === 1) {
			// Simple field update
			formData = {
				...formData,
				[keys[0]]: value
			};
		} else {
			// Nested field update
			const newFormData = { ...formData };
			let target = newFormData;

			for (let i = 0; i < keys.length - 1; i++) {
				const key = keys[i];
				if (!target[key]) {
					target[key] = {};
				}
				target = target[key];
			}

			target[keys[keys.length - 1]] = value;
			formData = newFormData;
		}

		// If product_template_id changed, update the selected template
		if (fieldPath === 'product_template_id') {
			updateSelectedTemplate();
		}
	}

	function updateFieldValue(fieldId: string, value: any, isCustom: boolean = false) {
		if (isCustom) {
			formData.custom_fields = {
				...formData.custom_fields,
				[fieldId]: value
			};
		} else {
			formData.filter_fields = {
				...formData.filter_fields,
				[fieldId]: value
			};
		}
		formData = formData;
	}

	function getFieldComponent(field: any) {
		const value = formData.filter_fields[field.id] || field.constraints?.default;
		return { field, value };
	}

	function handleSubmit() {
		const submitData = {
			...formData,
			...formData.filter_fields
		};
		delete submitData.filter_fields;

		if (onSubmit) {
			onSubmit(submitData);
		}

		dispatch('submit', submitData);
	}

	function handleCancel() {
		if (onCancel) {
			onCancel();
		}
		dispatch('cancel');
	}

	function validateForm(): { valid: boolean; errors: string[] } {
		const errors: string[] = [];

		if (mode === 'create') {
			if (!formData.group_id) errors.push('Group is required');
			if (!formData.product_template_id) errors.push('Product type is required');
		}

		if (!formData.name) errors.push('Name is required');

		templateFields.forEach((field: any) => {
			if (field.required && !formData.filter_fields[field.id]) {
				errors.push(`${field.name} is required`);
			}
		});

		customFields.forEach((field: any) => {
			if (field.required && !formData.custom_fields[field.id]) {
				errors.push(`${field.name} is required`);
			}
		});

		return {
			valid: errors.length === 0,
			errors
		};
	}
</script>

<form on:submit|preventDefault={handleSubmit}>
	<div class="form-content">
		<!-- Basic Fields -->
		<div class="form-section">
			<h3>Basic Information</h3>

			{#if mode === 'create'}
				<div class="form-row">
					<div class="form-group">
						<label for="group">Group <span class="required">*</span></label>
						<select id="group" value={String(formData.group_id || '')} on:change={(e) => updateField('group_id', e.currentTarget.value ? parseInt(e.currentTarget.value) : '')} required>
							<option value="">Select a group</option>
							{#each groups as group}
								<option value={String(group.id)}>{group.name || `Group ${group.id}`}</option>
							{/each}
						</select>
					</div>

					<div class="form-group">
						<label for="template">Product Type <span class="required">*</span></label>
						<select id="template" value={String(formData.product_template_id || '')} on:change={(e) => updateField('product_template_id', e.currentTarget.value ? parseInt(e.currentTarget.value) : '')} required>
							<option value="">Select a type</option>
							{#each productTemplates as template}
								<option value={String(template.id)}>{template.display_name || template.name}</option>
							{/each}
						</select>
					</div>
				</div>
			{/if}

			<div class="form-row">
				<div class="form-group">
					<label for="name">Name <span class="required">*</span></label>
					<input
						id="name"
						type="text"
						value={formData.name}
						on:input={(e) => updateField('name', e.currentTarget.value)}
						placeholder="Product name"
						required
					/>
				</div>

				<div class="form-group">
					<label for="base_price">Base Price</label>
					<input
						id="base_price"
						type="number"
						value={formData.base_price}
						on:input={(e) => updateField('base_price', parseFloat(e.currentTarget.value) || 0)}
						step="0.01"
						min="0"
					/>
				</div>
			</div>

			<div class="form-group">
				<label for="description">Description</label>
				<textarea
					id="description"
					value={formData.description}
					on:input={(e) => updateField('description', e.currentTarget.value)}
					rows="3"
					placeholder="Product description"
				></textarea>
			</div>
		</div>

		<!-- Template Fields (Filter Fields) -->
		{#if templateFields.length > 0}
			<div class="form-section">
				<h3>Product Properties</h3>

				{#each templateFields as field}
					{@const fieldData = getFieldComponent(field)}
					<div class="form-group">
						<label for={field.id}>
							{field.name}
							{#if field.required}<span class="required">*</span>{/if}
						</label>

						{#if field.description}
							<p class="field-description">{field.description}</p>
						{/if}

						<!-- Render based on field type -->
						{#if field.type === 'enum' || field.type === 'select'}
							<select
								id={field.id}
								value={formData.filter_fields[field.id] || ''}
								on:change={(e) => updateFieldValue(field.id, e.currentTarget.value)}
								required={field.required}
							>
								<option value="">Select {field.name}</option>
								{#each field.constraints?.options || [] as option}
									<option value={option}>{option}</option>
								{/each}
							</select>
						{:else if field.type === 'boolean'}
							<select
								id={field.id}
								value={formData.filter_fields[field.id]}
								on:change={(e) => updateFieldValue(field.id, e.currentTarget.value === 'true')}
								required={field.required}
							>
								<option value="">Select</option>
								<option value="true">Yes</option>
								<option value="false">No</option>
							</select>
						{:else if field.type === 'numeric' || field.type === 'number'}
							<input
								id={field.id}
								type="number"
								value={formData.filter_fields[field.id] || ''}
								on:input={(e) => updateFieldValue(field.id, parseFloat(e.currentTarget.value) || 0)}
								min={field.constraints?.min}
								max={field.constraints?.max}
								step="any"
								placeholder={field.constraints?.placeholder}
								required={field.required}
							/>
						{:else if field.type === 'color'}
							<div class="color-input-wrapper">
								<input
									id={field.id}
									type="color"
									value={formData.filter_fields[field.id] || '#000000'}
									on:input={(e) => updateFieldValue(field.id, e.currentTarget.value)}
									required={field.required}
									class="color-picker"
								/>
								<input
									type="text"
									value={formData.filter_fields[field.id] || '#000000'}
									on:input={(e) => updateFieldValue(field.id, e.currentTarget.value)}
									placeholder="#000000"
									pattern="^#[0-9A-Fa-f]{6}$"
									class="color-text"
								/>
							</div>
						{:else}
							<!-- Default to text input -->
							<input
								id={field.id}
								type="text"
								value={formData.filter_fields[field.id] || ''}
								on:input={(e) => updateFieldValue(field.id, e.currentTarget.value)}
								placeholder={field.constraints?.placeholder}
								maxlength={field.constraints?.max_length}
								required={field.required}
							/>
						{/if}
					</div>
				{/each}
			</div>
		{/if}

		<!-- Custom Fields (stored in CustomFields JSON) -->
		{#if customFields.length > 0}
			<div class="form-section">
				<h3>Additional Configuration</h3>

				{#each customFields as field}
					<div class="form-group">
						<label for={`custom-${field.id}`}>
							{field.name}
							{#if field.required}<span class="required">*</span>{/if}
						</label>

						{#if field.description}
							<p class="field-description">{field.description}</p>
						{/if}

						{#if field.type === 'color'}
							<div class="color-input-wrapper">
								<input
									id={`custom-${field.id}`}
									type="color"
									value={formData.custom_fields[field.id] || field.default || '#000000'}
									on:input={(e) => updateFieldValue(field.id, e.currentTarget.value, true)}
									required={field.required}
									class="color-picker"
								/>
								<input
									type="text"
									value={formData.custom_fields[field.id] || field.default || '#000000'}
									on:input={(e) => updateFieldValue(field.id, e.currentTarget.value, true)}
									placeholder="#000000"
									pattern="^#[0-9A-Fa-f]{6}$"
									class="color-text"
								/>
							</div>
						{:else if field.type === 'enum' || field.type === 'select'}
							<select
								id={`custom-${field.id}`}
								value={formData.custom_fields[field.id] || field.default}
								on:change={(e) => updateFieldValue(field.id, e.currentTarget.value, true)}
								required={field.required}
							>
								<option value="">Select {field.name}</option>
								{#each field.options || [] as option}
									<option value={option}>{option}</option>
								{/each}
							</select>
						{:else if field.type === 'numeric' || field.type === 'number'}
							<input
								id={`custom-${field.id}`}
								type="number"
								value={formData.custom_fields[field.id] || field.default}
								on:input={(e) => updateFieldValue(field.id, parseFloat(e.currentTarget.value), true)}
								min={field.min}
								max={field.max}
								step="any"
								placeholder={field.placeholder}
								required={field.required}
							/>
						{:else}
							<input
								id={`custom-${field.id}`}
								type="text"
								value={formData.custom_fields[field.id] || field.default || ''}
								on:input={(e) => updateFieldValue(field.id, e.currentTarget.value, true)}
								placeholder={field.placeholder}
								required={field.required}
							/>
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	</div>

	<div class="form-footer">
		<button type="button" class="btn btn-secondary" on:click={handleCancel}>
			Cancel
		</button>
		<button type="submit" class="btn btn-primary">
			{submitButtonText}
		</button>
	</div>
</form>

<style>
	.form-content {
		padding: 1.5rem;
		overflow-y: auto;
		max-height: 60vh;
	}

	.form-section {
		margin-bottom: 2rem;
	}

	.form-section:last-child {
		margin-bottom: 0;
	}

	.form-section h3 {
		font-size: 1rem;
		font-weight: 600;
		color: #374151;
		margin-bottom: 1rem;
		padding-bottom: 0.5rem;
		border-bottom: 1px solid #e5e7eb;
	}

	.form-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
	}

	.form-group {
		margin-bottom: 1rem;
	}

	.form-group label {
		display: block;
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
		margin-bottom: 0.25rem;
	}

	.field-description {
		font-size: 0.75rem;
		color: #6b7280;
		margin: 0.25rem 0;
	}

	.required {
		color: #ef4444;
	}

	input[type="text"],
	input[type="number"],
	textarea,
	select {
		width: 100%;
		padding: 0.5rem 0.75rem;
		border: 1px solid #d1d5db;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		transition: all 0.2s;
	}

	input[type="text"]:focus,
	input[type="number"]:focus,
	textarea:focus,
	select:focus {
		outline: none;
		border-color: #3b82f6;
		box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
	}

	textarea {
		resize: vertical;
		font-family: inherit;
	}

	.color-input-wrapper {
		display: flex;
		gap: 0.5rem;
		align-items: center;
	}

	.color-picker {
		width: 60px;
		height: 38px;
		padding: 0.25rem;
		border: 1px solid #d1d5db;
		border-radius: 0.375rem;
		cursor: pointer;
	}

	.color-text {
		flex: 1;
		font-family: 'Courier New', monospace;
		text-transform: uppercase;
	}

	.form-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid #e5e7eb;
		background: #f9fafb;
	}

	.btn {
		padding: 0.5rem 1rem;
		font-size: 0.875rem;
		font-weight: 500;
		border-radius: 0.375rem;
		border: 1px solid transparent;
		cursor: pointer;
		transition: all 0.2s;
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

	@media (max-width: 640px) {
		.form-row {
			grid-template-columns: 1fr;
		}
	}
</style>