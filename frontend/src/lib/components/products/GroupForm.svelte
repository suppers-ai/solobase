<script context="module" lang="ts">
	export interface GroupFormProps {
		mode?: 'create' | 'edit';
		group?: any;
		submitButtonText?: string;
		onSubmit?: (data: any) => void;
		onCancel?: () => void;
	}
</script>

<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let mode: 'create' | 'edit' = 'create';
	export let group: any = null;
	export let submitButtonText = mode === 'create' ? 'Create Group' : 'Save Changes';
	export let onSubmit: ((data: any) => void) | undefined = undefined;
	export let onCancel: (() => void) | undefined = undefined;

	const dispatch = createEventDispatcher();

	let formData: any = {
		name: '',
		description: '',
		settings: {},
		metadata: {}
	};

	// Initialize form data from existing group
	function initializeFromGroup() {
		if (mode === 'edit' && group) {
			formData = {
				...group,
				settings: group.settings || {},
				metadata: group.metadata || {}
			};
		}
	}

	// React to group changes
	$: if (mode === 'edit' && group) {
		initializeFromGroup();
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
	}

	function handleSubmit() {
		const submitData = { ...formData };

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

		if (!formData.name || formData.name.trim() === '') {
			errors.push('Group name is required');
		}

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
			<h3>Group Information</h3>

			<div class="form-group">
				<label for="name">Name <span class="required">*</span></label>
				<input
					id="name"
					type="text"
					value={formData.name}
					on:input={(e) => updateField('name', e.currentTarget.value)}
					placeholder="Group name"
					required
				/>
			</div>

			<div class="form-group">
				<label for="description">Description</label>
				<textarea
					id="description"
					value={formData.description}
					on:input={(e) => updateField('description', e.currentTarget.value)}
					rows="4"
					placeholder="Group description"
				></textarea>
			</div>
		</div>

		<!-- Settings Section -->
		<div class="form-section">
			<h3>Settings</h3>

			<div class="form-group">
				<label for="max_products">Maximum Products</label>
				<input
					id="max_products"
					type="number"
					value={formData.settings.max_products || ''}
					on:input={(e) => updateField('settings.max_products', e.currentTarget.value ? parseInt(e.currentTarget.value) : null)}
					placeholder="No limit"
					min="0"
				/>
				<p class="field-description">Leave empty for unlimited products</p>
			</div>

			<div class="form-group">
				<label for="default_currency">Default Currency</label>
				<select
					id="default_currency"
					value={formData.settings.default_currency || 'USD'}
					on:change={(e) => updateField('settings.default_currency', e.currentTarget.value)}
				>
					<option value="USD">USD - US Dollar</option>
					<option value="EUR">EUR - Euro</option>
					<option value="GBP">GBP - British Pound</option>
					<option value="JPY">JPY - Japanese Yen</option>
					<option value="AUD">AUD - Australian Dollar</option>
					<option value="CAD">CAD - Canadian Dollar</option>
					<option value="CHF">CHF - Swiss Franc</option>
					<option value="CNY">CNY - Chinese Yuan</option>
				</select>
			</div>

			<div class="form-group">
				<label for="visibility">Visibility</label>
				<select
					id="visibility"
					value={formData.settings.visibility || 'private'}
					on:change={(e) => updateField('settings.visibility', e.currentTarget.value)}
				>
					<option value="private">Private - Only visible to you</option>
					<option value="team">Team - Visible to team members</option>
					<option value="public">Public - Visible to everyone</option>
				</select>
			</div>
		</div>

		<!-- Metadata Section (optional) -->
		<div class="form-section">
			<h3>Additional Information</h3>

			<div class="form-group">
				<label for="tags">Tags</label>
				<input
					id="tags"
					type="text"
					value={formData.metadata.tags || ''}
					on:input={(e) => updateField('metadata.tags', e.currentTarget.value)}
					placeholder="Enter tags separated by commas"
				/>
				<p class="field-description">Use tags to organize and categorize your groups</p>
			</div>

			<div class="form-group">
				<label for="notes">Internal Notes</label>
				<textarea
					id="notes"
					value={formData.metadata.notes || ''}
					on:input={(e) => updateField('metadata.notes', e.currentTarget.value)}
					rows="3"
					placeholder="Internal notes about this group"
				></textarea>
			</div>
		</div>
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
		.form-content {
			max-height: 100vh;
		}
	}
</style>