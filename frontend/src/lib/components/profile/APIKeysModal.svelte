<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { Key, Plus, Copy, Trash2 } from 'lucide-svelte';
	import Modal from '$lib/components/ui/Modal.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import LoadingSpinner from '$lib/components/ui/LoadingSpinner.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';

	export let show = false;
	export let apiKeys: any[] = [];
	export let loading = false;
	export let error = '';
	export let createdKey: string | null = null;

	const dispatch = createEventDispatcher();

	function handleClose() {
		dispatch('close');
	}

	function handleCreateKey() {
		dispatch('createKey');
	}

	function handleRevokeKey(keyId: string, keyName: string) {
		dispatch('revokeKey', { id: keyId, name: keyName });
	}

	async function copyToClipboard(text: string) {
		try {
			await navigator.clipboard.writeText(text);
		} catch (err) {
			console.error('Failed to copy to clipboard:', err);
		}
	}

	function formatDate(dateString: string | null): string {
		if (!dateString) return 'Never';
		return new Date(dateString).toLocaleDateString(undefined, {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<Modal {show} title="API Keys" maxWidth="600px" on:close={handleClose}>
	{#if error}
		<div class="alert alert-error">{error}</div>
	{/if}

	<!-- Newly created key alert -->
	{#if createdKey}
		<div class="created-key-alert">
			<div class="alert-header">
				<Key size={16} />
				<strong>API Key Created!</strong>
			</div>
			<p class="alert-warning">Copy this key now. You won't be able to see it again!</p>
			<div class="key-display">
				<code>{createdKey}</code>
				<button
					class="copy-btn"
					on:click={() => copyToClipboard(createdKey || '')}
					title="Copy to clipboard"
				>
					<Copy size={16} />
				</button>
			</div>
		</div>
	{/if}

	<!-- API Keys List -->
	<div class="api-keys-section">
		<div class="section-header">
			<span>Your API Keys</span>
			<Button size="sm" icon={Plus} on:click={handleCreateKey}>
				Create Key
			</Button>
		</div>

		{#if loading}
			<div class="loading-state">
				<LoadingSpinner size="sm" />
				<span>Loading API keys...</span>
			</div>
		{:else if !apiKeys || apiKeys.length === 0}
			<EmptyState icon={Key} title="No API keys yet" message="Create an API key to access the API programmatically" compact />
		{:else}
			<div class="api-keys-list">
				{#each apiKeys as key}
					<div class="api-key-item">
						<div class="key-info">
							<div class="key-name">{key.name}</div>
							<div class="key-details">
								<span class="key-prefix" title="Key prefix">{key.keyPrefix}...</span>
								<span class="key-separator">-</span>
								<span class="key-created">Created {formatDate(key.createdAt)}</span>
							</div>
							{#if key.lastUsedAt}
								<div class="key-last-used">
									Last used: {formatDate(key.lastUsedAt)}
									{#if key.lastUsedIp}
										from {key.lastUsedIp}
									{/if}
								</div>
							{:else}
								<div class="key-last-used">Never used</div>
							{/if}
							{#if key.expiresAt}
								<div class="key-expiry" class:expired={new Date(key.expiresAt) < new Date()}>
									{#if new Date(key.expiresAt) < new Date()}
										Expired {formatDate(key.expiresAt)}
									{:else}
										Expires {formatDate(key.expiresAt)}
									{/if}
								</div>
							{/if}
						</div>
						<button
							class="revoke-btn"
							on:click={() => handleRevokeKey(key.id, key.name)}
							title="Revoke API key"
						>
							<Trash2 size={16} />
						</button>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	<!-- Usage instructions -->
	<div class="api-usage-info">
		<h4>How to use API Keys</h4>
		<p>Include your API key in the Authorization header:</p>
		<code class="code-block">Authorization: Bearer sb_your_api_key_here</code>
	</div>

	<svelte:fragment slot="footer">
		<Button on:click={handleClose}>Close</Button>
	</svelte:fragment>
</Modal>

<style>
	.alert {
		padding: 0.75rem 1rem;
		border-radius: 0.375rem;
		margin-bottom: 1rem;
		font-size: 0.875rem;
	}

	.alert-error {
		background: #fee2e2;
		color: #991b1b;
		border: 1px solid #fecaca;
	}

	.created-key-alert {
		background: #d1fae5;
		border: 1px solid #6ee7b7;
		border-radius: 0.5rem;
		padding: 1rem;
		margin-bottom: 1.5rem;
	}

	.alert-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: #065f46;
		margin-bottom: 0.5rem;
	}

	.alert-warning {
		font-size: 0.875rem;
		color: #047857;
		margin: 0 0 0.75rem 0;
	}

	.key-display {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		background: white;
		padding: 0.75rem;
		border-radius: 0.375rem;
		border: 1px solid #6ee7b7;
	}

	.key-display code {
		flex: 1;
		font-family: monospace;
		font-size: 0.875rem;
		word-break: break-all;
		color: #065f46;
	}

	.copy-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border: 1px solid #d1d5db;
		border-radius: 0.375rem;
		background: white;
		color: #6b7280;
		cursor: pointer;
		transition: all 0.15s;
		flex-shrink: 0;
	}

	.copy-btn:hover {
		background: #f3f4f6;
		color: #374151;
	}

	.api-keys-section {
		margin-bottom: 1.5rem;
	}

	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
		font-weight: 600;
		color: #374151;
	}

	.loading-state {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 1.5rem;
		color: #6b7280;
		font-size: 0.875rem;
	}

	.api-keys-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.api-key-item {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 1rem;
		padding: 1rem;
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
	}

	.key-info {
		flex: 1;
		min-width: 0;
	}

	.key-name {
		font-weight: 600;
		color: #111827;
		margin-bottom: 0.25rem;
	}

	.key-details {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.75rem;
		color: #6b7280;
		margin-bottom: 0.25rem;
	}

	.key-prefix {
		font-family: monospace;
		background: #e5e7eb;
		padding: 0.125rem 0.375rem;
		border-radius: 0.25rem;
	}

	.key-separator {
		color: #d1d5db;
	}

	.key-last-used {
		font-size: 0.75rem;
		color: #6b7280;
	}

	.key-expiry {
		font-size: 0.75rem;
		color: #d97706;
		margin-top: 0.25rem;
	}

	.key-expiry.expired {
		color: #dc2626;
	}

	.revoke-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border: 1px solid #fecaca;
		border-radius: 0.375rem;
		background: white;
		color: #ef4444;
		cursor: pointer;
		transition: all 0.15s;
		flex-shrink: 0;
	}

	.revoke-btn:hover {
		background: #fee2e2;
		border-color: #f87171;
	}

	.api-usage-info {
		background: #f3f4f6;
		border-radius: 0.5rem;
		padding: 1rem;
	}

	.api-usage-info h4 {
		font-size: 0.875rem;
		font-weight: 600;
		color: #374151;
		margin: 0 0 0.5rem 0;
	}

	.api-usage-info p {
		font-size: 0.875rem;
		color: #6b7280;
		margin: 0 0 0.75rem 0;
	}

	.code-block {
		display: block;
		font-family: monospace;
		font-size: 0.813rem;
		background: white;
		padding: 0.75rem;
		border-radius: 0.375rem;
		border: 1px solid #e5e7eb;
		color: #0891b2;
		word-break: break-all;
	}
</style>
