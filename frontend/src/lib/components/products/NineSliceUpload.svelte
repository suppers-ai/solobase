<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let fieldId: string;
	export let value: any = {};
	export let onUpdate: (value: any) => void;

	const dispatch = createEventDispatcher();

	const positions = [
		{ key: 'top_left', label: 'Top Left' },
		{ key: 'top_middle', label: 'Top' },
		{ key: 'top_right', label: 'Top Right' },
		{ key: 'middle_left', label: 'Left' },
		{ key: 'background', label: 'Center' },
		{ key: 'middle_right', label: 'Right' },
		{ key: 'bottom_left', label: 'Bottom Left' },
		{ key: 'bottom_middle', label: 'Bottom' },
		{ key: 'bottom_right', label: 'Bottom Right' }
	];

	function handleFileUpload(e: Event, position: { key: string; label: string }) {
		const target = e.currentTarget as HTMLInputElement;
		const file = target.files?.[0];
		if (!file) return;

		const uploadedId = `pending_${fieldId}_${position.key}_${Date.now()}`;
		const newValue = { ...value };
		newValue[position.key] = uploadedId;
		onUpdate(newValue);

		dispatch('fileUpload', {
			field: `${fieldId}_${position.key}`,
			file,
			customField: true,
			nineSlice: true,
			position: position.key
		});
	}

	function removeFile(positionKey: string) {
		const newValue = { ...value };
		delete newValue[positionKey];
		onUpdate(newValue);
	}
</script>

<div class="nine-slice-upload-wrapper">
	<div class="nine-slice-grid">
		{#each positions as position}
			<div class="nine-slice-cell">
				<label class="cell-label">{position.label}</label>
				{#if value[position.key]}
					<div class="cell-uploaded">
						<span class="cell-file-id" title={value[position.key]}>✓</span>
						<button
							type="button"
							class="cell-remove"
							on:click={() => removeFile(position.key)}
						>×</button>
					</div>
				{:else}
					<input
						type="file"
						accept="image/*"
						class="cell-input"
						on:change={(e) => handleFileUpload(e, position)}
					/>
				{/if}
			</div>
		{/each}
	</div>
</div>

<style>
	.nine-slice-upload-wrapper {
		padding: 1rem;
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
	}

	.nine-slice-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 0.75rem;
		max-width: 480px;
		margin: 0 auto;
	}

	.nine-slice-cell {
		aspect-ratio: 1;
		background: white;
		border: 2px dashed #d1d5db;
		border-radius: 0.375rem;
		padding: 0.5rem;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		position: relative;
		transition: all 0.2s;
	}

	.nine-slice-cell:hover {
		border-color: #9ca3af;
		background: #f3f4f6;
	}

	.cell-label {
		font-size: 0.75rem;
		color: #6b7280;
		margin-bottom: 0.25rem;
		text-align: center;
	}

	.cell-input {
		position: absolute;
		width: 100%;
		height: 100%;
		opacity: 0;
		cursor: pointer;
		top: 0;
		left: 0;
	}

	.cell-uploaded {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.25rem;
	}

	.cell-file-id {
		width: 32px;
		height: 32px;
		background: #10b981;
		color: white;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: bold;
		font-size: 1.25rem;
	}

	.cell-remove {
		position: absolute;
		top: 4px;
		right: 4px;
		width: 20px;
		height: 20px;
		background: #ef4444;
		color: white;
		border: none;
		border-radius: 50%;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 1rem;
		line-height: 1;
		padding: 0;
	}

	.cell-remove:hover {
		background: #dc2626;
	}
</style>