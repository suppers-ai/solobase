<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { SvelteComponent } from 'svelte';
	import Modal from '$lib/components/ui/Modal.svelte';
	import ProductForm from './ProductForm.svelte';
	import { productPreviewRegistry } from '$lib/stores/productPreviewRegistry';

	export let show = false;
	export let mode: 'create' | 'edit' = 'create';
	export let product: any = null;
	export let productTemplates: any[] = [];
	export let groups: any[] = [];
	export let initialGroupId: number | string | null = null;
	export let title = mode === 'create' ? 'Create Product' : 'Edit Product';
	export let submitButtonText = mode === 'create' ? 'Create' : 'Save';
	export let customFieldsConfig: any = null;
	export let previewComponent: typeof SvelteComponent | null = null;
	export let showPreview: boolean = true;

	const dispatch = createEventDispatcher();

	// Track current form data for preview
	let currentFormData: any = {};

	// Get the current selected template ID from form data
	$: currentTemplateId = currentFormData.productTemplateId || product?.productTemplateId;

	// Find the template object to get its name
	$: currentTemplate = currentTemplateId ? productTemplates.find((t: any) => t.id === currentTemplateId) : null;

	// Get preview URL from registry based on template name (not numeric ID)
	$: previewUrl = currentTemplate?.name ? $productPreviewRegistry[currentTemplate.name] : null;

	// Use provided preview component or URL from registry
	$: hasPreview = previewComponent || previewUrl;

	let iframeElement: HTMLIFrameElement;

	// Send updates to iframe when form data changes
	$: if (iframeElement && previewUrl && currentFormData) {
		const message = {
			type: 'updatePreview',
			customFields: currentFormData.customFields || {},
			formData: currentFormData
		};
		iframeElement.contentWindow?.postMessage(message, '*');
	}

	function handleClose() {
		dispatch('close');
		show = false;
	}

	function handleFormSubmit(event: CustomEvent) {
		dispatch('submit', event.detail);
		handleClose();
	}

	function handleFormCancel() {
		handleClose();
	}

	function handleFieldChange(event: CustomEvent) {
		// Update current form data for preview
		const { fieldId, value } = event.detail;
		if (fieldId) {
			if (fieldId.startsWith('custom_')) {
				const customFieldId = fieldId.replace('custom_', '');
				currentFormData = {
					...currentFormData,
					customFields: {
						...currentFormData.customFields,
						[customFieldId]: value
					}
				};
			} else {
				currentFormData = {
					...currentFormData,
					[fieldId]: value
				};
			}
		}
		// Forward event to parent
		dispatch('fieldChange', event.detail);
	}
</script>

<Modal {show} {title} maxWidth={hasPreview && showPreview ? '1400px' : '800px'} on:close={handleClose}>
	<div class="modal-body-content" class:with-preview={hasPreview && showPreview}>
		<!-- Form Panel -->
		<div class="form-panel">
			<ProductForm
				{mode}
				{product}
				{productTemplates}
				{groups}
				{initialGroupId}
				{customFieldsConfig}
				{submitButtonText}
				on:submit={handleFormSubmit}
				on:cancel={handleFormCancel}
				on:fieldChange={handleFieldChange}
			/>
		</div>

		<!-- Preview Panel -->
		{#if showPreview}
			{#if previewComponent}
				<div class="preview-panel">
					<svelte:component
						this={previewComponent}
						customFields={currentFormData.customFields || {}}
						formData={currentFormData}
					/>
				</div>
			{:else if previewUrl}
				<div class="preview-panel">
					<iframe
						bind:this={iframeElement}
						src={previewUrl}
						title="Preview"
						class="preview-iframe"
					/>
				</div>
			{/if}
		{/if}
	</div>
</Modal>

<style>
	.modal-body-content {
		display: flex;
		gap: 1.5rem;
	}

	.form-panel {
		flex: 1;
		min-width: 0;
	}

	.preview-panel {
		flex: 1;
		min-width: 0;
		background: #f9fafb;
		border-radius: 0.375rem;
		padding: 1rem;
	}

	.modal-body-content.with-preview {
		min-height: 500px;
	}

	.preview-iframe {
		width: 100%;
		height: 100%;
		min-height: 400px;
		border: none;
	}

	@media (max-width: 1024px) {
		.modal-body-content {
			flex-direction: column;
		}

		.preview-panel {
			min-height: 300px;
		}
	}
</style>
