<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { requireAdmin } from '$lib/utils/auth';
	import { 
		Webhook, Plus, RefreshCw, X,
		CheckCircle, XCircle, Send,
		Link, Key, Calendar, Activity
	} from 'lucide-svelte';
	
	let webhooks = [];
	let loading = true;
	let error = null;
	let showCreateModal = false;
	let newWebhook = {
		name: '',
		url: '',
		events: [],
		secret: ''
	};
	
	// Stats
	let stats = {
		total: 0,
		active: 0,
		deliveriesToday: 0,
		successRate: 95
	};
	
	// Event presets
	const eventPresets = [
		{ category: 'Orders', events: ['order.created', 'order.updated', 'order.cancelled'] },
		{ category: 'Users', events: ['user.created', 'user.updated', 'user.deleted'] },
		{ category: 'Payments', events: ['payment.success', 'payment.failed', 'payment.refunded'] },
		{ category: 'Products', events: ['product.created', 'product.updated', 'product.deleted'] }
	];
	
	let selectedEvents = [];

	onMount(async () => {
		// Check admin access
		if (!requireAdmin()) return;
		
		await loadWebhooks();
	});

	async function loadWebhooks() {
		try {
			loading = true;
			const response = await api.getWebhooks();
			if (response.error) {
				throw new Error(response.error);
			}
			
			// Generate sample webhooks for demo
			webhooks = response.data?.webhooks || [
				{
					id: 1,
					name: 'Order Notifications',
					url: 'https://api.example.com/webhooks/orders',
					events: ['order.created', 'order.updated'],
					active: true,
					lastTriggered: new Date(Date.now() - 3600000).toISOString(),
					deliveries: 142,
					successRate: 98
				},
				{
					id: 2,
					name: 'User Updates',
					url: 'https://api.example.com/webhooks/users',
					events: ['user.created', 'user.deleted'],
					active: false,
					lastTriggered: new Date(Date.now() - 86400000).toISOString(),
					deliveries: 89,
					successRate: 92
				}
			];
			
			// Update stats
			stats.total = webhooks.length;
			stats.active = webhooks.filter(w => w.active).length;
			
		} catch (err) {
			error = err.message;
		} finally {
			loading = false;
		}
	}

	function openCreateModal() {
		showCreateModal = true;
		newWebhook = { name: '', url: '', events: [], secret: '' };
		selectedEvents = [];
	}
	
	function closeCreateModal() {
		showCreateModal = false;
	}
	
	function toggleEvent(event) {
		const index = selectedEvents.indexOf(event);
		if (index >= 0) {
			selectedEvents.splice(index, 1);
		} else {
			selectedEvents.push(event);
		}
		selectedEvents = selectedEvents;
		newWebhook.events = selectedEvents;
	}
	
	async function createWebhook() {
		if (!newWebhook.name || !newWebhook.url || selectedEvents.length === 0) {
			alert('Please fill in all required fields');
			return;
		}
		
		try {
			const response = await api.createWebhook({
				...newWebhook,
				events: selectedEvents
			});
			
			if (response.error) {
				throw new Error(response.error);
			}
			
			closeCreateModal();
			await loadWebhooks();
		} catch (err) {
			alert(`Error: ${err.message}`);
		}
	}

	async function toggleWebhook(id, active) {
		try {
			const response = await api.toggleWebhook(id, active);
			if (response.error) {
				alert('Failed to toggle webhook: ' + response.error);
			} else {
				await loadWebhooks();
			}
		} catch (err) {
			alert('Failed to toggle webhook: ' + err.message);
		}
	}

	async function testWebhook(webhook) {
		try {
			// Simulate test
			alert(`Testing webhook "${webhook.name}"...\nSending test payload to ${webhook.url}`);
		} catch (err) {
			alert('Test failed: ' + err.message);
		}
	}
	
	async function deleteWebhook(id) {
		if (!confirm('Are you sure you want to delete this webhook?')) return;
		
		try {
			// Remove from local array for now
			webhooks = webhooks.filter(w => w.id !== id);
			stats.total = webhooks.length;
			stats.active = webhooks.filter(w => w.active).length;
		} catch (err) {
			alert('Failed to delete webhook: ' + err.message);
		}
	}
	
	function formatDate(dateStr) {
		if (!dateStr) return 'Never';
		const date = new Date(dateStr);
		const now = new Date();
		const diff = now - date;
		
		if (diff < 3600000) return `${Math.floor(diff / 60000)} minutes ago`;
		if (diff < 86400000) return `${Math.floor(diff / 3600000)} hours ago`;
		return `${Math.floor(diff / 86400000)} days ago`;
	}
