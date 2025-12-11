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
	import FieldRenderer from './FieldRenderer.svelte';
	import NineSliceUpload from './NineSliceUpload.svelte';
	import { toasts } from '$lib/stores/toast';

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
		groupId: mode === 'create' && initialGroupId ? initialGroupId : '',
		productTemplateId: '',
		name: '',
		description: '',
		basePrice: 0,
		currency: 'USD',
		active: true,
		filterFields: {},
		customFields: {}
	};

	let selectedTemplate: any = null;
	let templateFields: any[] = [];
	let customFields: any[] = [];
	let activeTab = '';
	let sections: Map<string, any[]> = new Map();

	// Initialize form data from existing product
	function initializeFromProduct() {
		if (mode === 'edit' && product) {
			const newFormData = {
				...product,
				filterFields: {},
				customFields: product.customFields || {}
			};

			// Find template and map filter fields
			const template = productTemplates.find(t => t.id === product.productTemplateId);
			if (template?.filterFieldsSchema) {
				template.filterFieldsSchema.forEach((field: any) => {
					if (field.id && product[field.id] !== undefined) {
						newFormData.filterFields[field.id] = product[field.id];
					}
				});
			}

			formData = newFormData;
			updateSelectedTemplate();
		}
	}

	// Update selected template when productTemplateId changes
	function updateSelectedTemplate() {
		const templateId = typeof formData.productTemplateId === 'string'
			? parseInt(formData.productTemplateId)
			: formData.productTemplateId;

		selectedTemplate = productTemplates.find(t => t.id === templateId) || null;
		templateFields = selectedTemplate?.filterFieldsSchema || [];
		// Only use customFieldsConfig if it's explicitly provided and not empty
		customFields = (customFieldsConfig && customFieldsConfig.length > 0)
			? customFieldsConfig
			: (selectedTemplate?.customFieldsSchema || []);

		// Group custom fields by section
		organizeFieldsBySections();
	}

	// Organize fields into sections for tabbed display
	function organizeFieldsBySections() {
		sections = new Map();

		if (customFields.length > 0) {
			customFields.forEach(field => {
				const sectionName = field.section || 'General';
				if (!sections.has(sectionName)) {
					sections.set(sectionName, []);
				}
				sections.get(sectionName)?.push(field);
			});

			// Sort fields within each section by order
			sections.forEach((fields) => {
				fields.sort((a, b) => (a.order || 0) - (b.order || 0));
			});

			// Set initial active tab
			if (sections.size > 0 && !activeTab) {
				activeTab = Array.from(sections.keys())[0];
			}
		}
	}

	// React to product changes
	$: if (mode === 'edit' && product) {
		initializeFromProduct();
	}

	// React to template ID changes
	$: if (formData.productTemplateId) {
		updateSelectedTemplate();
	}

	function updateField(fieldPath: string, value: any) {
		formData = { ...formData, [fieldPath]: value };

		if (fieldPath === 'productTemplateId') {
			updateSelectedTemplate();
		}

		// Emit field change event for preview updates
		dispatch('fieldChange', {
			fieldId: fieldPath,
			value,
			formData: { ...formData }
		});
	}

	function updateFieldValue(fieldId: string, value: any, isCustom: boolean = false) {
		if (isCustom) {
			formData.customFields = {
				...formData.customFields,
				[fieldId]: value
			};
		} else {
			formData.filterFields = {
				...formData.filterFields,
				[fieldId]: value
			};
		}
		formData = formData;

		// Emit field change event for preview updates
		dispatch('fieldChange', {
			fieldId: isCustom ? `custom_${fieldId}` : `filter_${fieldId}`,
			value,
			formData: { ...formData }
		});
	}

	function getFieldValue(field: any, isCustom = false) {
		const source = isCustom ? formData.customFields : formData.filterFields;
		return source[field.id] || field.default || field.constraints?.default || '';
	}

	function handleFileUpload(e: Event, fieldId: string, isCustom = false) {
		const target = e.currentTarget as HTMLInputElement;
		const file = target.files?.[0];
		if (!file) return;

		const uploadedId = `pending_upload_${Date.now()}`;
		updateFieldValue(fieldId, uploadedId, isCustom);
		dispatch('fileUpload', { field: fieldId, file, customField: isCustom });
	}

	function handleSubmit() {
		const validation = validateForm();
		if (!validation.valid) {
			toasts.warning('Please fix the following errors:\n' + validation.errors.join('\n'));
			return;
		}

		const submitData = {
			...formData,
			...formData.filterFields
		};
		delete submitData.filterFields;

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
			if (!formData.groupId) errors.push('Group is required');
			if (!formData.productTemplateId) errors.push('Product type is required');
		}

		if (!formData.name) errors.push('Name is required');

		templateFields.forEach((field: any) => {
			// Only validate fields that are editable by users
			if (field.constraints?.editableByUser === true && field.required && !formData.filterFields[field.id]) {
				errors.push(`${field.name} is required`);
			}
		});

		customFields.forEach((field: any) => {
			// Only validate fields that are editable by users
			if (field.constraints?.editableByUser === true && field.required && !formData.customFields[field.id]) {
				errors.push(`${field.name} is required`);
			}
		});

		return {
			valid: errors.length === 0,
			errors
		};
	}
