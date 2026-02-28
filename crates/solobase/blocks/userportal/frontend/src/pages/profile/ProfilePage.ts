import { html, api, LoadingSpinner, EmptyState, Modal, ConfirmDialog, checkAuth } from '@solobase/ui';
import { formatBytes } from '@solobase/utils';
import { useState, useEffect } from 'preact/hooks';
import {
	Lock, LogOut, Shield, Save, Settings,
	Package, HardDrive, TrendingUp,
	Activity, Share2, Download, Upload, ArrowLeft,
	Key, Copy, Trash2, Plus
} from 'lucide-preact';

interface ProfileSection {
	extension: string;
	name: string;
	displayName: string;
	icon: string;
	priority: number;
	type: 'card' | 'modal' | 'page';
	visibilityEndpoint?: string;
	route?: string;
	dataEndpoint?: string;
}

interface SettingResponse {
	value?: string | boolean;
}

function getInitials(email: string | undefined | null): string {
	if (!email) return '??';
	return email.substring(0, 2).toUpperCase();
}

function getAvatarColor(email: string | undefined | null): string {
	const colors = ['#3b82f6', '#ef4444', '#10b981', '#f59e0b', '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16'];
	if (!email) return colors[0];
	let hash = 0;
	for (let i = 0; i < email.length; i++) {
		hash = email.charCodeAt(i) + ((hash << 5) - hash);
	}
	return colors[Math.abs(hash) % colors.length];
}

function formatDate(dateString: string | null): string {
	if (!dateString) return 'Never';
	return new Date(dateString).toLocaleDateString(undefined, {
		year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit'
	});
}

function getActionIcon(action: string) {
	switch (action) {
		case 'download': return Download;
		case 'upload': return Upload;
		case 'share': return Share2;
		default: return Activity;
	}
}

function getActionColor(action: string): string {
	switch (action) {
		case 'download': return 'color: #9333ea';
		case 'upload': return 'color: #16a34a';
		case 'share': return 'color: #0891b2';
		default: return 'color: #6b7280';
	}
}

function getSectionIcon(iconName: string) {
	switch (iconName) {
		case 'Package': return Package;
		case 'HardDrive': return HardDrive;
		default: return Activity;
	}
}

