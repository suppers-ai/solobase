<script lang="ts">
	export let status: string;
	export let variant: 'success' | 'warning' | 'danger' | 'info' | 'default' | 'auto' = 'auto';
	export let size: 'sm' | 'md' = 'md';
	export let pill: boolean = true;

	// Auto-detect variant based on common status names
	function getAutoVariant(status: string): string {
		const s = status.toLowerCase();
		if (['active', 'completed', 'paid', 'success', 'enabled', 'online', 'approved', 'published', 'confirmed'].includes(s)) {
			return 'success';
		}
		if (['pending', 'processing', 'waiting', 'draft', 'building'].includes(s)) {
			return 'warning';
		}
		if (['failed', 'error', 'cancelled', 'inactive', 'disabled', 'offline', 'rejected', 'unconfirmed'].includes(s)) {
			return 'danger';
		}
		if (['info', 'new', 'scheduled'].includes(s)) {
			return 'info';
		}
		return 'default';
	}

	$: effectiveVariant = variant === 'auto' ? getAutoVariant(status) : variant;
</script>

<span class="status-badge {effectiveVariant}" class:pill class:sm={size === 'sm'}>
	{status}
</span>

<style>
	.status-badge {
		display: inline-block;
		padding: 0.25rem 0.75rem;
		font-size: 0.75rem;
		font-weight: 500;
		text-transform: capitalize;
		border-radius: 0.25rem;
	}

	.status-badge.pill {
		border-radius: 9999px;
	}

	.status-badge.sm {
		padding: 0.125rem 0.5rem;
		font-size: 0.6875rem;
	}

	.status-badge.success {
		background: #dcfce7;
		color: #166534;
	}

	.status-badge.warning {
		background: #fef3c7;
		color: #92400e;
	}

	.status-badge.danger {
		background: #fee2e2;
		color: #991b1b;
	}

	.status-badge.info {
		background: #dbeafe;
		color: #1e40af;
	}

	.status-badge.default {
		background: #f3f4f6;
		color: #4b5563;
	}
</style>
