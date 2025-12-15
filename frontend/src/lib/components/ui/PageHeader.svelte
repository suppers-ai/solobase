<script lang="ts">
	import type { ComponentType } from 'svelte';
	import { ArrowLeft } from 'lucide-svelte';

	export let title: string;
	export let subtitle: string = '';
	export let icon: ComponentType | null = null;
	export let backHref: string = '';
	export let variant: 'default' | 'card' = 'default';
</script>

<div class="page-header {variant}">
	{#if backHref}
		<a href={backHref} class="back-button">
			<ArrowLeft size={20} />
		</a>
	{/if}
	<div class="header-content" class:has-back={backHref}>
		<div class="header-left">
			<div class="header-title">
				{#if icon}
					<svelte:component this={icon} size={24} />
				{/if}
				<h1>{title}</h1>
			</div>
			{#if subtitle}
				<p class="header-subtitle">{subtitle}</p>
			{/if}
			{#if $$slots.meta}
				<div class="header-meta">
					<slot name="meta" />
				</div>
			{/if}
		</div>
		{#if $$slots.actions || $$slots.info}
			<div class="header-right">
				{#if $$slots.info}
					<div class="header-info">
						<slot name="info" />
					</div>
				{/if}
				{#if $$slots.actions}
					<div class="header-actions">
						<slot name="actions" />
					</div>
				{/if}
			</div>
		{/if}
	</div>
</div>

<style>
	.page-header {
		position: relative;
	}

	.page-header.default {
		background: white;
		border-bottom: 1px solid #e5e7eb;
		padding: 1.5rem 2rem;
	}

	.page-header.card {
		background: white;
		border-radius: 0.5rem;
		padding: 1.5rem;
		margin-bottom: 1.5rem;
		border: 1px solid #e5e7eb;
	}

	.back-button {
		position: absolute;
		top: 1.5rem;
		left: 1.5rem;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 36px;
		height: 36px;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		background: white;
		color: #6b7280;
		text-decoration: none;
		transition: all 0.15s;
	}

	.back-button:hover {
		background: #f9fafb;
		color: #111827;
		border-color: #d1d5db;
	}

	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 1rem;
	}

	.header-content.has-back {
		margin-left: 48px;
	}

	.header-left {
		flex: 1;
		min-width: 0;
	}

	.header-title {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 0.25rem;
		color: #189AB4;
	}

	.header-title h1 {
		font-size: 1.5rem;
		font-weight: 600;
		color: #111827;
		margin: 0;
	}

	.header-subtitle {
		color: #6b7280;
		font-size: 0.875rem;
		margin: 0;
	}

	.header-meta {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-top: 0.5rem;
		font-size: 0.875rem;
		color: #6b7280;
	}

	.header-meta :global(.meta-item) {
		color: #6b7280;
	}

	.header-meta :global(.meta-item.error) {
		color: #ef4444;
	}

	.header-meta :global(.meta-item.warning) {
		color: #f59e0b;
	}

	.header-meta :global(.meta-item.success) {
		color: #10b981;
	}

	.header-meta :global(.meta-item.info) {
		color: #3b82f6;
	}

	.header-meta :global(.meta-separator) {
		color: #d1d5db;
	}

	.header-right {
		display: flex;
		align-items: center;
		gap: 1rem;
		flex-shrink: 0;
	}

	.header-info {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.header-info :global(.info-item) {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.375rem 0.75rem;
		font-size: 0.75rem;
		font-weight: 500;
		border-radius: 9999px;
		background: #f3f4f6;
		color: #6b7280;
	}

	.header-info :global(.info-item.success) {
		background: #d1fae5;
		color: #065f46;
	}

	.header-info :global(.info-item.warning) {
		background: #fef3c7;
		color: #92400e;
	}

	.header-info :global(.info-item.error) {
		background: #fee2e2;
		color: #991b1b;
	}

	.header-info :global(.info-item.active) {
		background: #dbeafe;
		color: #1e40af;
	}

	.header-info :global(.info-item.paused) {
		background: #f3f4f6;
		color: #6b7280;
	}

	.header-actions {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	@media (max-width: 768px) {
		.page-header.default {
			padding: 1rem;
		}

		.header-content {
			flex-direction: column;
			align-items: flex-start;
		}

		.header-content.has-back {
			margin-left: 0;
			margin-top: 3rem;
		}

		.header-right {
			width: 100%;
			justify-content: flex-start;
			margin-top: 1rem;
		}

		.header-title h1 {
			font-size: 1.25rem;
		}
	}
</style>