export function ProfilePage() {
	const [user, setUser] = useState<any>(null);
	const [roles, setRoles] = useState<string[]>([]);
	const [loading, setLoading] = useState(true);
	const [saving, setSaving] = useState(false);
	const [error, setError] = useState('');
	const [successMessage, setSuccessMessage] = useState('');

	// Modals
	const [showAccountSettings, setShowAccountSettings] = useState(false);
	const [showPasswordChange, setShowPasswordChange] = useState(false);
	const [showStorageModal, setShowStorageModal] = useState(false);
	const [showAPIKeysModal, setShowAPIKeysModal] = useState(false);
	const [showCreateKeyModal, setShowCreateKeyModal] = useState(false);

	// Profile sections from extensions
	const [profileSections, setProfileSections] = useState<ProfileSection[]>([]);

	// Storage data
	const [storageStats, setStorageStats] = useState<any>(null);
	const [storageQuota, setStorageQuota] = useState<any>(null);
	const [recentActivity, setRecentActivity] = useState<any[]>([]);

	// API Keys data
	const [apiKeys, setApiKeys] = useState<any[]>([]);
	const [apiKeysLoading, setApiKeysLoading] = useState(false);
	const [newKeyName, setNewKeyName] = useState('');
	const [newKeyExpiry, setNewKeyExpiry] = useState<string | null>(null);
	const [createdKey, setCreatedKey] = useState<string | null>(null);
	const [keyCreating, setKeyCreating] = useState(false);
	const [keyError, setKeyError] = useState('');
	const [showRevokeConfirm, setShowRevokeConfirm] = useState(false);
	const [keyToRevoke, setKeyToRevoke] = useState<{ id: string; name: string } | null>(null);

	// Profile form data
	const [profileForm, setProfileForm] = useState({
		firstName: '', lastName: '', displayName: '', email: '', phone: '', location: ''
	});

	// Password change form
	const [passwordForm, setPasswordForm] = useState({
		currentPassword: '', newPassword: '', confirmPassword: ''
	});
	const [passwordError, setPasswordError] = useState('');
	const [passwordSuccess, setPasswordSuccess] = useState('');

	function updateProfileField(field: string, value: string) {
		setProfileForm(prev => ({ ...prev, [field]: value }));
	}

	function updatePasswordField(field: string, value: string) {
		setPasswordForm(prev => ({ ...prev, [field]: value }));
	}

	async function loadProfileSections() {
		try {
			const sections = await api.get<ProfileSection[]>('/profile/sections');
			if (!Array.isArray(sections)) {
				setProfileSections([]);
				return;
			}

			const visibleSections: ProfileSection[] = [];
			for (const section of sections) {
				if (section.visibilityEndpoint) {
					try {
						const response = await api.get<SettingResponse>(section.visibilityEndpoint);
						if (response && (response.value === 'true' || response.value === true)) {
							visibleSections.push(section);
						}
					} catch { /* skip */ }
				} else {
					visibleSections.push(section);
				}
			}
			setProfileSections(visibleSections.sort((a, b) => a.priority - b.priority));
		} catch {
			setProfileSections([]);
		}
	}

	useEffect(() => {
		(async () => {
			const authed = await checkAuth();
			if (!authed) {
				window.location.href = '/login';
				return;
			}

			try {
				const response = await api.get<{ user: any; roles?: string[] }>('/auth/me');
				const u = response.user;
				setUser(u);
				setRoles(response.roles || []);
				setProfileForm({
					firstName: u.firstName || '',
					lastName: u.lastName || '',
					displayName: u.displayName || u.email?.split('@')[0] || '',
					email: u.email || '',
					phone: u.phone || '',
					location: u.location || ''
				});
				await loadProfileSections();
			} catch (err: any) {
				setError(err.message || 'Failed to load user profile');
			}
			setLoading(false);
		})();
	}, []);

	async function saveProfile() {
		setSaving(true);
		setError('');
		setSuccessMessage('');

		try {
			await api.patch('/auth/me', {
				firstName: profileForm.firstName,
				lastName: profileForm.lastName,
				displayName: profileForm.displayName,
				phone: profileForm.phone,
				location: profileForm.location
			});

			setSuccessMessage('Profile updated successfully');
			const updatedUser = {
				...user,
				firstName: profileForm.firstName,
				lastName: profileForm.lastName,
				displayName: profileForm.displayName,
				phone: profileForm.phone,
				location: profileForm.location
			};
			setUser(updatedUser);
			setShowAccountSettings(false);
			setTimeout(() => setSuccessMessage(''), 3000);
		} catch (err: any) {
			setError(err.message || 'Failed to update profile');
		} finally {
			setSaving(false);
		}
	}

	async function changePassword() {
		setPasswordError('');
		setPasswordSuccess('');

		if (passwordForm.newPassword !== passwordForm.confirmPassword) {
			setPasswordError('New passwords do not match');
			return;
		}
		if (passwordForm.newPassword.length < 8) {
			setPasswordError('Password must be at least 8 characters');
			return;
		}

		try {
			await api.post('/auth/change-password', {
				currentPassword: passwordForm.currentPassword,
				newPassword: passwordForm.newPassword
			});
			setPasswordSuccess('Password changed successfully');
			setShowPasswordChange(false);
			setPasswordForm({ currentPassword: '', newPassword: '', confirmPassword: '' });
			setTimeout(() => setPasswordSuccess(''), 3000);
		} catch (err: any) {
			setPasswordError(err.message || 'Failed to change password');
		}
	}

	async function handleLogout() {
		try {
			await api.post('/auth/logout', {});
		} catch { /* ignore */ }
		window.location.href = '/login';
	}

	async function loadStorageData() {
		try {
			const [statsRes, quotaRes, logsRes] = await Promise.all([
				api.get('/ext/cloudstorage/stats').catch(() => null),
				api.get('/ext/cloudstorage/quotas/user').catch(() => null),
				api.get<any[]>('/ext/cloudstorage/access-logs?user_id=me&limit=10').catch(() => [])
			]);
			setStorageStats(statsRes);
			setStorageQuota(quotaRes);
			setRecentActivity(logsRes || []);
		} catch { /* ignore */ }
	}

	async function openStorageModal() {
		setShowStorageModal(true);
		if (!storageStats) await loadStorageData();
	}

	async function loadAPIKeys() {
		setApiKeysLoading(true);
		setKeyError('');
		try {
			const response = await api.get('/auth/api-keys');
			setApiKeys(Array.isArray(response) ? response : []);
		} catch (err: any) {
			setKeyError(err.message || 'Failed to load API keys');
			setApiKeys([]);
		} finally {
			setApiKeysLoading(false);
		}
	}

	async function openAPIKeysModal() {
		setShowAPIKeysModal(true);
		setCreatedKey(null);
		await loadAPIKeys();
	}

	function openCreateKeyModal() {
		setShowCreateKeyModal(true);
		setNewKeyName('');
		setNewKeyExpiry(null);
		setKeyError('');
	}

	async function createAPIKey() {
		if (!newKeyName.trim()) {
			setKeyError('Please enter a name for the API key');
			return;
		}

		setKeyCreating(true);
		setKeyError('');

		try {
			const payload: any = { name: newKeyName.trim() };
			if (newKeyExpiry) {
				payload.expiresAt = new Date(newKeyExpiry).toISOString();
			}
			const response = await api.post<{ key: string }>('/auth/api-keys', payload);
			setCreatedKey(response.key);
			setShowCreateKeyModal(false);
			await loadAPIKeys();
		} catch (err: any) {
			setKeyError(err.message || 'Failed to create API key');
		} finally {
			setKeyCreating(false);
		}
	}

	function revokeAPIKey(keyId: string, keyName: string) {
		setKeyToRevoke({ id: keyId, name: keyName });
		setShowRevokeConfirm(true);
	}

	async function confirmRevokeKey() {
		if (!keyToRevoke) return;
		setShowRevokeConfirm(false);

		try {
			await api.delete(`/auth/api-keys/${keyToRevoke.id}`);
			await loadAPIKeys();
		} catch (err: any) {
			setKeyError(err.message || 'Failed to revoke API key');
		}
		setKeyToRevoke(null);
	}

	async function copyToClipboard(text: string) {
		try { await navigator.clipboard.writeText(text); } catch { /* ignore */ }
	}

	function handleSectionClick(section: ProfileSection) {
		if (section.type === 'card' && section.route) {
			window.location.href = section.route;
		} else if (section.type === 'modal' && section.extension === 'cloudstorage') {
			openStorageModal();
		}
	}

	return html`
		<div class="profile-page">
			<div class="profile-container">
				<a href="/" class="back-button">
					<${ArrowLeft} size=${18} />
					<span>Back to Home</span>
				</a>

				<div class="profile-card">
					<div class="logo-header">
						<img src="/logo_long.png" alt="Solobase" class="logo" />
					</div>

					${loading ? html`
						<div class="loading">
							<${LoadingSpinner} size="lg" />
							<p>Loading profile...</p>
						</div>
					` : user ? html`
						<div class="user-header">
							<div class="avatar" style=${'background-color: ' + getAvatarColor(user.email)}>
								${getInitials(user.email)}
							</div>
							<div class="user-info">
								<h2>${profileForm.displayName || user.email}</h2>
								<p class="email">${user.email}</p>
							</div>
						</div>

						${successMessage ? html`<div class="alert alert-success">${successMessage}</div>` : null}
						${passwordSuccess ? html`<div class="alert alert-success">${passwordSuccess}</div>` : null}

						<div class="actions-grid">
							${profileSections.map(section => html`
								<button
									class="action-card"
									key=${section.name}
									onClick=${() => handleSectionClick(section)}
								>
									<${getSectionIcon(section.icon)} size=${24} />
									<span>${section.displayName}</span>
								</button>
							`)}

							<button class="action-card" onClick=${() => setShowAccountSettings(true)}>
								<${Settings} size=${24} />
								<span>Account Settings</span>
							</button>

							<button class="action-card" onClick=${() => setShowPasswordChange(true)}>
								<${Lock} size=${24} />
								<span>Change Password</span>
							</button>

							<button class="action-card" onClick=${openAPIKeysModal}>
								<${Key} size=${24} />
								<span>API Keys</span>
							</button>

							${roles.includes('admin') ? html`
								<a href="/admin" class="action-card">
									<${Shield} size=${24} />
									<span>Admin Dashboard</span>
								</a>
							` : null}

							<button class="action-card logout" onClick=${handleLogout}>
								<${LogOut} size=${24} />
								<span>Logout</span>
							</button>
						</div>
					` : null}
				</div>
			</div>
		</div>

		<!-- Account Settings Modal -->
		<${Modal}
			show=${showAccountSettings}
			title="Account Settings"
			onClose=${() => setShowAccountSettings(false)}
			footer=${html`
				<button class="btn btn-secondary" onClick=${() => setShowAccountSettings(false)}>Cancel</button>
				<button class="btn btn-primary" onClick=${saveProfile} disabled=${saving}>
					${saving ? html`<${LoadingSpinner} size="sm" color="white" /> Saving...` : html`<${Save} size=${16} /> Save Changes`}
				</button>
			`}
		>
			${error ? html`<div class="alert alert-error">${error}</div>` : null}
			<div class="form-row">
				<div class="form-group">
					<label for="firstName">First Name</label>
					<input type="text" id="firstName" value=${profileForm.firstName} onInput=${(e: Event) => updateProfileField('firstName', (e.target as HTMLInputElement).value)} placeholder="Enter first name" />
				</div>
				<div class="form-group">
					<label for="lastName">Last Name</label>
					<input type="text" id="lastName" value=${profileForm.lastName} onInput=${(e: Event) => updateProfileField('lastName', (e.target as HTMLInputElement).value)} placeholder="Enter last name" />
				</div>
			</div>
			<div class="form-group">
				<label for="displayName">Display Name</label>
				<input type="text" id="displayName" value=${profileForm.displayName} onInput=${(e: Event) => updateProfileField('displayName', (e.target as HTMLInputElement).value)} placeholder="Enter display name" />
			</div>
			<div class="form-group">
				<label for="profileEmail">Email</label>
				<input type="email" id="profileEmail" value=${profileForm.email} disabled class="disabled" />
			</div>
			<div class="form-group">
				<label for="phone">Phone</label>
				<input type="tel" id="phone" value=${profileForm.phone} onInput=${(e: Event) => updateProfileField('phone', (e.target as HTMLInputElement).value)} placeholder="Enter phone number" />
			</div>
			<div class="form-group">
				<label for="location">Location</label>
				<input type="text" id="location" value=${profileForm.location} onInput=${(e: Event) => updateProfileField('location', (e.target as HTMLInputElement).value)} placeholder="Enter location" />
			</div>
		<//>

		<!-- Change Password Modal -->
		<${Modal}
			show=${showPasswordChange}
			title="Change Password"
			onClose=${() => {
				setShowPasswordChange(false);
				setPasswordForm({ currentPassword: '', newPassword: '', confirmPassword: '' });
				setPasswordError('');
			}}
			footer=${html`
				<button class="btn btn-secondary" onClick=${() => {
					setShowPasswordChange(false);
					setPasswordForm({ currentPassword: '', newPassword: '', confirmPassword: '' });
					setPasswordError('');
				}}>Cancel</button>
				<button class="btn btn-primary" onClick=${changePassword}>Change Password</button>
			`}
		>
			${passwordError ? html`<div class="alert alert-error">${passwordError}</div>` : null}
			<div class="form-group">
				<label for="currentPassword">Current Password</label>
				<input type="password" id="currentPassword" value=${passwordForm.currentPassword} onInput=${(e: Event) => updatePasswordField('currentPassword', (e.target as HTMLInputElement).value)} placeholder="Enter current password" />
			</div>
			<div class="form-group">
				<label for="newPassword">New Password</label>
				<input type="password" id="newPassword" value=${passwordForm.newPassword} onInput=${(e: Event) => updatePasswordField('newPassword', (e.target as HTMLInputElement).value)} placeholder="Enter new password (min 8 characters)" />
			</div>
			<div class="form-group">
				<label for="confirmNewPassword">Confirm New Password</label>
				<input type="password" id="confirmNewPassword" value=${passwordForm.confirmPassword} onInput=${(e: Event) => updatePasswordField('confirmPassword', (e.target as HTMLInputElement).value)} placeholder="Confirm new password" />
			</div>
		<//>

		<!-- Storage Usage Modal -->
		<${Modal}
			show=${showStorageModal}
			title="Storage Usage"
			maxWidth="600px"
			onClose=${() => setShowStorageModal(false)}
			footer=${html`
				${user && roles.includes('admin') ? html`
					<a href="/admin/extensions/cloudstorage" class="btn btn-secondary">Extension Settings</a>
					<a href="/admin/storage" class="btn btn-secondary">Manage Files</a>
				` : null}
				<button class="btn btn-primary" onClick=${() => setShowStorageModal(false)}>Close</button>
			`}
		>
			<div class="storage-overview">
				<div class="storage-stat-card">
					<div class="stat-icon storage-icon"><${HardDrive} size=${20} /></div>
					<div class="stat-details">
						<span class="stat-label">Storage Used</span>
						<span class="stat-value">${storageQuota ? formatBytes(storageQuota.storageUsed || 0) : 'Loading...'}</span>
						${storageQuota && storageQuota.maxStorageBytes ? html`
							<div class="progress-bar">
								<div class="progress-fill" style=${'width: ' + Math.min((storageQuota.storageUsed / storageQuota.maxStorageBytes) * 100, 100) + '%'}></div>
							</div>
							<span class="stat-detail">of ${formatBytes(storageQuota.maxStorageBytes)} available</span>
						` : null}
					</div>
				</div>
				<div class="storage-stat-card">
					<div class="stat-icon bandwidth-icon"><${TrendingUp} size=${20} /></div>
					<div class="stat-details">
						<span class="stat-label">Bandwidth Used</span>
						<span class="stat-value">${storageQuota ? formatBytes(storageQuota.bandwidthUsed || 0) : 'Loading...'}</span>
						${storageQuota && storageQuota.maxBandwidthBytes ? html`
							<div class="progress-bar">
								<div class="progress-fill bandwidth" style=${'width: ' + Math.min((storageQuota.bandwidthUsed / storageQuota.maxBandwidthBytes) * 100, 100) + '%'}></div>
							</div>
							<span class="stat-detail">of ${formatBytes(storageQuota.maxBandwidthBytes)} this month</span>
						` : null}
					</div>
				</div>
			</div>

			${storageStats || storageQuota ? html`
				<div class="storage-details">
					<h4>Storage Details</h4>
					<div class="detail-grid">
						<div class="detail-item">
							<span class="detail-label">Total Files:</span>
							<span class="detail-value">${storageStats?.storage?.totalObjects || 0}</span>
						</div>
						<div class="detail-item">
							<span class="detail-label">Shared Files:</span>
							<span class="detail-value">${storageStats?.shares?.totalShares || 0}</span>
						</div>
						${storageQuota?.resetBandwidthAt ? html`
							<div class="detail-item">
								<span class="detail-label">Bandwidth Resets:</span>
								<span class="detail-value">${new Date(storageQuota.resetBandwidthAt).toLocaleDateString()}</span>
							</div>
						` : null}
						${storageQuota && storageQuota.storageUsed > storageQuota.maxStorageBytes * 0.9 ? html`
							<div class="detail-item warning">
								<span class="detail-label">Storage Warning:</span>
								<span class="detail-value">Over 90% used</span>
							</div>
						` : null}
					</div>
				</div>
			` : null}

			<div class="storage-tips">
				<h4>Storage Information</h4>
				<ul>
					<li>Your storage quota is managed by your administrator</li>
					<li>Contact your admin if you need more storage space</li>
					${storageQuota && storageQuota.storageUsed > storageQuota.maxStorageBytes * 0.75 ? html`
						<li class="warning">Your storage is almost full - please contact your administrator</li>
					` : null}
				</ul>
			</div>

			${recentActivity && recentActivity.length > 0 ? html`
				<div class="recent-activity">
					<h4>Recent Activity</h4>
					<div class="activity-list">
						${recentActivity.slice(0, 5).map(activity => html`
							<div class="activity-item">
								<div class="activity-icon" style=${getActionColor(activity.action)}>
									<${getActionIcon(activity.action)} size=${14} />
								</div>
								<div class="activity-details">
									<span class="activity-action">${activity.action}</span>
									<span class="activity-time">${new Date(activity.createdAt).toLocaleString()}</span>
								</div>
							</div>
						`)}
					</div>
				</div>
			` : null}
		<//>

		<!-- API Keys Modal -->
		<${Modal}
			show=${showAPIKeysModal}
			title="API Keys"
			maxWidth="600px"
			onClose=${() => setShowAPIKeysModal(false)}
			footer=${html`
				<button class="btn btn-primary" onClick=${() => setShowAPIKeysModal(false)}>Close</button>
			`}
		>
			${keyError ? html`<div class="alert alert-error">${keyError}</div>` : null}

			${createdKey ? html`
				<div class="created-key-alert">
					<div class="alert-header">
						<${Key} size=${16} />
						<strong>API Key Created!</strong>
					</div>
					<p class="alert-warning">Copy this key now. You won't be able to see it again!</p>
					<div class="key-display">
						<code>${createdKey}</code>
						<button class="copy-btn" onClick=${() => copyToClipboard(createdKey || '')} title="Copy to clipboard">
							<${Copy} size=${16} />
						</button>
					</div>
				</div>
			` : null}

			<div class="api-keys-section">
				<div class="section-header">
					<span>Your API Keys</span>
					<button class="btn btn-small btn-primary" onClick=${openCreateKeyModal}>
						<${Plus} size=${14} />
						Create Key
					</button>
				</div>

				${apiKeysLoading ? html`
					<div class="loading-state">
						<${LoadingSpinner} size="sm" />
						<span>Loading API keys...</span>
					</div>
				` : !apiKeys || apiKeys.length === 0 ? html`
					<${EmptyState} icon=${Key} title="No API keys yet" description="Create an API key to access the API programmatically" />
				` : html`
					<div class="api-keys-list">
						${apiKeys.map(key => html`
							<div class="api-key-item" key=${key.id}>
								<div class="key-info">
									<div class="key-name">${key.name}</div>
									<div class="key-details">
										<span class="key-prefix" title="Key prefix">${key.keyPrefix}...</span>
										<span class="key-separator">•</span>
										<span class="key-created">Created ${formatDate(key.createdAt)}</span>
									</div>
									${key.lastUsedAt ? html`
										<div class="key-last-used">
											Last used: ${formatDate(key.lastUsedAt)}${key.lastUsedIp ? ` from ${key.lastUsedIp}` : ''}
										</div>
									` : html`<div class="key-last-used">Never used</div>`}
									${key.expiresAt ? html`
										<div class=${`key-expiry${new Date(key.expiresAt) < new Date() ? ' expired' : ''}`}>
											${new Date(key.expiresAt) < new Date() ? `Expired ${formatDate(key.expiresAt)}` : `Expires ${formatDate(key.expiresAt)}`}
										</div>
									` : null}
								</div>
								<button class="revoke-btn" onClick=${() => revokeAPIKey(key.id, key.name)} title="Revoke API key">
									<${Trash2} size=${16} />
								</button>
							</div>
						`)}
					</div>
				`}
			</div>

			<div class="api-usage-info">
				<h4>How to use API Keys</h4>
				<p>Include your API key in the Authorization header:</p>
				<code class="code-block">Authorization: Bearer sb_your_api_key_here</code>
			</div>
		<//>

		<!-- Create API Key Modal -->
		<${Modal}
			show=${showCreateKeyModal}
			title="Create API Key"
			onClose=${() => setShowCreateKeyModal(false)}
			footer=${html`
				<button class="btn btn-secondary" onClick=${() => setShowCreateKeyModal(false)}>Cancel</button>
				<button class="btn btn-primary" onClick=${createAPIKey} disabled=${keyCreating}>
					${keyCreating ? html`<${LoadingSpinner} size="sm" color="white" /> Creating...` : html`<${Key} size=${16} /> Create Key`}
				</button>
			`}
		>
			${keyError ? html`<div class="alert alert-error">${keyError}</div>` : null}
			<div class="form-group">
				<label for="keyName">Key Name</label>
				<input type="text" id="keyName" value=${newKeyName} onInput=${(e: Event) => setNewKeyName((e.target as HTMLInputElement).value)} placeholder="e.g., Production Server, CI/CD Pipeline" />
				<span class="form-hint">A descriptive name to identify this key</span>
			</div>
			<div class="form-group">
				<label for="keyExpiry">Expiration (Optional)</label>
				<input type="datetime-local" id="keyExpiry" value=${newKeyExpiry || ''} onInput=${(e: Event) => setNewKeyExpiry((e.target as HTMLInputElement).value || null)} />
				<span class="form-hint">Leave empty for a key that never expires</span>
			</div>
		<//>

		<!-- Revoke Confirmation -->
		<${ConfirmDialog}
			show=${showRevokeConfirm}
			title="Revoke API Key"
			message=${`Are you sure you want to revoke the API key "${keyToRevoke?.name}"? This action cannot be undone.`}
			confirmText="Revoke"
			variant="danger"
			onConfirm=${confirmRevokeKey}
			onCancel=${() => { setShowRevokeConfirm(false); setKeyToRevoke(null); }}
		/>

		<style>
			.profile-page { min-height: 100vh; display: flex; align-items: center; justify-content: center; background: #f0f0f0; padding: 1rem; }
			.profile-container { width: 100%; max-width: 500px; position: relative; }
			.back-button { display: flex; align-items: center; gap: 0.375rem; padding: 0.375rem 0.625rem; margin-bottom: 0.75rem; background: white; border: 1px solid #e5e7eb; border-radius: 0.375rem; color: #374151; font-size: 0.813rem; font-weight: 500; text-decoration: none; transition: all 0.2s; width: fit-content; }
			.back-button:hover { background: #f9fafb; border-color: #189AB4; transform: translateX(-2px); }
			.profile-card { background: white; border: 1px solid #e2e8f0; border-radius: 12px; padding: 2rem; animation: slideUp 0.4s ease-out; }
			@keyframes slideUp { from { opacity: 0; transform: translateY(20px); } to { opacity: 1; transform: translateY(0); } }
			.logo-header { text-align: center; margin-bottom: 2rem; }
			.logo { height: 60px; width: auto; margin: 0 auto; display: block; }
			.loading { padding: 3rem; text-align: center; }
			.user-header { padding: 2rem 0; display: flex; flex-direction: column; align-items: center; text-align: center; gap: 1rem; border-bottom: 1px solid #e5e7eb; margin-bottom: 1.5rem; }
			.avatar { width: 80px; height: 80px; border-radius: 50%; display: flex; align-items: center; justify-content: center; color: white; font-size: 1.75rem; font-weight: 600; box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1); }
			.user-info h2 { margin: 0; font-size: 1.5rem; color: #1f2937; }
			.user-info .email { margin: 0.25rem 0; color: #6b7280; font-size: 0.875rem; }
			.alert { margin: 1rem 0; padding: 0.75rem 1rem; border-radius: 6px; font-size: 0.875rem; }
			.alert-error { background: #fee2e2; color: #991b1b; border: 1px solid #fca5a5; }
			.alert-success { background: #d1fae5; color: #065f46; border: 1px solid #6ee7b7; }
			.actions-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 0.75rem; }
			.action-card { display: flex; flex-direction: column; align-items: center; gap: 0.5rem; padding: 1rem; background: #f9fafb; border: 1px solid #e5e7eb; border-radius: 8px; color: #374151; text-decoration: none; font-size: 0.813rem; font-weight: 500; transition: all 0.2s; cursor: pointer; }
			.action-card:hover { background: #f3f4f6; border-color: #189AB4; transform: translateY(-1px); box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05); }
			.action-card.logout { color: #ef4444; }
			.action-card.logout:hover { background: rgba(239, 68, 68, 0.05); border-color: #ef4444; }
			.form-row { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
			.form-group { margin-bottom: 1rem; }
			.form-group:last-child { margin-bottom: 0; }
			.form-group label { display: block; margin-bottom: 0.5rem; font-size: 0.875rem; font-weight: 500; color: #374151; }
			.form-group input { width: 100%; padding: 0.625rem 0.875rem; border: 1px solid #d1d5db; border-radius: 8px; font-size: 0.875rem; background: white; color: #1f2937; transition: all 0.2s; box-sizing: border-box; }
			.form-group input:focus { outline: none; border-color: #189AB4; box-shadow: 0 0 0 3px rgba(24, 154, 180, 0.1); }
			.form-group input.disabled, .form-group input:disabled { background: #f9fafb; color: #9ca3af; cursor: not-allowed; }
			.form-hint { display: block; font-size: 0.75rem; color: #9ca3af; margin-top: 0.375rem; }
			.btn { padding: 0.5rem 1rem; border: none; border-radius: 6px; font-size: 0.875rem; font-weight: 500; cursor: pointer; transition: all 0.2s; display: inline-flex; align-items: center; gap: 0.5rem; }
			.btn-primary { background: #189AB4; color: white; }
			.btn-primary:hover:not(:disabled) { background: #147b91; }
			.btn-secondary { background: #f3f4f6; color: #1f2937; border: 1px solid #e5e7eb; }
			.btn-secondary:hover { background: #e5e7eb; }
			.btn:disabled { opacity: 0.5; cursor: not-allowed; }
			.btn-small { padding: 0.375rem 0.75rem; font-size: 0.75rem; }

			/* Storage Modal */
			.storage-overview { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; margin-bottom: 1.5rem; }
			.storage-stat-card { display: flex; gap: 0.75rem; padding: 1rem; background: #f9fafb; border-radius: 8px; border: 1px solid #e5e7eb; }
			.stat-icon { width: 40px; height: 40px; border-radius: 8px; display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
			.stat-icon.storage-icon { background: rgba(59, 130, 246, 0.1); color: #3b82f6; }
			.stat-icon.bandwidth-icon { background: rgba(139, 92, 246, 0.1); color: #8b5cf6; }
			.stat-details { flex: 1; display: flex; flex-direction: column; gap: 0.25rem; }
			.stat-label { font-size: 0.75rem; color: #6b7280; text-transform: uppercase; }
			.stat-value { font-size: 1.25rem; font-weight: 600; color: #1f2937; }
			.progress-bar { height: 6px; background: #e5e7eb; border-radius: 3px; overflow: hidden; margin: 0.25rem 0; }
			.progress-fill { height: 100%; background: linear-gradient(to right, #3b82f6, #2563eb); border-radius: 3px; transition: width 0.3s ease; }
			.progress-fill.bandwidth { background: linear-gradient(to right, #8b5cf6, #7c3aed); }
			.stat-detail { font-size: 0.75rem; color: #9ca3af; }
			.storage-details { margin-bottom: 1.5rem; }
			.storage-details h4, .recent-activity h4 { margin: 0 0 0.75rem 0; font-size: 0.875rem; font-weight: 600; color: #374151; }
			.detail-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 0.75rem; }
			.detail-item { display: flex; justify-content: space-between; align-items: center; padding: 0.5rem; background: #f9fafb; border-radius: 6px; font-size: 0.813rem; }
			.detail-label { color: #6b7280; }
			.detail-value { font-weight: 600; color: #1f2937; }
			.detail-item.warning { background: #fef2f2; border: 1px solid #fecaca; }
			.detail-item.warning .detail-label { color: #b91c1c; font-weight: 500; }
			.detail-item.warning .detail-value { color: #dc2626; }
			.storage-tips { margin-top: 1.5rem; padding: 1rem; background: #f0f9ff; border: 1px solid #bae6fd; border-radius: 8px; }
			.storage-tips h4 { margin: 0 0 0.75rem 0; font-size: 0.875rem; font-weight: 600; color: #0369a1; }
			.storage-tips ul { margin: 0; padding-left: 1.25rem; list-style-type: disc; }
			.storage-tips li { margin: 0.5rem 0; font-size: 0.813rem; color: #0c4a6e; line-height: 1.5; }
			.storage-tips li.warning { color: #dc2626; font-weight: 500; }
			.activity-list { display: flex; flex-direction: column; gap: 0.5rem; }
			.activity-item { display: flex; gap: 0.75rem; padding: 0.625rem; border-radius: 6px; border: 1px solid #e5e7eb; transition: background 0.2s; }
			.activity-item:hover { background: #f9fafb; }
			.activity-icon { width: 28px; height: 28px; border-radius: 6px; display: flex; align-items: center; justify-content: center; background: #f3f4f6; }
			.activity-details { flex: 1; display: flex; justify-content: space-between; align-items: center; }
			.activity-action { font-size: 0.813rem; font-weight: 500; color: #374151; text-transform: capitalize; }
			.activity-time { font-size: 0.75rem; color: #9ca3af; }

			/* API Keys Modal */
			.created-key-alert { background: #ecfdf5; border: 1px solid #6ee7b7; border-radius: 8px; padding: 1rem; margin-bottom: 1.5rem; }
			.created-key-alert .alert-header { display: flex; align-items: center; gap: 0.5rem; color: #065f46; margin-bottom: 0.5rem; }
			.created-key-alert .alert-warning { color: #047857; font-size: 0.813rem; margin: 0 0 0.75rem 0; }
			.key-display { display: flex; align-items: center; gap: 0.5rem; background: white; border: 1px solid #d1fae5; border-radius: 6px; padding: 0.5rem 0.75rem; }
			.key-display code { flex: 1; font-family: monospace; font-size: 0.813rem; color: #065f46; word-break: break-all; }
			.copy-btn { background: none; border: none; color: #6b7280; cursor: pointer; padding: 0.25rem; display: flex; align-items: center; justify-content: center; border-radius: 4px; transition: all 0.2s; }
			.copy-btn:hover { background: #f3f4f6; color: #1f2937; }
			.api-keys-section { margin-bottom: 1.5rem; }
			.section-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 1rem; font-weight: 600; color: #374151; }
			.loading-state { display: flex; align-items: center; justify-content: center; gap: 0.75rem; padding: 2rem; color: #6b7280; }
			.api-keys-list { display: flex; flex-direction: column; gap: 0.75rem; }
			.api-key-item { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; padding: 1rem; background: #f9fafb; border: 1px solid #e5e7eb; border-radius: 8px; transition: all 0.2s; }
			.api-key-item:hover { border-color: #d1d5db; }
			.key-info { flex: 1; min-width: 0; }
			.key-name { font-weight: 600; color: #1f2937; margin-bottom: 0.25rem; }
			.key-details { display: flex; align-items: center; gap: 0.5rem; font-size: 0.75rem; color: #6b7280; flex-wrap: wrap; }
			.key-prefix { font-family: monospace; background: #e5e7eb; padding: 0.125rem 0.375rem; border-radius: 4px; }
			.key-separator { color: #d1d5db; }
			.key-last-used { font-size: 0.75rem; color: #9ca3af; margin-top: 0.25rem; }
			.key-expiry { font-size: 0.75rem; color: #6b7280; margin-top: 0.25rem; }
			.key-expiry.expired { color: #dc2626; }
			.revoke-btn { background: none; border: none; color: #9ca3af; cursor: pointer; padding: 0.5rem; display: flex; align-items: center; justify-content: center; border-radius: 6px; transition: all 0.2s; }
			.revoke-btn:hover { background: #fee2e2; color: #dc2626; }
			.api-usage-info { background: #f0f9ff; border: 1px solid #bae6fd; border-radius: 8px; padding: 1rem; }
			.api-usage-info h4 { margin: 0 0 0.5rem 0; font-size: 0.875rem; font-weight: 600; color: #0369a1; }
			.api-usage-info p { margin: 0 0 0.75rem 0; font-size: 0.813rem; color: #0c4a6e; }
			.code-block { display: block; background: white; border: 1px solid #bae6fd; border-radius: 6px; padding: 0.625rem 0.875rem; font-family: monospace; font-size: 0.75rem; color: #0369a1; word-break: break-all; }

			@media (max-width: 640px) {
				.profile-page { padding: 1rem; }
				.actions-grid { grid-template-columns: 1fr; }
				.form-row { grid-template-columns: 1fr; }
				.storage-overview { grid-template-columns: 1fr; }
				.detail-grid { grid-template-columns: 1fr; }
				.key-details { flex-direction: column; align-items: flex-start; gap: 0.25rem; }
				.key-separator { display: none; }
			}
		</style>
	`;
}
