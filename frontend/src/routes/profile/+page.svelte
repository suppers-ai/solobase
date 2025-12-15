<script lang="ts">
	import '../../app.css';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import {
		Lock, LogOut, Shield,
		Save, Settings,
		Package, HardDrive, TrendingUp,
		Activity, Share2, Download, Upload, ArrowLeft, Key, Copy, Trash2, Plus	} from 'lucide-svelte';
	import { api } from '$lib/api';
	import { authStore } from '$lib/stores/auth';
	import { formatBytes } from '$lib/utils/formatters';
	import LoadingSpinner from '$lib/components/ui/LoadingSpinner.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import Modal from '$lib/components/ui/Modal.svelte';

	let user: any = null;
	let roles: string[] = [];
	let loading = true;
	let saving = false;
	let error = '';
	let successMessage = '';

	// Modals
	let showAccountSettings = false;
	let showPasswordChange = false;
	let showStorageModal = false;
	let showAPIKeysModal = false;
	let showCreateKeyModal = false;

	// Storage data
	let showStorageCard = false;
	let storageStats: any = null;
	let storageQuota: any = null;
	let recentActivity: any[] = [];

	// API Keys data
	let apiKeys: any[] = [];
	let apiKeysLoading = false;
	let newKeyName = '';
	let newKeyExpiry: string | null = null;
	let createdKey: string | null = null;
	let keyCreating = false;
	let keyError = '';
	let showRevokeConfirm = false;
	let keyToRevoke: { id: string; name: string } | null = null;
	
	// Profile form data
	let profileForm = {
		firstName: '',
		lastName: '',
		displayName: '',
		email: '',
		phone: '',
		location: ''
	};
	
	// Password change form
	let passwordForm = {
		currentPassword: '',
		newPassword: '',
		confirmPassword: ''
	};
	let passwordError = '';
	let passwordSuccess = '';
	
	onMount(async () => {
		// Check if user is logged in
		const currentUser = $authStore.user;
		if (!currentUser) {
			goto('/auth/login');
			return;
		}

		interface AuthMeResponse {
			user: any;
			roles?: string[];
		}

		try {
			// Fetch current user details - returns { user, roles, permissions }
			const response = await api.get<AuthMeResponse>('/auth/me');
			user = response.user;
			roles = response.roles || [];

			// Populate form with current user data (using camelCase from API)
			profileForm = {
				firstName: user.firstName || '',
				lastName: user.lastName || '',
				displayName: user.displayName || user.email?.split('@')[0] || '',
				email: user.email || '',
				phone: user.phone || '',
				location: user.location || ''
			};

			// Check if cloud storage extension is enabled and should show in profile
			await checkStorageSettings();

			loading = false;
		} catch (err: any) {
			error = err.message || 'Failed to load user profile';
			loading = false;
		}
	});
	
	async function saveProfile() {
		saving = true;
		error = '';
		successMessage = '';

		try {
			// Use /auth/me endpoint for updating own profile
			await api.patch('/auth/me', {
				firstName: profileForm.firstName,
				lastName: profileForm.lastName,
				displayName: profileForm.displayName,
				phone: profileForm.phone,
				location: profileForm.location
			});

			successMessage = 'Profile updated successfully';

			// Update local user object (camelCase to match API response)
			user = {
				...user,
				firstName: profileForm.firstName,
				lastName: profileForm.lastName,
				displayName: profileForm.displayName,
				phone: profileForm.phone,
				location: profileForm.location
			};

			// Update auth store
			authStore.updateUser(user);

			showAccountSettings = false;

			setTimeout(() => {
				successMessage = '';
			}, 3000);
		} catch (err: any) {
			error = err.message || 'Failed to update profile';
		} finally {
			saving = false;
		}
	}
	
	async function changePassword() {
		passwordError = '';
		passwordSuccess = '';
		
		// Validate passwords
		if (passwordForm.newPassword !== passwordForm.confirmPassword) {
			passwordError = 'New passwords do not match';
			return;
		}
		
		if (passwordForm.newPassword.length < 8) {
			passwordError = 'Password must be at least 8 characters';
			return;
		}
		
		try {
			await api.post('/auth/change-password', {
				currentPassword: passwordForm.currentPassword,
				newPassword: passwordForm.newPassword
			});
			
			passwordSuccess = 'Password changed successfully';
			showPasswordChange = false;
			
			// Clear form
			passwordForm = {
				currentPassword: '',
				newPassword: '',
				confirmPassword: ''
			};
			
			setTimeout(() => {
				passwordSuccess = '';
			}, 3000);
		} catch (err: any) {
			passwordError = err.message || 'Failed to change password';
		}
	}
	
	async function logout() {
		try {
			await api.post('/auth/logout', {});
			authStore.logout();
			goto('/auth/login');
		} catch (err) {
			// Even if logout fails on server, clear local auth
			authStore.logout();
			goto('/auth/login');
		}
	}
	
	function getInitials(email: string | undefined | null): string {
		if (!email) return '??';
		return email.substring(0, 2).toUpperCase();
	}
	
	function getAvatarColor(email: string | undefined | null): string {
		const colors = [
			'#3b82f6', '#ef4444', '#10b981', '#f59e0b',
			'#8b5cf6', '#ec4899', '#06b6d4', '#84cc16'
		];
		if (!email) return colors[0];
		let hash = 0;
		for (let i = 0; i < email.length; i++) {
			hash = email.charCodeAt(i) + ((hash << 5) - hash);
		}
		return colors[Math.abs(hash) % colors.length];
	}
	
	interface SettingResponse {
		value?: string | boolean;
	}

	async function checkStorageSettings() {
		try {
			// Check if the setting exists and is enabled
			const response = await api.get<SettingResponse>('/settings/ext_cloudstorage_profile_show_usage');
			if (response && response.value) {
				showStorageCard = response.value === 'true' || response.value === true;
			}
		} catch (err) {
			// Setting doesn't exist, don't show storage card
			showStorageCard = false;
		}
	}
	
	async function loadStorageData() {
		try {
			// Load storage statistics
			const [statsRes, quotaRes, logsRes] = await Promise.all([
				api.get('/ext/cloudstorage/stats').catch(() => null),
				api.get('/ext/cloudstorage/quotas/user').catch(() => null),
				api.get<any[]>('/ext/cloudstorage/access-logs?user_id=me&limit=10').catch(() => [])
			]);

			storageStats = statsRes;
			storageQuota = quotaRes;
			recentActivity = logsRes || [];
		} catch (err) {
			console.error('Failed to load storage data:', err);
		}
	}
	
	async function openStorageModal() {
		showStorageModal = true;
		if (!storageStats) {
			await loadStorageData();
		}
	}
	
	function getActionIcon(action: string) {
		switch (action) {
			case 'download': return Download;
			case 'upload': return Upload;
			case 'share': return Share2;
			default: return Activity;
		}
	}
	
	function getActionColor(action: string) {
		switch (action) {
			case 'download': return 'text-purple-600';
			case 'upload': return 'text-green-600';
			case 'share': return 'text-cyan-600';
			default: return 'text-gray-600';
		}
	}

	// API Key functions
	async function loadAPIKeys() {
		apiKeysLoading = true;
		keyError = '';
		apiKeys = []; // Initialize to empty array first
		try {
			const response = await api.get('/auth/api-keys');
			apiKeys = Array.isArray(response) ? response : [];
		} catch (err: any) {
			keyError = err.message || 'Failed to load API keys';
			apiKeys = [];
		} finally {
			apiKeysLoading = false;
		}
	}

	async function openAPIKeysModal() {
		showAPIKeysModal = true;
		createdKey = null;
		await loadAPIKeys();
	}

	function openCreateKeyModal() {
		showCreateKeyModal = true;
		newKeyName = '';
		newKeyExpiry = null;
		keyError = '';
	}

	async function createAPIKey() {
		if (!newKeyName.trim()) {
			keyError = 'Please enter a name for the API key';
			return;
		}

		keyCreating = true;
		keyError = '';

		interface ApiKeyResponse {
			key: string;
		}

		try {
			const payload: any = { name: newKeyName.trim() };
			if (newKeyExpiry) {
				payload.expiresAt = new Date(newKeyExpiry).toISOString();
			}

			const response = await api.post<ApiKeyResponse>('/auth/api-keys', payload);
			createdKey = response.key;
			showCreateKeyModal = false;
			await loadAPIKeys();
		} catch (err: any) {
			keyError = err.message || 'Failed to create API key';
		} finally {
			keyCreating = false;
		}
	}

	function revokeAPIKey(keyId: string, keyName: string) {
		keyToRevoke = { id: keyId, name: keyName };
		showRevokeConfirm = true;
	}

	async function confirmRevokeKey() {
		if (!keyToRevoke) return;
		showRevokeConfirm = false;

		try {
			await api.delete(`/auth/api-keys/${keyToRevoke.id}`);
			await loadAPIKeys();
		} catch (err: any) {
			keyError = err.message || 'Failed to revoke API key';
		}
		keyToRevoke = null;
	}

	async function copyToClipboard(text: string) {
		try {
			await navigator.clipboard.writeText(text);
			// Brief visual feedback could be added here
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

<svelte:head>
	<title>Profile - Solobase</title>
</svelte:head>

<div class="profile-page">
	<div class="profile-container">
		<!-- Back Button -->
		<a href="/" class="back-button" data-sveltekit-reload>
			<ArrowLeft size={18} />
			<span>Back to Home</span>
		</a>

		<div class="profile-card">
			<!-- Logo Header -->
			<div class="logo-header">
				<img src="/logo_long.png" alt="Solobase" class="logo" />
			</div>
			
			{#if loading}
				<div class="loading">
					<LoadingSpinner size="lg" />
					<p>Loading profile...</p>
				</div>
			{:else if user}
				<!-- User Avatar and Basic Info -->
				<div class="user-header">
					<div class="avatar" style="background-color: {getAvatarColor(user.email)}">
						{getInitials(user.email)}
					</div>
					<div class="user-info">
						<h2>{profileForm.displayName || user.email}</h2>
						<p class="email">{user.email}</p>
						<!-- Role removed from user model - now managed by IAM -->
					</div>
				</div>
				
				{#if successMessage}
					<div class="alert alert-success">{successMessage}</div>
				{/if}
				
				{#if passwordSuccess}
					<div class="alert alert-success">{passwordSuccess}</div>
				{/if}
				
				<!-- Quick Actions -->
				<div class="actions-grid">
					<!-- Products -->
					<button 
						class="action-card"
						on:click={() => goto('/profile/products')}
					>
						<Package size={24} />
						<span>Products</span>
					</button>
					
					<!-- Account Settings -->
					<button 
						class="action-card"
						on:click={() => showAccountSettings = true}
					>
						<Settings size={24} />
						<span>Account Settings</span>
					</button>
					
					<!-- Change Password -->
					<button
						class="action-card"
						on:click={() => showPasswordChange = true}
					>
						<Lock size={24} />
						<span>Change Password</span>
					</button>

					<!-- API Keys -->
					<button
						class="action-card"
						on:click={openAPIKeysModal}
					>
						<Key size={24} />
						<span>API Keys</span>
					</button>

					<!-- Storage Usage (if enabled) -->
					{#if showStorageCard}
						<button 
							class="action-card"
							on:click={openStorageModal}
						>
							<HardDrive size={24} />
							<span>Storage</span>
						</button>
					{/if}
					
					<!-- Admin Dashboard (only for admins) -->
					{#if roles.includes('admin')}
						<a href="/admin" class="action-card">
							<Shield size={24} />
							<span>Admin Dashboard</span>
						</a>
					{/if}
					
					<!-- Logout -->
					<button class="action-card logout" on:click={logout}>
						<LogOut size={24} />
						<span>Logout</span>
					</button>
				</div>
			{/if}
		</div>
	</div>
</div>

<!-- Account Settings Modal -->
<Modal show={showAccountSettings} title="Account Settings" on:close={() => showAccountSettings = false}>
	{#if error}
		<div class="alert alert-error">{error}</div>
	{/if}

	<div class="form-row">
		<div class="form-group">
			<label for="firstName">First Name</label>
			<input
				type="text"
				id="firstName"
				bind:value={profileForm.firstName}
				placeholder="Enter first name"
			/>
		</div>

		<div class="form-group">
			<label for="lastName">Last Name</label>
			<input
				type="text"
				id="lastName"
				bind:value={profileForm.lastName}
				placeholder="Enter last name"
			/>
		</div>
	</div>

	<div class="form-group">
		<label for="displayName">Display Name</label>
		<input
			type="text"
			id="displayName"
			bind:value={profileForm.displayName}
			placeholder="Enter display name"
		/>
	</div>

	<div class="form-group">
		<label for="email">Email</label>
		<input
			type="email"
			id="email"
			value={profileForm.email}
			disabled
			class="disabled"
		/>
	</div>

	<div class="form-group">
		<label for="phone">Phone</label>
		<input
			type="tel"
			id="phone"
			bind:value={profileForm.phone}
			placeholder="Enter phone number"
		/>
	</div>

	<div class="form-group">
		<label for="location">Location</label>
		<input
			type="text"
			id="location"
			bind:value={profileForm.location}
			placeholder="Enter location"
		/>
	</div>

	<svelte:fragment slot="footer">
		<button
			class="btn btn-secondary"
			on:click={() => showAccountSettings = false}
		>
			Cancel
		</button>
		<button
			class="btn btn-primary"
			on:click={saveProfile}
			disabled={saving}
		>
			{#if saving}
				<LoadingSpinner size="sm" color="white" />
				Saving...
			{:else}
				<Save size={16} />
				Save Changes
			{/if}
		</button>
	</svelte:fragment>
</Modal>

<!-- Change Password Modal -->
<Modal show={showPasswordChange} title="Change Password" on:close={() => {
	showPasswordChange = false;
	passwordForm = { currentPassword: '', newPassword: '', confirmPassword: '' };
	passwordError = '';
}}>
	{#if passwordError}
		<div class="alert alert-error">{passwordError}</div>
	{/if}

	<div class="form-group">
		<label for="currentPassword">Current Password</label>
		<input
			type="password"
			id="currentPassword"
			bind:value={passwordForm.currentPassword}
			placeholder="Enter current password"
		/>
	</div>

	<div class="form-group">
		<label for="newPassword">New Password</label>
		<input
			type="password"
			id="newPassword"
			bind:value={passwordForm.newPassword}
			placeholder="Enter new password (min 8 characters)"
		/>
	</div>

	<div class="form-group">
		<label for="confirmPassword">Confirm New Password</label>
		<input
			type="password"
			id="confirmPassword"
			bind:value={passwordForm.confirmPassword}
			placeholder="Confirm new password"
		/>
	</div>

	<svelte:fragment slot="footer">
		<button
			class="btn btn-secondary"
			on:click={() => {
				showPasswordChange = false;
				passwordForm = { currentPassword: '', newPassword: '', confirmPassword: '' };
				passwordError = '';
			}}
		>
			Cancel
		</button>
		<button
			class="btn btn-primary"
			on:click={changePassword}
		>
			Change Password
		</button>
	</svelte:fragment>
</Modal>

<!-- Storage Usage Modal -->
<Modal show={showStorageModal} title="Storage Usage" maxWidth="600px" on:close={() => showStorageModal = false}>
	<!-- Storage Overview -->
	<div class="storage-overview">
		<div class="storage-stat-card">
			<div class="stat-icon storage-icon">
				<HardDrive size={20} />
			</div>
			<div class="stat-details">
				<span class="stat-label">Storage Used</span>
				<span class="stat-value">
					{#if storageQuota}
						{formatBytes(storageQuota.storageUsed || 0)}
					{:else}
						Loading...
					{/if}
				</span>
				{#if storageQuota && storageQuota.maxStorageBytes}
					<div class="progress-bar">
						<div class="progress-fill" style="width: {Math.min((storageQuota.storageUsed / storageQuota.maxStorageBytes) * 100, 100)}%"></div>
					</div>
					<span class="stat-detail">
						of {formatBytes(storageQuota.maxStorageBytes)} available
					</span>
				{/if}
			</div>
		</div>

		<div class="storage-stat-card">
			<div class="stat-icon bandwidth-icon">
				<TrendingUp size={20} />
			</div>
			<div class="stat-details">
				<span class="stat-label">Bandwidth Used</span>
				<span class="stat-value">
					{#if storageQuota}
						{formatBytes(storageQuota.bandwidthUsed || 0)}
					{:else}
						Loading...
					{/if}
				</span>
				{#if storageQuota && storageQuota.maxBandwidthBytes}
					<div class="progress-bar">
						<div class="progress-fill bandwidth" style="width: {Math.min((storageQuota.bandwidthUsed / storageQuota.maxBandwidthBytes) * 100, 100)}%"></div>
					</div>
					<span class="stat-detail">
						of {formatBytes(storageQuota.maxBandwidthBytes)} this month
					</span>
				{/if}
			</div>
		</div>
	</div>

	<!-- Storage Details -->
	{#if storageStats || storageQuota}
		<div class="storage-details">
			<h4>Storage Details</h4>
			<div class="detail-grid">
				<div class="detail-item">
					<span class="detail-label">Total Files:</span>
					<span class="detail-value">{storageStats?.storage?.totalObjects || 0}</span>
				</div>
				<div class="detail-item">
					<span class="detail-label">Shared Files:</span>
					<span class="detail-value">{storageStats?.shares?.totalShares || 0}</span>
				</div>
				{#if storageQuota?.resetBandwidthAt}
					<div class="detail-item">
						<span class="detail-label">Bandwidth Resets:</span>
						<span class="detail-value">
							{new Date(storageQuota.resetBandwidthAt).toLocaleDateString()}
						</span>
					</div>
				{/if}
				{#if storageQuota && storageQuota.storageUsed > storageQuota.maxStorageBytes * 0.9}
					<div class="detail-item warning">
						<span class="detail-label">⚠️ Storage Warning:</span>
						<span class="detail-value">Over 90% used</span>
					</div>
				{/if}
			</div>
		</div>
	{/if}

	<!-- Storage Info -->
	<div class="storage-tips">
		<h4>Storage Information</h4>
		<ul>
			<li>Your storage quota is managed by your administrator</li>
			<li>Contact your admin if you need more storage space</li>
			{#if storageQuota && storageQuota.storageUsed > storageQuota.maxStorageBytes * 0.75}
				<li class="warning">Your storage is almost full - please contact your administrator</li>
			{/if}
		</ul>
	</div>

	<!-- Recent Activity -->
	{#if recentActivity && recentActivity.length > 0}
		<div class="recent-activity">
			<h4>Recent Activity</h4>
			<div class="activity-list">
				{#each recentActivity.slice(0, 5) as activity}
					<div class="activity-item">
						<div class="activity-icon {getActionColor(activity.action)}">
							<svelte:component this={getActionIcon(activity.action)} size={14} />
						</div>
						<div class="activity-details">
							<span class="activity-action">{activity.action}</span>
							<span class="activity-time">{new Date(activity.createdAt).toLocaleString()}</span>
						</div>
					</div>
				{/each}
			</div>
		</div>
	{/if}

	<svelte:fragment slot="footer">
		{#if user && roles.includes('admin')}
			<a href="/admin/extensions/cloudstorage" class="btn btn-secondary">
				Extension Settings
			</a>
			<a href="/admin/storage" class="btn btn-secondary">
				Manage Files
			</a>
		{/if}
		<button
			class="btn btn-primary"
			on:click={() => showStorageModal = false}
		>
			Close
		</button>
	</svelte:fragment>
</Modal>

<!-- API Keys Modal -->
<Modal show={showAPIKeysModal} title="API Keys" maxWidth="600px" on:close={() => showAPIKeysModal = false}>
	{#if keyError}
		<div class="alert alert-error">{keyError}</div>
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
			<button class="btn btn-small btn-primary" on:click={openCreateKeyModal}>
				<Plus size={14} />
				Create Key
			</button>
		</div>

		{#if apiKeysLoading}
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
								<span class="key-separator">•</span>
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
							on:click={() => revokeAPIKey(key.id, key.name)}
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
		<button
			class="btn btn-primary"
			on:click={() => showAPIKeysModal = false}
		>
			Close
		</button>
	</svelte:fragment>
</Modal>

<!-- Create API Key Modal -->
<Modal show={showCreateKeyModal} title="Create API Key" on:close={() => showCreateKeyModal = false}>
	{#if keyError}
		<div class="alert alert-error">{keyError}</div>
	{/if}

	<div class="form-group">
		<label for="keyName">Key Name</label>
		<input
			type="text"
			id="keyName"
			bind:value={newKeyName}
			placeholder="e.g., Production Server, CI/CD Pipeline"
		/>
		<span class="form-hint">A descriptive name to identify this key</span>
	</div>

	<div class="form-group">
		<label for="keyExpiry">Expiration (Optional)</label>
		<input
			type="datetime-local"
			id="keyExpiry"
			bind:value={newKeyExpiry}
		/>
		<span class="form-hint">Leave empty for a key that never expires</span>
	</div>

	<svelte:fragment slot="footer">
		<button
			class="btn btn-secondary"
			on:click={() => showCreateKeyModal = false}
		>
			Cancel
		</button>
		<button
			class="btn btn-primary"
			on:click={createAPIKey}
			disabled={keyCreating}
		>
			{#if keyCreating}
				<LoadingSpinner size="sm" color="white" />
				Creating...
			{:else}
				<Key size={16} />
				Create Key
			{/if}
		</button>
	</svelte:fragment>
</Modal>

<ConfirmDialog
	bind:show={showRevokeConfirm}
	title="Revoke API Key"
	message={`Are you sure you want to revoke the API key "${keyToRevoke?.name}"? This action cannot be undone.`}
	confirmText="Revoke"
	variant="danger"
	on:confirm={confirmRevokeKey}
/>

<style>
	.profile-page {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #f0f0f0;
		padding: 1rem;
	}
	
	.profile-container {
		width: 100%;
		max-width: 500px;
		position: relative;
	}

	/* Back Button */
	.back-button {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.375rem 0.625rem;
		margin-bottom: 0.75rem;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		color: #374151;
		font-size: 0.813rem;
		font-weight: 500;
		text-decoration: none;
		transition: all 0.2s;
		width: fit-content;
	}

	.back-button:hover {
		background: #f9fafb;
		border-color: #189AB4;
		transform: translateX(-2px);
	}

	.profile-card {
		background: white;
		border: 1px solid #e2e8f0;
		border-radius: 12px;
		padding: 2rem;
		animation: slideUp 0.4s ease-out;
	}
	
	@keyframes slideUp {
		from {
			opacity: 0;
			transform: translateY(20px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
	
	.logo-header {
		text-align: center;
		margin-bottom: 2rem;
	}
	
	.logo {
		height: 60px;
		width: auto;
		margin: 0 auto;
		display: block;
	}
	
	.loading {
		padding: 3rem;
		text-align: center;
	}
	
	
	.user-header {
		padding: 2rem 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		gap: 1rem;
		border-bottom: 1px solid #e5e7eb;
		margin-bottom: 1.5rem;
	}
	
	.avatar {
		width: 80px;
		height: 80px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		color: white;
		font-size: 1.75rem;
		font-weight: 600;
		box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
	}
	
	.user-info h2 {
		margin: 0;
		font-size: 1.5rem;
		color: #1f2937;
	}
	
	.user-info .email {
		margin: 0.25rem 0;
		color: #6b7280;
		font-size: 0.875rem;
	}
	
	.role-badge {
		display: inline-block;
		padding: 0.25rem 0.75rem;
		border-radius: 9999px;
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
		margin-top: 0.25rem;
	}
	
	.role-badge.admin {
		background: rgba(24, 154, 180, 0.1);
		color: #189AB4;
	}
	
	.role-badge.user {
		background: rgba(99, 102, 241, 0.1);
		color: #6366f1;
	}
	
	.alert {
		margin: 1rem 2rem;
		padding: 0.75rem 1rem;
		border-radius: 6px;
		font-size: 0.875rem;
	}
	
	.alert-error {
		background: #fee2e2;
		color: #991b1b;
		border: 1px solid #fca5a5;
	}
	
	.alert-success {
		background: #d1fae5;
		color: #065f46;
		border: 1px solid #6ee7b7;
	}
	
	.actions-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 0.75rem;
		padding: 0;
	}
	
	.action-card {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
		padding: 1rem;
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 8px;
		color: #374151;
		text-decoration: none;
		font-size: 0.813rem;
		font-weight: 500;
		transition: all 0.2s;
		cursor: pointer;
	}
	
	.action-card:hover {
		background: #f3f4f6;
		border-color: #189AB4;
		transform: translateY(-1px);
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
	}
	
	.action-card.logout {
		color: #ef4444;
	}
	
	.action-card.logout:hover {
		background: rgba(239, 68, 68, 0.05);
		border-color: #ef4444;
	}
	
	.form-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
	}
	
	.form-group {
		margin-bottom: 1rem;
	}
	
	.form-group:last-child {
		margin-bottom: 0;
	}
	
	.form-group label {
		display: block;
		margin-bottom: 0.5rem;
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
	}
	
	.form-group input {
		width: 100%;
		padding: 0.625rem 0.875rem;
		border: 1px solid #d1d5db;
		border-radius: 8px;
		font-size: 0.875rem;
		background: white;
		color: #1f2937;
		transition: all 0.2s;
	}
	
	.form-group input:focus {
		outline: none;
		border-color: #189AB4;
		box-shadow: 0 0 0 3px rgba(24, 154, 180, 0.1);
	}
	
	.form-group input.disabled,
	.form-group input:disabled {
		background: #f9fafb;
		color: #9ca3af;
		cursor: not-allowed;
	}
	
	.btn {
		padding: 0.5rem 1rem;
		border: none;
		border-radius: 6px;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
	}
	
	.btn-primary {
		background: #189AB4;
		color: white;
	}
	
	.btn-primary:hover:not(:disabled) {
		background: #147b91;
	}
	
	.btn-secondary {
		background: #f3f4f6;
		color: #1f2937;
		border: 1px solid #e5e7eb;
	}
	
	.btn-secondary:hover {
		background: #e5e7eb;
	}
	
	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	
	/* Storage Modal Styles */
	.storage-modal {
		max-width: 600px;
	}
	
	.storage-overview {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
		margin-bottom: 1.5rem;
	}
	
	.storage-stat-card {
		display: flex;
		gap: 0.75rem;
		padding: 1rem;
		background: #f9fafb;
		border-radius: 8px;
		border: 1px solid #e5e7eb;
	}
	
	.stat-icon {
		width: 40px;
		height: 40px;
		border-radius: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}
	
	.stat-icon.storage-icon {
		background: rgba(59, 130, 246, 0.1);
		color: #3b82f6;
	}
	
	.stat-icon.bandwidth-icon {
		background: rgba(139, 92, 246, 0.1);
		color: #8b5cf6;
	}
	
	.stat-details {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	
	.stat-label {
		font-size: 0.75rem;
		color: #6b7280;
		text-transform: uppercase;
	}
	
	.stat-value {
		font-size: 1.25rem;
		font-weight: 600;
		color: #1f2937;
	}
	
	.progress-bar {
		height: 6px;
		background: #e5e7eb;
		border-radius: 3px;
		overflow: hidden;
		margin: 0.25rem 0;
	}
	
	.progress-fill {
		height: 100%;
		background: linear-gradient(to right, #3b82f6, #2563eb);
		border-radius: 3px;
		transition: width 0.3s ease;
	}
	
	.progress-fill.bandwidth {
		background: linear-gradient(to right, #8b5cf6, #7c3aed);
	}
	
	.stat-detail {
		font-size: 0.75rem;
		color: #9ca3af;
	}
	
	.storage-details {
		margin-bottom: 1.5rem;
	}
	
	.storage-details h4,
	.recent-activity h4 {
		margin: 0 0 0.75rem 0;
		font-size: 0.875rem;
		font-weight: 600;
		color: #374151;
	}
	
	.detail-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.75rem;
	}
	
	.detail-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.5rem;
		background: #f9fafb;
		border-radius: 6px;
		font-size: 0.813rem;
	}
	
	.detail-label {
		color: #6b7280;
	}
	
	.detail-value {
		font-weight: 600;
		color: #1f2937;
	}
	
	.activity-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	
	.activity-item {
		display: flex;
		gap: 0.75rem;
		padding: 0.625rem;
		border-radius: 6px;
		border: 1px solid #e5e7eb;
		transition: background 0.2s;
	}
	
	.activity-item:hover {
		background: #f9fafb;
	}
	
	.activity-icon {
		width: 28px;
		height: 28px;
		border-radius: 6px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #f3f4f6;
	}
	
	.activity-details {
		flex: 1;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	
	.activity-action {
		font-size: 0.813rem;
		font-weight: 500;
		color: #374151;
		text-transform: capitalize;
	}
	
	.activity-time {
		font-size: 0.75rem;
		color: #9ca3af;
	}
	
	.storage-tips {
		margin-top: 1.5rem;
		padding: 1rem;
		background: #f0f9ff;
		border: 1px solid #bae6fd;
		border-radius: 8px;
	}
	
	.storage-tips h4 {
		margin: 0 0 0.75rem 0;
		font-size: 0.875rem;
		font-weight: 600;
		color: #0369a1;
	}
	
	.storage-tips ul {
		margin: 0;
		padding-left: 1.25rem;
		list-style-type: disc;
	}
	
	.storage-tips li {
		margin: 0.5rem 0;
		font-size: 0.813rem;
		color: #0c4a6e;
		line-height: 1.5;
	}
	
	.storage-tips li.warning {
		color: #dc2626;
		font-weight: 500;
	}
	
	.storage-tips a {
		color: #0891b2;
		text-decoration: underline;
		font-weight: 500;
	}
	
	.storage-tips a:hover {
		color: #0e7490;
	}
	
	.detail-item.warning {
		background: #fef2f2;
		border: 1px solid #fecaca;
	}
	
	.detail-item.warning .detail-label {
		color: #b91c1c;
		font-weight: 500;
	}
	
	.detail-item.warning .detail-value {
		color: #dc2626;
	}
	
	/* API Keys Modal Styles */
	.api-keys-modal {
		max-width: 600px;
	}

	.created-key-alert {
		background: #ecfdf5;
		border: 1px solid #6ee7b7;
		border-radius: 8px;
		padding: 1rem;
		margin-bottom: 1.5rem;
	}

	.created-key-alert .alert-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: #065f46;
		margin-bottom: 0.5rem;
	}

	.created-key-alert .alert-warning {
		color: #047857;
		font-size: 0.813rem;
		margin: 0 0 0.75rem 0;
	}

	.key-display {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		background: white;
		border: 1px solid #d1fae5;
		border-radius: 6px;
		padding: 0.5rem 0.75rem;
	}

	.key-display code {
		flex: 1;
		font-family: monospace;
		font-size: 0.813rem;
		color: #065f46;
		word-break: break-all;
	}

	.copy-btn {
		background: none;
		border: none;
		color: #6b7280;
		cursor: pointer;
		padding: 0.25rem;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 4px;
		transition: all 0.2s;
	}

	.copy-btn:hover {
		background: #f3f4f6;
		color: #1f2937;
	}

	.api-keys-section {
		margin-bottom: 1.5rem;
	}

	.section-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 1rem;
		font-weight: 600;
		color: #374151;
	}

	.btn-small {
		padding: 0.375rem 0.75rem;
		font-size: 0.75rem;
	}

	.loading-state {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		padding: 2rem;
		color: #6b7280;
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
		border-radius: 8px;
		transition: all 0.2s;
	}

	.api-key-item:hover {
		border-color: #d1d5db;
	}

	.key-info {
		flex: 1;
		min-width: 0;
	}

	.key-name {
		font-weight: 600;
		color: #1f2937;
		margin-bottom: 0.25rem;
	}

	.key-details {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.75rem;
		color: #6b7280;
		flex-wrap: wrap;
	}

	.key-prefix {
		font-family: monospace;
		background: #e5e7eb;
		padding: 0.125rem 0.375rem;
		border-radius: 4px;
	}

	.key-separator {
		color: #d1d5db;
	}

	.key-last-used {
		font-size: 0.75rem;
		color: #9ca3af;
		margin-top: 0.25rem;
	}

	.key-expiry {
		font-size: 0.75rem;
		color: #6b7280;
		margin-top: 0.25rem;
	}

	.key-expiry.expired {
		color: #dc2626;
	}

	.revoke-btn {
		background: none;
		border: none;
		color: #9ca3af;
		cursor: pointer;
		padding: 0.5rem;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 6px;
		transition: all 0.2s;
	}

	.revoke-btn:hover {
		background: #fee2e2;
		color: #dc2626;
	}

	.api-usage-info {
		background: #f0f9ff;
		border: 1px solid #bae6fd;
		border-radius: 8px;
		padding: 1rem;
	}

	.api-usage-info h4 {
		margin: 0 0 0.5rem 0;
		font-size: 0.875rem;
		font-weight: 600;
		color: #0369a1;
	}

	.api-usage-info p {
		margin: 0 0 0.75rem 0;
		font-size: 0.813rem;
		color: #0c4a6e;
	}

	.code-block {
		display: block;
		background: white;
		border: 1px solid #bae6fd;
		border-radius: 6px;
		padding: 0.625rem 0.875rem;
		font-family: monospace;
		font-size: 0.75rem;
		color: #0369a1;
		word-break: break-all;
	}

	.form-hint {
		display: block;
		font-size: 0.75rem;
		color: #9ca3af;
		margin-top: 0.375rem;
	}

	@media (max-width: 640px) {
		.profile-page {
			padding: 1rem;
		}

		.actions-grid {
			grid-template-columns: 1fr;
		}

		.form-row {
			grid-template-columns: 1fr;
		}

		.storage-overview {
			grid-template-columns: 1fr;
		}

		.detail-grid {
			grid-template-columns: 1fr;
		}

		.key-details {
			flex-direction: column;
			align-items: flex-start;
			gap: 0.25rem;
		}

		.key-separator {
			display: none;
		}
	}

</style>