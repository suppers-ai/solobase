<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { formatHexColor, isValidHexColor } from '$lib/utils/colorUtils';

	export let field: any;
	export let value: any;
	export let fieldId: string;
	export let required = false;
	export let onUpdate: (value: any) => void;
	export let onFileUpload: ((e: Event) => void) | null = null;

	const dispatch = createEventDispatcher();

	const constraints = field.constraints || {};
	const options = field.options || constraints.options || [];
	const placeholder = field.placeholder || constraints.placeholder || '';
	const min = field.min ?? constraints.min;
	const max = field.max ?? constraints.max;
	const step = field.step ?? constraints.step;


	// Format color value if it's a color field
	$: formattedColorValue = field.type === 'color' ? formatHexColor(value || '#000000') : value;

	// Ensure color values are formatted on mount/change
	$: if (field.type === 'color' && value && !isValidHexColor(value)) {
		// Format and update the value immediately if it's invalid
		const formatted = formatHexColor(value);
		if (formatted !== value) {
			onUpdate(formatted);
		}
	}
</script>

{#if field.type === 'enum' || field.type === 'select'}
	<select
		id={fieldId}
		{value}
		on:change={(e) => onUpdate(e.currentTarget.value)}
		{required}
	>
		<option value="">Select {field.name}</option>
		{#each options as option}
			<option value={option}>{option}</option>
		{/each}
	</select>
{:else if field.type === 'boolean'}
	<select
		id={fieldId}
		value={value === true ? 'true' : value === false ? 'false' : ''}
		on:change={(e) => onUpdate(e.currentTarget.value === 'true')}
		{required}
	>
		<option value="">Select</option>
		<option value="true">Yes</option>
		<option value="false">No</option>
	</select>
{:else if field.type === 'numeric' || field.type === 'number'}
	<input
		id={fieldId}
		type="number"
		{value}
		on:input={(e) => onUpdate(parseFloat(e.currentTarget.value) || 0)}
		{min}
		{max}
		step={step || 'any'}
		{placeholder}
		{required}
	/>
{:else if field.type === 'color'}
	<div class="color-input-wrapper">
		<input
			id={fieldId}
			type="color"
			value={formattedColorValue}
			on:input={(e) => {
				const formatted = formatHexColor(e.currentTarget.value);
				onUpdate(formatted);
			}}
			on:invalid={(e) => {
				// Prevent default validation message
				e.preventDefault();
				e.currentTarget.setCustomValidity('');
			}}
			required={false}
			class="color-picker"
		/>
		<input
			type="text"
			value={formattedColorValue}
			on:input={(e) => {
				// Clear any custom validity
				e.currentTarget.setCustomValidity('');
				const formatted = formatHexColor(e.currentTarget.value);
				onUpdate(formatted);
			}}
			on:blur={(e) => {
				// On blur, ensure the value is properly formatted
				e.currentTarget.setCustomValidity('');
				const formatted = formatHexColor(e.currentTarget.value);
				onUpdate(formatted);
				e.currentTarget.value = formatted;
			}}
			on:invalid={(e) => {
				// Prevent default validation message
				e.preventDefault();
				e.currentTarget.setCustomValidity('');
			}}
			placeholder="#000000"
			required={false}
			class="color-text"
		/>
	</div>
{:else if field.type === 'url'}
	<input
		id={fieldId}
		type="url"
		{value}
		on:input={(e) => onUpdate(e.currentTarget.value)}
		placeholder={placeholder || 'https://example.com'}
		pattern="https?://.*"
		{required}
		class="url-input"
	/>
{:else if field.type === 'range'}
	<div class="range-input-wrapper">
		<input
			id={fieldId}
			type="range"
			{value}
			on:input={(e) => onUpdate(parseFloat(e.currentTarget.value))}
			{min}
			{max}
			step={step || 1}
			{required}
			class="range-slider"
		/>
		<span class="range-value">{value}</span>
	</div>
{:else if field.type === 'upload'}
	<div class="upload-input-wrapper">
		{#if value}
			<div class="uploaded-file">
				<span class="file-name">File ID: {value}</span>
				<button type="button" on:click={() => onUpdate(null)} class="btn-remove">Remove</button>
			</div>
		{/if}
		<input
			id={fieldId}
			type="file"
			on:change={onFileUpload}
			accept={field.accept || 'image/*'}
			required={required && !value}
			class="file-input"
		/>
	</div>
{:else}
	<input
		id={fieldId}
		type="text"
		{value}
		on:input={(e) => onUpdate(e.currentTarget.value)}
		{placeholder}
		maxlength={constraints.max_length}
		{required}
	/>
{/if}

<style>
	input[type="text"],
	input[type="number"],
	input[type="url"],
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
	input[type="url"]:focus,
	select:focus {
		outline: none;
		border-color: #3b82f6;
		box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
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

	.url-input {
		font-family: 'Courier New', monospace;
		font-size: 0.875rem;
	}

	.range-input-wrapper {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.range-slider {
		flex: 1;
		height: 6px;
		-webkit-appearance: none;
		appearance: none;
		background: #e5e7eb;
		border-radius: 3px;
		outline: none;
	}

	.range-slider::-webkit-slider-thumb {
		-webkit-appearance: none;
		appearance: none;
		width: 20px;
		height: 20px;
		background: #3b82f6;
		cursor: pointer;
		border-radius: 50%;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
	}

	.range-slider::-moz-range-thumb {
		width: 20px;
		height: 20px;
		background: #3b82f6;
		cursor: pointer;
		border-radius: 50%;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
		border: none;
	}

	.range-value {
		min-width: 40px;
		text-align: center;
		font-weight: 500;
		color: #374151;
	}

	.upload-input-wrapper {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.uploaded-file {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.5rem;
		background: #f3f4f6;
		border: 1px solid #d1d5db;
		border-radius: 0.375rem;
	}

	.file-name {
		font-size: 0.875rem;
		color: #374151;
	}

	.btn-remove {
		padding: 0.25rem 0.5rem;
		background: #ef4444;
		color: white;
		border: none;
		border-radius: 0.25rem;
		font-size: 0.75rem;
		cursor: pointer;
	}

	.btn-remove:hover {
		background: #dc2626;
	}

	.file-input {
		padding: 0.5rem;
		border: 2px dashed #d1d5db;
		border-radius: 0.375rem;
		background: #f9fafb;
		cursor: pointer;
	}

	.file-input:hover {
		border-color: #9ca3af;
		background: #f3f4f6;
	}
</style>