</script>

<form on:submit|preventDefault={handleSubmit} novalidate>
	<div class="form-content">
		<!-- Basic Fields -->
		<div class="form-section">
			<h3>Basic Information</h3>

			{#if mode === 'create'}
				<div class="form-row">
					<div class="form-group">
						<label for="group">Group <span class="required">*</span></label>
						<select
							id="group"
							value={String(formData.groupId || '')}
							on:change={(e) => updateField('groupId', e.currentTarget.value ? parseInt(e.currentTarget.value) : '')}
							required
						>
							<option value="">Select a group</option>
							{#each groups as group}
								<option value={String(group.id)}>{group.name || `Group ${group.id}`}</option>
							{/each}
						</select>
					</div>

					<div class="form-group">
						<label for="template">Product Type <span class="required">*</span></label>
						<select
							id="template"
							value={String(formData.productTemplateId || '')}
							on:change={(e) => updateField('productTemplateId', e.currentTarget.value ? parseInt(e.currentTarget.value) : '')}
							required
						>
							<option value="">Select a type</option>
							{#each productTemplates as template}
								<option value={String(template.id)}>{template.displayName || template.name}</option>
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
					<label for="basePrice">Base Price</label>
					<input
						id="basePrice"
						type="number"
						value={formData.basePrice}
						on:input={(e) => updateField('basePrice', parseFloat(e.currentTarget.value) || 0)}
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
					{#if field.constraints?.editableByUser === true}
						<div class="form-group">
							<label for={field.id}>
								{field.name}
								{#if field.required}<span class="required">*</span>{/if}
							</label>

							{#if field.description}
								<p class="field-description">{field.description}</p>
							{/if}

							<FieldRenderer
								{field}
								value={getFieldValue(field)}
								fieldId={field.id}
								required={field.required}
								onUpdate={(val) => updateFieldValue(field.id, val)}
								onFileUpload={field.type === 'upload' ? (e) => handleFileUpload(e, field.id) : null}
							/>
						</div>
					{/if}
				{/each}
			</div>
		{/if}

		<!-- Custom Fields (stored in CustomFields JSON) -->
		{#if customFields.length > 0}
			<div class="form-section">
				<h3>Additional Configuration</h3>

				{#if sections.size > 1}
					<!-- Tabbed interface when there are multiple sections -->
					<div class="tabs">
						<div class="tab-list">
							{#each Array.from(sections.keys()) as sectionName}
								<button
									type="button"
									class="tab-button"
									class:active={activeTab === sectionName}
									on:click={() => activeTab = sectionName}
								>
									{sectionName}
								</button>
							{/each}
						</div>

						<div class="tab-content">
							{#each Array.from(sections.entries()) as [sectionName, sectionFields]}
								{#if activeTab === sectionName}
									<div class="tab-panel">
										{#each sectionFields as field}
											{#if field.constraints?.editableByUser === true}
												<div class="form-group">
													<label for={`custom-${field.id}`}>
														{field.name}
														{#if field.required}<span class="required">*</span>{/if}
													</label>

													{#if field.description}
														<p class="field-description">{field.description}</p>
													{/if}

													{#if field.type === 'nine-slice-upload'}
														<NineSliceUpload
															fieldId={field.id}
															value={formData.customFields[field.id] || {}}
															onUpdate={(val) => updateFieldValue(field.id, val, true)}
															on:fileUpload
														/>
													{:else}
														<FieldRenderer
															{field}
														value={getFieldValue(field, true)}
														fieldId={`custom-${field.id}`}
														required={field.required}
														onUpdate={(val) => updateFieldValue(field.id, val, true)}
														onFileUpload={field.type === 'upload' ? (e) => handleFileUpload(e, field.id, true) : null}
													/>
												{/if}
											</div>
											{/if}
										{/each}
									</div>
								{/if}
							{/each}
						</div>
					</div>
				{:else}
					<!-- Single section or no sections - flat layout -->
					{#each customFields as field}
						{#if field.constraints?.editableByUser === true}
							<div class="form-group">
								<label for={`custom-${field.id}`}>
									{field.name}
									{#if field.required}<span class="required">*</span>{/if}
								</label>

								{#if field.description}
									<p class="field-description">{field.description}</p>
								{/if}

								{#if field.type === 'nine-slice-upload'}
									<NineSliceUpload
										fieldId={field.id}
										value={formData.customFields[field.id] || {}}
										onUpdate={(val) => updateFieldValue(field.id, val, true)}
										on:fileUpload
									/>
								{:else}
									<FieldRenderer
										{field}
										value={getFieldValue(field, true)}
										fieldId={`custom-${field.id}`}
										required={field.required}
										onUpdate={(val) => updateFieldValue(field.id, val, true)}
										onFileUpload={field.type === 'upload' ? (e) => handleFileUpload(e, field.id, true) : null}
									/>
								{/if}
							</div>
						{/if}
					{/each}
				{/if}
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
		max-height: 70vh;
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
		margin-bottom: 1.25rem;
	}

	.form-group label {
		display: block;
		font-size: 0.875rem;
		font-weight: 600;
		color: #111827;
		margin-bottom: 0.5rem;
	}

	.field-description {
		font-size: 0.8125rem;
		color: #6b7280;
		margin: 0.25rem 0 0.5rem 0;
		line-height: 1.4;
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

	/* Tab styles */
	.tabs {
		margin-top: 1rem;
		width: 100%;
		display: flex;
		flex-direction: column;
	}

	.tab-list {
		display: flex;
		gap: 0.5rem;
		border-bottom: 2px solid #e5e7eb;
		margin-bottom: 1.5rem;
		overflow-x: auto;
		overflow-y: hidden;
		width: 100%;
		flex-wrap: nowrap;
		white-space: nowrap;
		scrollbar-width: thin;
		scrollbar-color: #d1d5db #f9fafb;
	}

	.tab-list::-webkit-scrollbar {
		height: 6px;
	}

	.tab-list::-webkit-scrollbar-track {
		background: #f9fafb;
		border-radius: 3px;
	}

	.tab-list::-webkit-scrollbar-thumb {
		background: #d1d5db;
		border-radius: 3px;
	}

	.tab-list::-webkit-scrollbar-thumb:hover {
		background: #9ca3af;
	}

	.tab-button {
		padding: 0.5rem 1rem;
		font-size: 0.875rem;
		font-weight: 500;
		color: #6b7280;
		background: transparent;
		border: none;
		border-bottom: 2px solid transparent;
		margin-bottom: -2px;
		cursor: pointer;
		transition: all 0.2s;
		white-space: nowrap;
	}

	.tab-button:hover {
		color: #374151;
		background: #f9fafb;
	}

	.tab-button.active {
		color: #3b82f6;
		border-bottom-color: #3b82f6;
		background: transparent;
	}

	.tab-content {
		min-height: 300px;
		width: 100%;
		padding: 1rem 0;
		display: block;
		clear: both;
	}

	.tab-panel {
		animation: fadeIn 0.2s;
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1.25rem 2rem;
		width: 100%;
	}

	@media (max-width: 768px) {
		.tab-panel {
			grid-template-columns: 1fr;
		}
	}

	.tab-panel .form-group {
		margin-bottom: 0;
		min-width: 0; /* Prevent overflow */
	}

	@keyframes fadeIn {
		from {
			opacity: 0;
			transform: translateY(-10px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	@media (max-width: 640px) {
		.form-row {
			grid-template-columns: 1fr;
		}
	}
</style>