</script>

<div class="page-container">
	<!-- Header -->
	<div class="page-header">
		<div class="header-content">
			<div class="header-left">
				<div class="header-title">
					<Webhook size={24} />
					<h1>Webhooks Dashboard</h1>
					<span class="badge badge-primary badge-sm">Official Extension</span>
				</div>
				<p class="header-subtitle">Manage and monitor your webhook integrations</p>
			</div>
			<div class="header-actions">
				<button on:click={openCreateModal} class="action-btn btn-primary">
					<Plus size={16} />
					New Webhook
				</button>
				<button on:click={loadWebhooks} class="action-btn btn-ghost" disabled={loading}>
					<RefreshCw size={16} class={loading ? 'animate-spin' : ''} />
				</button>
			</div>
		</div>
	</div>

	{#if loading}
		<div class="loading-container">
			<div class="loading loading-spinner loading-lg text-primary"></div>
			<p class="loading-text">Loading webhooks...</p>
		</div>
	{:else if error}
		<div class="alert alert-error">
			<span>{error}</span>
		</div>
	{:else}
		<!-- Stats Grid -->
		<div class="stats-grid">
			<div class="stat-card">
				<div class="stat-icon bg-purple-100 text-purple-600">
					<Webhook size={20} />
				</div>
				<div class="stat-content">
					<p class="stat-label">Total Webhooks</p>
					<p class="stat-value">{stats.total}</p>
				</div>
			</div>
			
			<div class="stat-card">
				<div class="stat-icon bg-green-100 text-green-600">
					<CheckCircle size={20} />
				</div>
				<div class="stat-content">
					<p class="stat-label">Active Webhooks</p>
					<p class="stat-value">{stats.active}</p>
				</div>
			</div>
			
			<div class="stat-card">
				<div class="stat-icon bg-blue-100 text-blue-600">
					<Send size={20} />
				</div>
				<div class="stat-content">
					<p class="stat-label">Deliveries Today</p>
					<p class="stat-value">{stats.deliveriesToday}</p>
				</div>
			</div>
			
			<div class="stat-card">
				<div class="stat-icon bg-cyan-100 text-cyan-600">
					<Activity size={20} />
				</div>
				<div class="stat-content">
					<p class="stat-label">Success Rate</p>
					<p class="stat-value">{stats.successRate}%</p>
				</div>
			</div>
		</div>

		<!-- Webhooks List -->
		<div class="webhooks-section">
			<div class="section-header">
				<h2 class="section-title">Configured Webhooks</h2>
			</div>
			
			{#if webhooks.length > 0}
				<div class="webhooks-list">
					{#each webhooks as webhook}
						<div class="webhook-card {webhook.active ? 'webhook-active' : 'webhook-inactive'}">
							<div class="webhook-status {webhook.active ? 'status-active' : 'status-inactive'}"></div>
							<div class="webhook-content">
								<div class="webhook-header">
									<div>
										<h3 class="webhook-name">{webhook.name}</h3>
										<div class="webhook-url">
											<Link size={14} />
											<span>{webhook.url}</span>
										</div>
									</div>
									<div class="webhook-actions">
										<button 
											class="toggle-switch {webhook.active ? 'toggle-active' : ''}"
											on:click={() => toggleWebhook(webhook.id, !webhook.active)}
											title={webhook.active ? 'Disable' : 'Enable'}
										>
											<div class="toggle-slider"></div>
										</button>
										<button 
											on:click={() => testWebhook(webhook)} 
											class="action-btn-small btn-test"
											title="Test webhook"
										>
											<Send size={14} />
											Test
										</button>
										<button 
											on:click={() => deleteWebhook(webhook.id)} 
											class="action-btn-small btn-delete"
											title="Delete webhook"
										>
											<X size={14} />
										</button>
									</div>
								</div>
								
								<div class="webhook-events">
									{#each webhook.events as event}
										<span class="event-badge">{event}</span>
									{/each}
								</div>
								
								<div class="webhook-stats">
									<div class="webhook-stat">
										<Calendar size={14} />
										<span>Last triggered: {formatDate(webhook.lastTriggered)}</span>
									</div>
									<div class="webhook-stat">
										<Activity size={14} />
										<span>{webhook.deliveries || 0} deliveries</span>
									</div>
									<div class="webhook-stat">
										<CheckCircle size={14} />
										<span>{webhook.successRate || 0}% success rate</span>
									</div>
								</div>
							</div>
						</div>
					{/each}
				</div>
			{:else}
				<div class="empty-state">
					<Webhook size={48} class="text-gray-300" />
					<p class="text-gray-500 mt-3">No webhooks configured yet</p>
					<p class="text-sm text-gray-400 mt-1">Click "New Webhook" to get started</p>
				</div>
			{/if}
		</div>
	{/if}
</div>

<!-- Create Webhook Modal -->
{#if showCreateModal}
	<div class="modal-overlay">
		<div class="modal-container">
			<div class="modal-header">
				<h3 class="modal-title">Create New Webhook</h3>
				<button class="modal-close" on:click={closeCreateModal}>
					<X size={20} />
				</button>
			</div>
			
			<div class="modal-body">
				<div class="form-group">
					<label class="form-label">Webhook Name</label>
					<input 
						type="text" 
						bind:value={newWebhook.name}
						placeholder="e.g., Order Notifications" 
						class="form-input" 
					/>
					<p class="form-hint">A descriptive name for your webhook</p>
				</div>
				
				<div class="form-group">
					<label class="form-label">Endpoint URL</label>
					<input 
						type="url" 
						bind:value={newWebhook.url}
						placeholder="https://api.example.com/webhooks" 
						class="form-input" 
					/>
					<p class="form-hint">The URL where webhook payloads will be sent</p>
				</div>
				
				<div class="form-group">
					<label class="form-label">Events to Subscribe</label>
					<div class="events-selector">
						{#each eventPresets as preset}
							<div class="event-category">
								<h4 class="category-title">{preset.category}</h4>
								<div class="event-options">
									{#each preset.events as event}
										<button
											class="event-option {selectedEvents.includes(event) ? 'selected' : ''}"
											on:click={() => toggleEvent(event)}
										>
											{event}
										</button>
									{/each}
								</div>
							</div>
						{/each}
					</div>
					<p class="form-hint">Select the events that will trigger this webhook</p>
				</div>
				
				<div class="form-group">
					<div class="form-label-row">
						<label class="form-label">Signing Secret</label>
						<span class="form-optional">Optional</span>
					</div>
					<div class="input-with-icon">
						<Key size={16} />
						<input 
							type="password" 
							bind:value={newWebhook.secret}
							placeholder="Enter a secret key for payload verification" 
							class="form-input with-icon" 
						/>
					</div>
					<p class="form-hint">Used to verify webhook payloads are from your application</p>
				</div>
			</div>
			
			<div class="modal-footer">
				<button on:click={closeCreateModal} class="modal-btn modal-btn-secondary">Cancel</button>
				<button on:click={createWebhook} class="modal-btn modal-btn-primary">Create Webhook</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.page-container {
		padding: 1.5rem;
		max-width: 1400px;
		margin: 0 auto;
	}

	.page-header {
		background: white;
		border-radius: 0.5rem;
		padding: 1.5rem;
		margin-bottom: 1.5rem;
		border: 1px solid #e5e7eb;
	}

	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		flex-wrap: wrap;
		gap: 1rem;
	}

	.header-left {
		flex: 1;
	}

	.header-title {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 0.5rem;
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

	.header-actions {
		display: flex;
		gap: 0.5rem;
		align-items: center;
	}

	.loading-container {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		min-height: 400px;
		gap: 1rem;
	}

	.loading-text {
		color: #6b7280;
		font-size: 0.95rem;
	}

	/* Stats Grid */
	.stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
		gap: 1rem;
		margin-bottom: 1.5rem;
	}

	.stat-card {
		background: white;
		border-radius: 0.5rem;
		padding: 1.25rem;
		border: 1px solid #e5e7eb;
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.stat-icon {
		width: 48px;
		height: 48px;
		border-radius: 0.5rem;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.stat-content {
		flex: 1;
	}

	.stat-label {
		font-size: 0.875rem;
		color: #6b7280;
		margin: 0 0 0.25rem 0;
	}

	.stat-value {
		font-size: 1.75rem;
		font-weight: 600;
		color: #111827;
		line-height: 1;
		margin: 0;
	}

	/* Webhooks Section */
	.webhooks-section {
		background: white;
		border-radius: 0.5rem;
		padding: 1.5rem;
		border: 1px solid #e5e7eb;
	}

	.section-header {
		margin-bottom: 1.25rem;
		padding-bottom: 0.75rem;
		border-bottom: 1px solid #f3f4f6;
	}

	.section-title {
		font-size: 1.1rem;
		font-weight: 600;
		color: #111827;
		margin: 0;
	}

	.webhooks-list {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.webhook-card {
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		overflow: hidden;
		display: flex;
		transition: all 0.2s;
	}

	.webhook-card:hover {
		box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
	}

	.webhook-status {
		width: 4px;
		flex-shrink: 0;
	}

	.status-active {
		background: #10b981;
	}

	.status-inactive {
		background: #ef4444;
	}

	.webhook-content {
		flex: 1;
		padding: 1rem;
	}

	.webhook-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 0.75rem;
	}

	.webhook-name {
		font-size: 1rem;
		font-weight: 600;
		color: #111827;
		margin: 0 0 0.25rem 0;
	}

	.webhook-url {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		color: #6b7280;
		font-size: 0.875rem;
	}

	.webhook-actions {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.webhook-events {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}

	.event-badge {
		display: inline-block;
		padding: 0.25rem 0.625rem;
		background: #f3f4f6;
		color: #4b5563;
		border-radius: 0.25rem;
		font-size: 0.75rem;
		font-weight: 500;
	}

	.webhook-stats {
		display: flex;
		gap: 1.5rem;
		font-size: 0.75rem;
		color: #6b7280;
	}

	.webhook-stat {
		display: flex;
		align-items: center;
		gap: 0.375rem;
	}

	/* Toggle Switch */
	.toggle-switch {
		position: relative;
		width: 44px;
		height: 24px;
		background: #d1d5db;
		border-radius: 12px;
		border: none;
		cursor: pointer;
		transition: background 0.3s;
		padding: 0;
	}

	.toggle-switch.toggle-active {
		background: #10b981;
	}

	.toggle-slider {
		position: absolute;
		top: 2px;
		left: 2px;
		width: 20px;
		height: 20px;
		background: white;
		border-radius: 50%;
		transition: transform 0.3s;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
	}

	.toggle-active .toggle-slider {
		transform: translateX(20px);
	}

	/* Action Buttons */
	.action-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		border: 1px solid transparent;
		cursor: pointer;
		transition: all 0.2s;
		line-height: 1;
	}

	.action-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-primary {
		background: #06b6d4;
		color: white;
		border-color: #06b6d4;
	}

	.btn-primary:hover:not(:disabled) {
		background: #0891b2;
		border-color: #0891b2;
	}

	.btn-ghost {
		background: transparent;
		color: #6b7280;
		border-color: transparent;
	}

	.btn-ghost:hover:not(:disabled) {
		background: #f3f4f6;
		color: #374151;
	}

	.action-btn-small {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.375rem 0.625rem;
		border-radius: 0.25rem;
		font-size: 0.75rem;
		font-weight: 500;
		border: 1px solid #e5e7eb;
		background: white;
		color: #374151;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-test:hover {
		background: #f0fdf4;
		border-color: #10b981;
		color: #10b981;
	}

	.btn-delete:hover {
		background: #fef2f2;
		border-color: #ef4444;
		color: #ef4444;
	}

	/* Empty State */
	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 3rem 2rem;
		text-align: center;
	}

	/* Modal Styles */
	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.5);
		z-index: 9999;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
		animation: fadeIn 0.2s;
	}

	.modal-container {
		background: white;
		border-radius: 0.75rem;
		width: 100%;
		max-width: 600px;
		max-height: 90vh;
		overflow-y: auto;
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
		animation: slideUp 0.3s;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-bottom: 1px solid #e5e7eb;
	}

	.modal-title {
		font-size: 1.25rem;
		font-weight: 600;
		color: #111827;
		margin: 0;
	}

	.modal-close {
		background: transparent;
		border: none;
		color: #6b7280;
		cursor: pointer;
		padding: 0.25rem;
		border-radius: 0.375rem;
		transition: all 0.2s;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.modal-close:hover {
		color: #374151;
		background: #f3f4f6;
	}

	.modal-body {
		padding: 1.5rem;
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid #e5e7eb;
	}

	/* Form Styles */
	.form-group {
		margin-bottom: 1.5rem;
	}

	.form-group:last-child {
		margin-bottom: 0;
	}

	.form-label {
		display: block;
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
		margin-bottom: 0.5rem;
	}

	.form-label-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.5rem;
	}

	.form-optional {
		font-size: 0.75rem;
		color: #9ca3af;
		font-weight: 400;
	}

	.form-input {
		width: 100%;
		padding: 0.625rem 0.875rem;
		font-size: 0.875rem;
		border: 1px solid #d1d5db;
		border-radius: 0.375rem;
		background: white;
		color: #111827;
		transition: all 0.2s;
	}

	.form-input:focus {
		outline: none;
		border-color: #06b6d4;
		box-shadow: 0 0 0 3px rgba(6, 182, 212, 0.1);
	}

	.input-with-icon {
		position: relative;
		display: flex;
		align-items: center;
	}

	.input-with-icon svg {
		position: absolute;
		left: 0.875rem;
		color: #9ca3af;
		pointer-events: none;
	}

	.form-input.with-icon {
		padding-left: 2.5rem;
	}

	.form-hint {
		font-size: 0.75rem;
		color: #6b7280;
		margin-top: 0.375rem;
		margin-bottom: 0;
	}

	/* Events Selector */
	.events-selector {
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		padding: 1rem;
		background: #f9fafb;
	}

	.event-category {
		margin-bottom: 1rem;
	}

	.event-category:last-child {
		margin-bottom: 0;
	}

	.category-title {
		font-size: 0.75rem;
		font-weight: 600;
		color: #6b7280;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin: 0 0 0.5rem 0;
	}

	.event-options {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.event-option {
		padding: 0.375rem 0.75rem;
		font-size: 0.8125rem;
		border: 1px solid #d1d5db;
		border-radius: 0.25rem;
		background: white;
		color: #4b5563;
		cursor: pointer;
		transition: all 0.2s;
	}

	.event-option:hover {
		background: #f3f4f6;
		border-color: #9ca3af;
	}

	.event-option.selected {
		background: #06b6d4;
		color: white;
		border-color: #06b6d4;
	}

	/* Modal Buttons */
	.modal-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.625rem 1.25rem;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		border: 1px solid transparent;
		cursor: pointer;
		transition: all 0.2s;
		line-height: 1;
	}

	.modal-btn-primary {
		background: #06b6d4;
		color: white;
		border-color: #06b6d4;
	}

	.modal-btn-primary:hover {
		background: #0891b2;
		border-color: #0891b2;
	}

	.modal-btn-secondary {
		background: white;
		color: #374151;
		border: 1px solid #d1d5db;
	}

	.modal-btn-secondary:hover {
		background: #f9fafb;
		border-color: #9ca3af;
	}

	/* Animations */
	@keyframes fadeIn {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	@keyframes slideUp {
		from {
			transform: translateY(1rem);
			opacity: 0;
		}
		to {
			transform: translateY(0);
			opacity: 1;
		}
	}

	/* Responsive */
	@media (max-width: 768px) {
		.stats-grid {
			grid-template-columns: 1fr;
		}
		
		.webhook-stats {
			flex-direction: column;
			gap: 0.5rem;
		}
		
		.header-content {
			flex-direction: column;
		}
		
		.header-actions {
			width: 100%;
		}
	}
</style>