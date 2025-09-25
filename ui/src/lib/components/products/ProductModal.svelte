<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { X } from 'lucide-svelte';
	import type { SvelteComponent } from 'svelte';
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
	$: currentTemplateId = currentFormData.product_template_id || product?.product_template_id;

	// Find the template object to get its name
	$: currentTemplate = currentTemplateId ? productTemplates.find((t: any) => t.id === currentTemplateId) : null;

	// Debug logging
	$: {
		console.log('ProductModal Debug:', {
			currentTemplateId,
			currentTemplate,
			templateName: currentTemplate?.name,
			registry: $productPreviewRegistry,
			registryKeys: Object.keys($productPreviewRegistry)
		});
	}

	// Get preview URL from registry based on template name (not numeric ID)
	$: previewUrl = currentTemplate?.name ? $productPreviewRegistry[currentTemplate.name] : null;

	// Use provided preview component or URL from registry
	$: hasPreview = previewComponent || previewUrl;

	$: {
		console.log('Preview status:', {
			previewUrl,
			hasPreview,
			showPreview,
			willShowPanel: hasPreview && showPreview
		});
	}

	let iframeElement: HTMLIFrameElement;

	// Send updates to iframe when form data changes
	$: if (iframeElement && previewUrl && currentFormData) {
		const message = {
			type: 'updatePreview',
			customFields: currentFormData.custom_fields || {},
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
					custom_fields: {
						...currentFormData.custom_fields,
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

{#if show}
	<div class="modal-overlay" on:click={handleClose} on:keydown={(e) => e.key === 'Escape' && handleClose()}>
		<div class="modal-content" class:with-preview={hasPreview && showPreview} on:click|stopPropagation role="dialog" aria-modal="true">
			<div class="modal-header">
				<h2>{title}</h2>
				<button class="close-btn" on:click={handleClose} aria-label="Close">
					<X size={20} />
				</button>
			</div>

			<div class="modal-body">
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
								customFields={currentFormData.custom_fields || {}}
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
		</div>
	</div>
{/if}

<style>
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
		padding: 1rem;
	}

	.modal-content {
		background: white;
		border-radius: 0.5rem;
		width: 100%;
		max-width: 800px;
		max-height: 90vh;
		display: flex;
		flex-direction: column;
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
		overflow: hidden;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-bottom: 1px solid #e5e7eb;
		flex-shrink: 0;
	}

	.modal-header h2 {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 600;
		color: #111827;
	}

	.close-btn {
		background: none;
		border: none;
		color: #6b7280;
		cursor: pointer;
		padding: 0.25rem;
		border-radius: 0.25rem;
		transition: all 0.2s;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.close-btn:hover {
		color: #111827;
		background: #f3f4f6;
	}

	.modal-body {
		display: flex;
		flex: 1;
		overflow: hidden;
		min-height: 0;
	}

	.form-panel {
		flex: 1;
		overflow-y: auto;
		padding: 1.5rem;
		border-right: 1px solid #e5e7eb;
	}

	.preview-panel {
		flex: 1;
		overflow-y: auto;
		padding: 1.5rem;
		background: #f9fafb;
	}

	.modal-content.with-preview {
		max-width: 1400px;
	}

	.preview-iframe {
		width: 100%;
		height: 100%;
		border: none;
	}

	@media (max-width: 1024px) {
		.modal-body {
			flex-direction: column;
		}

		.form-panel {
			border-right: none;
			border-bottom: 1px solid #e5e7eb;
		}
	}

	@media (max-width: 640px) {
		.modal-content {
			max-height: 100vh;
			border-radius: 0;
		}

		.modal-overlay {
			padding: 0;
		}
	}
</style>