<script lang="ts">
	import type { ComponentType } from 'svelte';

	export let title = '';
	export let description = '';
	export let icon: ComponentType | null = null;
	export let collapsible = false;
	export let collapsed = false;
	export let noPadding = false;
	export let variant: 'default' | 'flat' | 'bordered' = 'default';
</script>

<div class="section {variant}" class:no-padding={noPadding}>
	{#if title || $$slots.header || $$slots.actions}
		<div class="section-header" class:collapsible on:click={() => collapsible && (collapsed = !collapsed)} on:keypress={(e) => e.key === 'Enter' && collapsible && (collapsed = !collapsed)} role={collapsible ? 'button' : undefined} tabindex={collapsible ? 0 : undefined}>
			<div class="section-title-area">
				{#if icon}
					<div class="section-icon">
						<svelte:component this={icon} size={20} />
					</div>
				{/if}
				<div class="section-title-content">
					{#if $$slots.header}
						<slot name="header" />
					{:else if title}
						<h3 class="section-title">{title}</h3>
						{#if description}
							<p class="section-description">{description}</p>
						{/if}
					{/if}
				</div>
			</div>
			{#if $$slots.actions || collapsible}
				<div class="section-actions">
					<slot name="actions" />
					{#if collapsible}
						<button class="collapse-btn" class:collapsed aria-label={collapsed ? 'Expand' : 'Collapse'}>
							<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
								<polyline points="6 9 12 15 18 9"></polyline>
							</svg>
						</button>
					{/if}
				</div>
			{/if}
		</div>
	{/if}

	{#if !collapsed}
		<div class="section-content">
			<slot />
		</div>
	{/if}

	{#if $$slots.footer && !collapsed}
		<div class="section-footer">
			<slot name="footer" />
		</div>
	{/if}
</div>

<style>
	.section {
		background: white;
		border-radius: 0.5rem;
		margin-bottom: 1.5rem;
	}

	.section:last-child {
		margin-bottom: 0;
	}

	.section.default {
		border: 1px solid #e5e7eb;
	}

	.section.bordered {
		border: 1px solid #e5e7eb;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
	}

	.section.flat {
		background: transparent;
		border: none;
	}

	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		padding: 1.25rem 1.5rem;
		gap: 1rem;
	}

	.section.flat .section-header {
		padding: 0 0 1rem 0;
	}

	.section-header.collapsible {
		cursor: pointer;
		user-select: none;
	}

	.section-header.collapsible:hover {
		background: #f9fafb;
	}

	.section.flat .section-header.collapsible:hover {
		background: transparent;
	}

	.section-title-area {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
		flex: 1;
		min-width: 0;
	}

	.section-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 36px;
		height: 36px;
		background: #f3f4f6;
		border-radius: 0.375rem;
		color: #6b7280;
		flex-shrink: 0;
	}

	.section-title-content {
		flex: 1;
		min-width: 0;
	}

	.section-title {
		font-size: 1rem;
		font-weight: 600;
		color: #111827;
		margin: 0;
		line-height: 1.4;
	}

	.section-description {
		font-size: 0.875rem;
		color: #6b7280;
		margin: 0.25rem 0 0 0;
		line-height: 1.4;
	}

	.section-actions {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex-shrink: 0;
	}

	.collapse-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border: none;
		background: transparent;
		color: #6b7280;
		cursor: pointer;
		border-radius: 0.25rem;
		transition: all 0.15s;
	}

	.collapse-btn:hover {
		background: #e5e7eb;
		color: #374151;
	}

	.collapse-btn svg {
		transition: transform 0.2s;
	}

	.collapse-btn.collapsed svg {
		transform: rotate(-90deg);
	}

	.section-content {
		padding: 0 1.5rem 1.5rem;
	}

	.section.flat .section-content {
		padding: 0;
	}

	.section.no-padding .section-content {
		padding: 0;
	}

	.section-header + .section-content {
		padding-top: 0;
	}

	.section:not(:has(.section-header)) .section-content {
		padding-top: 1.5rem;
	}

	.section.flat:not(:has(.section-header)) .section-content {
		padding-top: 0;
	}

	.section-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1rem 1.5rem;
		background: #f9fafb;
		border-top: 1px solid #e5e7eb;
		border-radius: 0 0 0.5rem 0.5rem;
	}

	.section.flat .section-footer {
		background: transparent;
		border-top: 1px solid #e5e7eb;
		border-radius: 0;
		padding: 1rem 0 0 0;
	}

	@media (max-width: 640px) {
		.section-header {
			flex-direction: column;
			align-items: flex-start;
		}

		.section-actions {
			width: 100%;
			justify-content: flex-end;
		}
	}
</style>
