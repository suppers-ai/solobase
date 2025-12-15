<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { AlertTriangle, Info, AlertCircle, CheckCircle } from 'lucide-svelte';
	import Modal from './Modal.svelte';

	export let show = false;
	export let title: string;
	export let message: string;
	export let confirmText: string = 'Confirm';
	export let cancelText: string = 'Cancel';
	export let variant: 'danger' | 'warning' | 'info' | 'success' = 'danger';
	export let loading: boolean = false;

	const dispatch = createEventDispatcher();

	const icons = {
		danger: AlertCircle,
		warning: AlertTriangle,
		info: Info,
		success: CheckCircle
	};

	const colors = {
		danger: { bg: '#fef2f2', border: '#fee2e2', icon: '#ef4444', text: '#991b1b' },
		warning: { bg: '#fffbeb', border: '#fef3c7', icon: '#f59e0b', text: '#92400e' },
		info: { bg: '#eff6ff', border: '#dbeafe', icon: '#3b82f6', text: '#1e40af' },
		success: { bg: '#f0fdf4', border: '#dcfce7', icon: '#22c55e', text: '#166534' }
	};

	$: Icon = icons[variant];
	$: colorScheme = colors[variant];

	function handleConfirm() {
		dispatch('confirm');
	}

	function handleClose() {
		if (!loading) {
			show = false;
			dispatch('close');
		}
	}
</script>

<Modal {show} {title} maxWidth="450px" on:close={handleClose} closeOnOverlay={!loading}>
	<div class="alert-box" style="background: {colorScheme.bg}; border-color: {colorScheme.border};">
		<div class="alert-icon" style="color: {colorScheme.icon};">
			<svelte:component this={Icon} size={24} />
		</div>
		<p class="alert-message" style="color: {colorScheme.text};">{message}</p>
	</div>

	<slot />

	<svelte:fragment slot="footer">
		<button class="modal-btn modal-btn-secondary" on:click={handleClose} disabled={loading}>
			{cancelText}
		</button>
		<button
			class="modal-btn"
			class:modal-btn-danger={variant === 'danger'}
			class:modal-btn-warning={variant === 'warning'}
			class:modal-btn-primary={variant === 'info' || variant === 'success'}
			on:click={handleConfirm}
			disabled={loading}
		>
			{loading ? 'Processing...' : confirmText}
		</button>
	</svelte:fragment>
</Modal>

<style>
	.alert-box {
		display: flex;
		gap: 1rem;
		padding: 1rem;
		border: 1px solid;
		border-radius: 0.375rem;
		margin-bottom: 1rem;
	}

	.alert-icon {
		flex-shrink: 0;
	}

	.alert-message {
		margin: 0;
		font-size: 0.875rem;
		line-height: 1.5;
	}
</style>
