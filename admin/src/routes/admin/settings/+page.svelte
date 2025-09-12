<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { AppSettings } from '$lib/types';
	import { Save, RefreshCw, AlertCircle, Check, Settings, Shield, Mail, HardDrive } from 'lucide-svelte';
	import { requireAdmin } from '$lib/utils/auth';
	
	let settings: AppSettings | null = null;
	let loading = true;
	let saving = false;
	let saved = false;
	let error = '';
	
	onMount(async () => {
		// Check admin access
		if (!requireAdmin()) return;
		
		await loadSettings();
	});
	
	async function loadSettings() {
		loading = true;
		error = '';
		const response = await api.getSettings();
		if (response.data) {
			settings = response.data;
		} else {
			error = response.error || 'Failed to load settings';
		}
		loading = false;
	}
	
	async function saveSettings() {
		if (!settings) return;
		
		saving = true;
		saved = false;
		error = '';
		
		const response = await api.updateSettings(settings);
		if (response.data) {
			settings = response.data;
			saved = true;
			setTimeout(() => saved = false, 3000);
		} else {
			error = response.error || 'Failed to save settings';
		}
		saving = false;
	}
	
	async function resetSettings() {
		if (!confirm('Are you sure you want to reset all settings to default values?')) {
			return;
		}
		
		saving = true;
		error = '';
		
		const response = await api.post('/settings/reset');
		if (response.data) {
			settings = response.data;
			saved = true;
			setTimeout(() => saved = false, 3000);
		} else {
			error = response.error || 'Failed to reset settings';
		}
		saving = false;
	}
	
	function formatBytes(bytes: number): string {
		const units = ['B', 'KB', 'MB', 'GB'];
		let size = bytes;
		let unitIndex = 0;
		
		while (size >= 1024 && unitIndex < units.length - 1) {
			size /= 1024;
			unitIndex++;
		}
		
		return `${size.toFixed(1)} ${units[unitIndex]}`;
	}
	
	function parseBytes(value: string): number {
		const match = value.match(/^(\d+(?:\.\d+)?)\s*([KMGT]?B)?$/i);
		if (!match) return 0;
		
		const num = parseFloat(match[1]);
		const unit = (match[2] || 'B').toUpperCase();
		
		const multipliers: Record<string, number> = {
			'B': 1,
			'KB': 1024,
			'MB': 1024 * 1024,
			'GB': 1024 * 1024 * 1024,
		};
		
		return Math.floor(num * (multipliers[unit] || 1));
	}
</script>

