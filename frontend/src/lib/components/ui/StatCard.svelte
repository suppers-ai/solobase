<script lang="ts">
	import type { ComponentType } from 'svelte';

	export let icon: ComponentType;
	export let value: string | number;
	export let label: string;
	export let sublabel: string = '';
	export let change: string = '';
	export let changeType: 'positive' | 'negative' | 'neutral' = 'neutral';
	export let iconBg: string = 'bg-gray-100';
	export let iconColor: string = 'text-gray-600';
	export let size: 'sm' | 'md' | 'lg' = 'md';
</script>

<div class="stat-card {size}">
	<div class="stat-icon {iconBg}">
		<svelte:component this={icon} size={size === 'sm' ? 16 : size === 'lg' ? 24 : 20} class={iconColor} />
	</div>
	<div class="stat-content">
		<p class="stat-label">{label}</p>
		<p class="stat-value">{value}</p>
		{#if sublabel}
			<p class="stat-sublabel">{sublabel}</p>
		{/if}
		{#if change}
			<p class="stat-change {changeType}">
				{change}
			</p>
		{/if}
	</div>
</div>

<style>
	.stat-card {
		display: flex;
		align-items: flex-start;
		gap: 1rem;
		padding: 1rem;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
	}

	.stat-card.sm {
		padding: 0.75rem;
		gap: 0.75rem;
	}

	.stat-card.lg {
		padding: 1.25rem;
		gap: 1.25rem;
	}

	.stat-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 40px;
		height: 40px;
		border-radius: 0.5rem;
		flex-shrink: 0;
	}

	.stat-card.sm .stat-icon {
		width: 32px;
		height: 32px;
	}

	.stat-card.lg .stat-icon {
		width: 48px;
		height: 48px;
	}

	/* Icon background colors */
	.stat-icon.bg-gray-100 { background: #f3f4f6; }
	.stat-icon.bg-blue-100 { background: #dbeafe; }
	.stat-icon.bg-green-100 { background: #dcfce7; }
	.stat-icon.bg-red-100 { background: #fee2e2; }
	.stat-icon.bg-yellow-100 { background: #fef3c7; }
	.stat-icon.bg-purple-100 { background: #f3e8ff; }
	.stat-icon.bg-pink-100 { background: #fce7f3; }
	.stat-icon.bg-cyan-100 { background: #cffafe; }
	.stat-icon.bg-orange-100 { background: #ffedd5; }

	/* Icon text colors */
	.stat-icon :global(.text-gray-600) { color: #4b5563; }
	.stat-icon :global(.text-blue-600) { color: #2563eb; }
	.stat-icon :global(.text-green-600) { color: #16a34a; }
	.stat-icon :global(.text-red-600) { color: #dc2626; }
	.stat-icon :global(.text-yellow-600) { color: #ca8a04; }
	.stat-icon :global(.text-purple-600) { color: #9333ea; }
	.stat-icon :global(.text-pink-600) { color: #db2777; }
	.stat-icon :global(.text-cyan-600) { color: #0891b2; }
	.stat-icon :global(.text-orange-600) { color: #ea580c; }

	.stat-content {
		flex: 1;
		min-width: 0;
	}

	.stat-label {
		font-size: 0.75rem;
		font-weight: 500;
		color: #6b7280;
		margin: 0;
		text-transform: uppercase;
		letter-spacing: 0.025em;
	}

	.stat-card.lg .stat-label {
		font-size: 0.875rem;
	}

	.stat-value {
		font-size: 1.5rem;
		font-weight: 600;
		color: #111827;
		margin: 0.25rem 0 0 0;
		line-height: 1.2;
	}

	.stat-card.sm .stat-value {
		font-size: 1.25rem;
	}

	.stat-card.lg .stat-value {
		font-size: 1.75rem;
	}

	.stat-sublabel {
		font-size: 0.75rem;
		color: #9ca3af;
		margin: 0.25rem 0 0 0;
	}

	.stat-change {
		font-size: 0.75rem;
		font-weight: 500;
		margin: 0.375rem 0 0 0;
	}

	.stat-change.positive {
		color: #16a34a;
	}

	.stat-change.negative {
		color: #dc2626;
	}

	.stat-change.neutral {
		color: #6b7280;
	}
</style>