<div class="container mx-auto p-6 max-w-6xl">
	<div class="flex justify-between items-center mb-8">
		<h1 class="text-3xl font-bold text-gray-900">Settings</h1>
		<div class="flex gap-2">
			{#if saved}
				<div class="flex items-center text-green-600 px-3 py-2 bg-green-50 rounded">
					<Check size={16} class="mr-2" />
					Settings saved
				</div>
			{/if}
			<button 
				class="btn btn-ghost btn-sm"
				on:click={resetSettings}
				disabled={saving || loading}
			>
				<RefreshCw size={16} class="mr-2" />
				Reset to Defaults
			</button>
		</div>
	</div>
	
	<div class="settings-content">
		{#if error}
			<div class="error-banner">
				<AlertCircle size={18} />
				<span>{error}</span>
			</div>
		{/if}
		
		{#if loading}
		<div class="settings-grid">
			{#each [1,2,3,4] as _}
				<div class="settings-card">
					<div class="skeleton-title"></div>
					<div class="skeleton-content">
						<div class="skeleton-input"></div>
						<div class="skeleton-input"></div>
					</div>
				</div>
			{/each}
		</div>
	{:else if settings}
		<div class="settings-grid">
			<!-- General Settings -->
			<div class="card bg-base-100 shadow-sm">
				<div class="card-body">
					<h2 class="card-title text-xl mb-4">
						<Settings size={20} />
						General Settings
					</h2>
				<div class="settings-fields">
					<div class="form-control">
						<label class="label">
							<span class="label-text font-medium">Application Name</span>
						</label>
						<input 
							type="text" 
							class="input input-bordered" 
							bind:value={settings.app_name}
							placeholder="My Application"
						/>
					</div>
					
					<div class="form-control">
						<label class="label">
							<span class="label-text font-medium">Application URL</span>
						</label>
						<input 
							type="url" 
							class="input input-bordered" 
							bind:value={settings.app_url}
							placeholder="https://example.com"
						/>
					</div>
					
					<div class="form-control">
						<label class="label cursor-pointer">
							<span class="label-text font-medium">Allow User Signup</span>
							<input 
								type="checkbox" 
								class="toggle toggle-primary" 
								bind:checked={settings.allow_signup}
							/>
						</label>
					</div>
					
					<div class="form-control">
						<label class="label cursor-pointer">
							<span class="label-text font-medium">Require Email Confirmation</span>
							<input 
								type="checkbox" 
								class="toggle toggle-primary" 
								bind:checked={settings.require_email_confirmation}
							/>
						</label>
					</div>
					
					<div class="form-control">
						<label class="label">
							<span class="label-text font-medium">Notification Banner</span>
						</label>
						<textarea 
							class="textarea textarea-bordered" 
							bind:value={settings.notification}
							placeholder="Enter a notification message to display to all users"
							rows="2"
						/>
						<label class="label">
							<span class="label-text-alt">Leave empty to hide the notification banner</span>
						</label>
					</div>
				</div>
				</div>
			</div>
			
			<!-- Security Settings -->
			<div class="card bg-base-100 shadow-sm">
				<div class="card-body">
					<h2 class="card-title text-xl mb-4">
						<Shield size={20} />
						Security
					</h2>
					<div class="settings-fields">
						<div class="form-control">
							<label class="label">
								<span class="label-text font-medium">Session Timeout</span>
								<span class="label-text-alt">minutes</span>
							</label>
							<input 
								type="number" 
								class="input input-bordered" 
								bind:value={settings.session_timeout}
								min="5"
								placeholder="1440"
							/>
						</div>
						
						<div class="form-control">
							<label class="label">
								<span class="label-text font-medium">Minimum Password Length</span>
							</label>
							<input 
								type="number" 
								class="input input-bordered" 
								bind:value={settings.password_min_length}
								min="6"
								max="32"
								placeholder="8"
							/>
						</div>
					</div>
				</div>
			</div>
			
			<!-- Email Configuration -->
			<div class="card bg-base-100 shadow-sm">
				<div class="card-body">
					<h2 class="card-title text-xl mb-4">
						<Mail size={20} />
						Email Configuration
					</h2>
					
					<div class="form-control mb-4">
						<label class="label cursor-pointer justify-start gap-3">
							<input 
								type="checkbox" 
								class="toggle toggle-primary" 
								bind:checked={settings.smtp_enabled}
							/>
							<span class="label-text font-medium">Enable SMTP</span>
						</label>
					</div>
					
					{#if settings.smtp_enabled}
						<div class="grid grid-cols-1 md:grid-cols-2 gap-4 pl-4 border-l-2 border-primary/20">
							<div class="form-control">
								<label class="label">
									<span class="label-text font-medium">SMTP Host</span>
								</label>
								<input 
									type="text" 
									class="input input-bordered" 
									bind:value={settings.smtp_host}
									placeholder="smtp.example.com"
								/>
							</div>
							
							<div class="form-control">
								<label class="label">
									<span class="label-text font-medium">SMTP Port</span>
								</label>
								<input 
									type="number" 
									class="input input-bordered" 
									bind:value={settings.smtp_port}
									placeholder="587"
								/>
							</div>
							
							<div class="form-control md:col-span-2">
								<label class="label">
									<span class="label-text font-medium">SMTP Username</span>
								</label>
								<input 
									type="text" 
									class="input input-bordered" 
									bind:value={settings.smtp_user}
									placeholder="user@example.com"
								/>
							</div>
						</div>
					{/if}
				</div>
			</div>
			
			<!-- Storage Configuration -->
			<div class="card bg-base-100 shadow-sm">
				<div class="card-body">
					<h2 class="card-title text-xl mb-4">
						<HardDrive size={20} />
						Storage Configuration
					</h2>
					
					<div class="settings-fields">
						<div class="form-control">
							<label class="label">
								<span class="label-text font-medium">Storage Provider</span>
							</label>
							<select class="select select-bordered" bind:value={settings.storage_provider}>
								<option value="local">Local Filesystem</option>
								<option value="s3">Amazon S3</option>
							</select>
						</div>
						
						<div class="form-control">
							<label class="label">
								<span class="label-text font-medium">Max Upload Size</span>
							</label>
							<input 
								type="text" 
								class="input input-bordered" 
								value={formatBytes(settings.max_upload_size)}
								on:change={(e) => settings.max_upload_size = parseBytes(e.currentTarget.value)}
								placeholder="10 MB"
							/>
						</div>
						
						<div class="form-control md:col-span-2">
							<label class="label">
								<span class="label-text font-medium">Allowed File Types</span>
								<span class="label-text-alt">comma-separated MIME types</span>
							</label>
							<input 
								type="text" 
								class="input input-bordered" 
								bind:value={settings.allowed_file_types}
								placeholder="image/*,application/pdf,text/*"
							/>
						</div>
					</div>
					
					{#if settings.storage_provider === 's3'}
						<div class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4 pl-4 border-l-2 border-primary/20">
							<div class="form-control">
								<label class="label">
									<span class="label-text font-medium">S3 Bucket</span>
								</label>
								<input 
									type="text" 
									class="input input-bordered" 
									bind:value={settings.s3_bucket}
									placeholder="my-bucket"
								/>
							</div>
							
							<div class="form-control">
								<label class="label">
									<span class="label-text font-medium">S3 Region</span>
								</label>
								<input 
									type="text" 
									class="input input-bordered" 
									bind:value={settings.s3_region}
									placeholder="us-east-1"
								/>
							</div>
						</div>
					{/if}
				</div>
			</div>
			
			<!-- Developer Settings -->
			<div class="card bg-base-100 shadow-sm">
				<div class="card-body">
					<h2 class="card-title text-xl mb-4">Developer Options</h2>
					
					<div class="space-y-3">
						<div class="form-control">
							<label class="label cursor-pointer justify-start gap-3">
								<input 
									type="checkbox" 
									class="toggle toggle-primary" 
									bind:checked={settings.enable_api_logs}
								/>
								<span class="label-text font-medium">Enable API Logging</span>
							</label>
						</div>
						
						<div class="form-control">
							<label class="label cursor-pointer justify-start gap-3">
								<input 
									type="checkbox" 
									class="toggle toggle-warning" 
									bind:checked={settings.enable_debug_mode}
								/>
								<span class="label-text font-medium">Enable Debug Mode</span>
							</label>
						</div>
					</div>
				</div>
			</div>
			
			<!-- Maintenance Mode -->
			<div class="card bg-base-100 shadow-sm border-2 border-warning/20">
				<div class="card-body">
					<h2 class="card-title text-xl mb-4">
						<AlertCircle size={20} class="text-warning" />
						Maintenance Mode
					</h2>
					
					<div class="form-control mb-4">
						<label class="label cursor-pointer justify-start gap-3">
							<input 
								type="checkbox" 
								class="toggle toggle-warning" 
								bind:checked={settings.maintenance_mode}
							/>
							<span class="label-text font-medium">Enable Maintenance Mode</span>
						</label>
					</div>
					
					{#if settings.maintenance_mode}
						<div class="form-control pl-4 border-l-2 border-warning/20">
							<label class="label">
								<span class="label-text font-medium">Maintenance Message</span>
							</label>
							<textarea 
								class="textarea textarea-bordered h-24" 
								bind:value={settings.maintenance_message}
								placeholder="We're currently performing maintenance. Please check back later."
							></textarea>
						</div>
					{/if}
				</div>
			</div>
			
			<!-- Save Button -->
			<div class="save-container">
				<button 
					class="save-button" 
					on:click={saveSettings}
					disabled={saving}
				>
					{#if saving}
						<div class="spinner"></div>
						Saving...
					{:else}
						<Save size={18} />
						Save Settings
					{/if}
				</button>
			</div>
		</div>
	{:else}
		<div class="error-banner">
			<AlertCircle size={18} />
			<span>Failed to load settings. Please try refreshing the page.</span>
		</div>
	{/if}
	</div>
</div>

<style>
	.settings-container {
		min-height: 100vh;
		background: #f3f4f6;
	}
	
	/* Make cards more compact */
	:global(.card) {
		box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05) !important;
	}
	
	:global(.card-body) {
		padding: 1rem !important;
	}
	
	:global(.card-title) {
		margin-bottom: 0.75rem !important;
		font-size: 1rem !important;
		font-weight: 600 !important;
	}
	
	/* Make form controls more compact */
	:global(.form-control) {
		margin-bottom: 0.5rem;
	}
	
	:global(.form-control:last-child) {
		margin-bottom: 0;
	}
	
	:global(.form-control .label) {
		padding-top: 0;
		padding-bottom: 0.125rem;
		min-height: 1.75rem;
	}
	
	:global(.input) {
		height: 2.25rem;
		font-size: 0.875rem;
		padding: 0.375rem 0.75rem;
	}
	
	:global(.textarea) {
		font-size: 0.875rem;
		min-height: 4rem;
		padding: 0.5rem 0.75rem;
	}
	
	:global(.input-bordered) {
		border-width: 1px;
	}
	
	/* Fix checkbox layout */
	:global(.form-control .label.cursor-pointer) {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.375rem 0;
		min-height: 2rem;
	}
	
	:global(.toggle) {
		flex-shrink: 0;
		margin-left: 1rem;
		transform: scale(0.9);
	}
	
	:global(.label-text) {
		font-size: 0.875rem;
	}
	
	.page-header {
		background: white;
		border-bottom: 1px solid #e5e7eb;
		padding: 1.5rem 2rem;
	}
	
	.header-content {
		max-width: 1200px;
		margin: 0 auto;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	
	.page-title {
		font-size: 1.5rem;
		font-weight: 700;
		color: #111827;
		margin: 0 0 0.25rem 0;
	}
	
	.page-subtitle {
		color: #6b7280;
		font-size: 0.875rem;
		margin: 0;
	}
	
	.header-actions {
		display: flex;
		align-items: center;
		gap: 1rem;
	}
	
	.saved-indicator {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: #10b981;
		font-size: 0.875rem;
		font-weight: 500;
		padding: 0.5rem 1rem;
		background: #ecfdf5;
		border-radius: 6px;
		animation: fadeIn 0.3s ease;
	}
	
	@keyframes fadeIn {
		from { opacity: 0; transform: translateY(-10px); }
		to { opacity: 1; transform: translateY(0); }
	}
	
	.btn-secondary {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		background: white;
		color: #374151;
		border: 1px solid #d1d5db;
		border-radius: 6px;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.btn-secondary:hover:not(:disabled) {
		background: #f9fafb;
		border-color: #9ca3af;
	}
	
	.btn-secondary:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	
	.settings-content {
		max-width: 1200px;
		margin: 0 auto;
		padding: 1rem;
	}
	
	.error-banner {
		background: #fee2e2;
		color: #dc2626;
		padding: 1rem;
		border-radius: 8px;
		margin-bottom: 1.5rem;
		display: flex;
		align-items: center;
		gap: 0.75rem;
		font-size: 0.875rem;
	}
	
	.settings-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(500px, 1fr));
		gap: 0.75rem;
	}
	
	.settings-card {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 8px;
		padding: 1.5rem;
	}
	
	.maintenance-card {
		border-color: #fed7aa;
		background: #fffbf5;
	}
	
	.card-title {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		font-size: 1.125rem;
		font-weight: 600;
		color: #111827;
		margin-bottom: 1.25rem;
		padding-bottom: 0.75rem;
		border-bottom: 1px solid #e5e7eb;
	}
	
	.settings-fields {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1rem;
	}
	
	.conditional-fields {
		margin-top: 1rem;
		padding-left: 1rem;
		border-left: 2px solid #e5e7eb;
	}
	
	.form-control {
		display: flex;
		flex-direction: column;
	}
	
	.form-label, .label {
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
		margin-bottom: 0.5rem;
	}
	
	.label {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	
	.label-text {
		color: #374151;
	}
	
	.label-text-alt {
		color: #9ca3af;
		font-size: 0.75rem;
	}
	
	.input, .select, .textarea {
		padding: 0.5rem 0.75rem;
		border: 1px solid #d1d5db;
		border-radius: 6px;
		font-size: 0.875rem;
		background: white;
		color: #111827;
		transition: all 0.2s;
	}
	
	.input:focus, .select:focus, .textarea:focus {
		outline: none;
		border-color: #6366f1;
		box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
	}
	
	.input:disabled, .select:disabled, .textarea:disabled {
		background: #f9fafb;
		cursor: not-allowed;
		opacity: 0.6;
	}
	
	.textarea {
		resize: vertical;
		min-height: 100px;
	}
	
	.label.cursor-pointer {
		cursor: pointer;
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.5rem 0;
	}
	
	.toggle {
		position: relative;
		width: 44px;
		height: 24px;
		appearance: none;
		background: #d1d5db;
		border-radius: 12px;
		cursor: pointer;
		transition: all 0.3s;
	}
	
	.toggle::after {
		content: '';
		position: absolute;
		top: 2px;
		left: 2px;
		width: 20px;
		height: 20px;
		background: white;
		border-radius: 50%;
		transition: all 0.3s;
	}
	
	.toggle:checked {
		background: #6366f1;
	}
	
	.toggle:checked::after {
		left: 22px;
	}
	
	.toggle-primary:checked {
		background: #6366f1;
	}
	
	.toggle-warning:checked {
		background: #f59e0b;
	}
	
	.save-container {
		position: sticky;
		bottom: 2rem;
		display: flex;
		justify-content: flex-end;
		margin-top: 2rem;
	}
	
	.save-button {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1.5rem;
		background: #6366f1;
		color: white;
		border: none;
		border-radius: 8px;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
		transition: all 0.2s;
	}
	
	.save-button:hover:not(:disabled) {
		background: #4f46e5;
		box-shadow: 0 6px 8px rgba(0, 0, 0, 0.15);
		transform: translateY(-1px);
	}
	
	.save-button:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
	
	.spinner {
		width: 16px;
		height: 16px;
		border: 2px solid rgba(255, 255, 255, 0.3);
		border-top-color: white;
		border-radius: 50%;
		animation: spin 0.6s linear infinite;
	}
	
	@keyframes spin {
		to { transform: rotate(360deg); }
	}
	
	/* Skeleton loaders */
	.skeleton-title {
		height: 28px;
		width: 150px;
		background: linear-gradient(90deg, #f0f0f0 25%, #e0e0e0 50%, #f0f0f0 75%);
		background-size: 200% 100%;
		animation: loading 1.5s infinite;
		border-radius: 4px;
		margin-bottom: 1.25rem;
	}
	
	.skeleton-content {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1rem;
	}
	
	.skeleton-input {
		height: 36px;
		background: linear-gradient(90deg, #f0f0f0 25%, #e0e0e0 50%, #f0f0f0 75%);
		background-size: 200% 100%;
		animation: loading 1.5s infinite;
		border-radius: 6px;
	}
	
	@keyframes loading {
		0% { background-position: 200% 0; }
		100% { background-position: -200% 0; }
	}
	
	/* Responsive */
	@media (max-width: 768px) {
		.settings-grid {
			grid-template-columns: 1fr;
		}
		
		.settings-fields {
			grid-template-columns: 1fr;
		}
		
		.header-content {
			flex-direction: column;
			align-items: flex-start;
			gap: 1rem;
		}
		
		.header-actions {
			width: 100%;
			justify-content: flex-start;
		}
	}
</style>

