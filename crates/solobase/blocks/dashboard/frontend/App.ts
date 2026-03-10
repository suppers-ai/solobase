import {
	html, api, checkAuth, isAuthenticated, authLoading, currentUser, userRoles, logout,
	LoadingSpinner, PageHeader, StatCard, EmptyState, StatusBadge, TabNavigation,
	ToastContainer, toasts, Button, Modal
} from '@solobase/ui';
import { useState, useEffect, useCallback } from 'preact/hooks';
import { Key, Settings, LogOut, CreditCard, Server, Activity, Plus, Trash2, Rocket, Shield, ExternalLink, Package } from 'lucide-preact';

// ─── Auth Guard ──────────────────────────────────────────────────────
function AuthGuard({ children }: { children: any }) {
	const [checked, setChecked] = useState(false);

	useEffect(() => {
		checkAuth().then(() => setChecked(true));
	}, []);

	if (!checked || authLoading.value) {
		return html`<div style=${{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh' }}><${LoadingSpinner} message="Loading..." /></div>`;
	}

	if (!isAuthenticated.value) {
		return html`<${LoginSignup} />`;
	}

	return children;
}

// ─── Login / Signup ──────────────────────────────────────────────────
function LoginSignup() {
	const [mode, setMode] = useState<'login' | 'signup'>('login');
	const [email, setEmail] = useState('');
	const [password, setPassword] = useState('');
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState('');

	async function handleSubmit(e: Event) {
		e.preventDefault();
		setLoading(true);
		setError('');

		try {
			if (mode === 'signup') {
				const res = await api.signup({ email, password });
				if (res.error) {
					const msg = typeof res.error === 'string' ? res.error : res.error.message;
					setError(msg);
					setLoading(false);
					return;
				}
			}
			const { login } = await import('@solobase/ui');
			const ok = await login(email, password);
			if (!ok) {
				setError('Invalid credentials');
			}
		} catch (err: any) {
			setError(err.message || 'Something went wrong');
		}
		setLoading(false);
	}

	return html`
		<div style=${{ minHeight: '100vh', display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'linear-gradient(135deg, #f0f9ff 0%, #e0f2fe 50%, #f0f9ff 100%)' }}>
			<div style=${{ width: '100%', maxWidth: '420px', padding: '2rem' }}>
				<div style=${{ textAlign: 'center', marginBottom: '2rem' }}>
					<img src="/images/logo_long.png" alt="Solobase" style=${{ height: '40px', width: 'auto', marginBottom: '1rem' }} />
					<p style=${{ fontSize: '0.875rem', color: '#64748b', marginTop: '0.25rem' }}>
						${mode === 'login' ? 'Sign in to your dashboard' : 'Create your account'}
					</p>
				</div>

				<div style=${{ background: 'white', borderRadius: '12px', padding: '1.5rem', boxShadow: '0 1px 3px rgba(0,0,0,0.1)' }}>
					${error ? html`
						<div style=${{ background: '#fef2f2', border: '1px solid #fecaca', borderRadius: '8px', padding: '0.75rem', marginBottom: '1rem', fontSize: '0.813rem', color: '#dc2626' }}>${error}</div>
					` : null}

					<form onSubmit=${handleSubmit}>
						<div style=${{ marginBottom: '1rem' }}>
							<label style=${{ display: 'block', fontSize: '0.813rem', fontWeight: 500, color: '#1e293b', marginBottom: '0.375rem' }}>Email</label>
							<input type="email" value=${email} onInput=${(e: any) => setEmail(e.target.value)} required
								placeholder="you@example.com"
								style=${{ width: '100%', padding: '0.625rem 0.75rem', border: '1px solid #e2e8f0', borderRadius: '8px', fontSize: '0.875rem', outline: 'none', boxSizing: 'border-box' }} />
						</div>
						<div style=${{ marginBottom: '1.5rem' }}>
							<label style=${{ display: 'block', fontSize: '0.813rem', fontWeight: 500, color: '#1e293b', marginBottom: '0.375rem' }}>Password</label>
							<input type="password" value=${password} onInput=${(e: any) => setPassword(e.target.value)} required
								placeholder=${mode === 'signup' ? 'Min 8 characters' : 'Enter your password'}
								style=${{ width: '100%', padding: '0.625rem 0.75rem', border: '1px solid #e2e8f0', borderRadius: '8px', fontSize: '0.875rem', outline: 'none', boxSizing: 'border-box' }} />
						</div>
						<button type="submit" disabled=${loading}
							style=${{ width: '100%', padding: '0.75rem', background: 'linear-gradient(135deg, #189AB4, #0ea5e9)', color: 'white', border: 'none', borderRadius: '8px', fontSize: '0.875rem', fontWeight: 600, cursor: loading ? 'not-allowed' : 'pointer', opacity: loading ? 0.7 : 1 }}>
							${loading ? (mode === 'signup' ? 'Creating account...' : 'Signing in...') : (mode === 'signup' ? 'Create Account' : 'Sign In')}
						</button>
					</form>

					<div style=${{ textAlign: 'center', marginTop: '1rem', fontSize: '0.813rem', color: '#64748b' }}>
						${mode === 'login'
							? html`Don't have an account? <button onClick=${() => { setMode('signup'); setError(''); }} style=${{ background: 'none', border: 'none', color: '#0ea5e9', cursor: 'pointer', fontWeight: 600, fontSize: '0.813rem' }}>Sign up</button>`
							: html`Already have an account? <button onClick=${() => { setMode('login'); setError(''); }} style=${{ background: 'none', border: 'none', color: '#0ea5e9', cursor: 'pointer', fontWeight: 600, fontSize: '0.813rem' }}>Sign in</button>`
						}
					</div>
				</div>
			</div>
		</div>
	`;
}

// ─── Dashboard Header ────────────────────────────────────────────────
function DashboardHeader() {
	const user = currentUser.value;
	const roles = userRoles.value;
	const isAdmin = Array.isArray(roles) && roles.includes('admin');

	return html`
		<header style=${{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '1rem 1.5rem', background: 'white', borderBottom: '1px solid #e2e8f0' }}>
			<div style=${{ display: 'flex', alignItems: 'center' }}>
				<img src="/images/logo_long.png" alt="Solobase" style=${{ height: '32px', width: 'auto' }} />
			</div>
			<div style=${{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
				${isAdmin ? html`
					<a href="/blocks/admin" style=${{ display: 'flex', alignItems: 'center', gap: '0.25rem', fontSize: '0.813rem', color: '#0ea5e9', textDecoration: 'none', fontWeight: 600 }}>
						<${Shield} size=${16} /> Admin
					</a>
				` : null}
				<span style=${{ fontSize: '0.813rem', color: '#64748b' }}>${user?.email || ''}</span>
				<button onClick=${() => { logout(); }} style=${{ background: 'none', border: 'none', cursor: 'pointer', color: '#64748b', display: 'flex', alignItems: 'center', gap: '0.25rem', fontSize: '0.813rem' }}>
					<${LogOut} size=${16} /> Logout
				</button>
			</div>
		</header>
	`;
}

// ─── Overview Tab ────────────────────────────────────────────────────
function OverviewTab() {
	const user = currentUser.value;
	const [planName, setPlanName] = useState<string>('...');
	const [deploymentCount, setDeploymentCount] = useState<string>('...');
	const [apiKeyCount, setApiKeyCount] = useState<string>('...');
	const [productCount, setProductCount] = useState<string>('...');

	useEffect(() => {
		// Fetch current plan from purchases
		api.get('/b/products/purchases').then((data: any) => {
			const records = Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : [];
			const completed = records.filter((p: any) => p.status === 'completed');
			if (completed.length > 0) {
				const latest = completed[completed.length - 1];
				setPlanName(latest.product_name || latest.name || 'Paid');
			} else {
				setPlanName('Free');
			}
		}).catch(() => setPlanName('Free'));

		// Fetch deployments count
		api.get('/b/deployments').then((data: any) => {
			const records = Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : [];
			setDeploymentCount(String(records.length));
		}).catch(() => setDeploymentCount('0'));

		// Fetch API keys count
		api.get('/auth/api-keys').then((data: any) => {
			const records = Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : [];
			setApiKeyCount(String(records.length));
		}).catch(() => setApiKeyCount('0'));

		// Fetch products count
		api.get('/b/products/products').then((data: any) => {
			const records = Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : [];
			setProductCount(String(records.length));
		}).catch(() => setProductCount('0'));
	}, []);

	const displayName = user?.name || user?.email?.split('@')[0] || 'there';

	return html`
		<div>
			<${PageHeader} title=${`Welcome back, ${displayName}`} description="Here's an overview of your account" />
			<div style=${{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(220px, 1fr))', gap: '1rem', marginBottom: '2rem' }}>
				<${StatCard} title="Plan" value=${planName} icon=${CreditCard} />
				<${StatCard} title="Products" value=${productCount} icon=${Package} />
				<${StatCard} title="Deployments" value=${deploymentCount} icon=${Server} />
				<${StatCard} title="API Keys" value=${apiKeyCount} icon=${Key} />
			</div>

			<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px', padding: '2rem', textAlign: 'center' }}>
				<${Rocket} size=${48} style=${{ color: '#0ea5e9', marginBottom: '1rem' }} />
				<h2 style=${{ fontSize: '1.25rem', fontWeight: 700, color: '#1e293b', marginBottom: '0.5rem' }}>Get Started</h2>
				<p style=${{ fontSize: '0.875rem', color: '#64748b', maxWidth: '400px', margin: '0 auto 1.5rem', lineHeight: 1.6 }}>
					Choose a plan, deploy your backend, and start building your application.
				</p>
				<div style=${{ display: 'flex', gap: '0.75rem', justifyContent: 'center' }}>
					<a href="/docs/" style=${{ padding: '0.5rem 1rem', border: '1px solid #e2e8f0', borderRadius: '8px', fontSize: '0.813rem', color: '#1e293b', textDecoration: 'none' }}>Read Docs</a>
				</div>
			</div>
		</div>
	`;
}

// ─── Deployments Tab ─────────────────────────────────────────────────
function DeploymentsTab() {
	const [deployments, setDeployments] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [showCreateForm, setShowCreateForm] = useState(false);
	const [newName, setNewName] = useState('');
	const [creating, setCreating] = useState(false);
	const [deleting, setDeleting] = useState<string | null>(null);

	const fetchDeployments = useCallback(async () => {
		try {
			const data: any = await api.get('/b/deployments');
			const records = Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : [];
			setDeployments(records);
		} catch {
			setDeployments([]);
		}
		setLoading(false);
	}, []);

	useEffect(() => { fetchDeployments(); }, [fetchDeployments]);

	async function handleCreate(e: Event) {
		e.preventDefault();
		if (!newName.trim()) return;
		setCreating(true);
		try {
			await api.post('/b/deployments', { name: newName.trim() });
			toasts.success('Deployment created successfully');
			setNewName('');
			setShowCreateForm(false);
			await fetchDeployments();
		} catch (err: any) {
			toasts.error(err.message || 'Failed to create deployment');
		}
		setCreating(false);
	}

	async function handleDelete(id: string) {
		setDeleting(id);
		try {
			await api.delete(`/b/deployments/${id}`);
			toasts.success('Deployment deleted');
			await fetchDeployments();
		} catch (err: any) {
			toasts.error(err.message || 'Failed to delete deployment');
		}
		setDeleting(null);
	}

	function getStatusVariant(status: string): 'success' | 'warning' | 'danger' | 'neutral' {
		switch (status) {
			case 'active': return 'success';
			case 'pending': return 'warning';
			case 'stopped': return 'danger';
			case 'deleted': return 'neutral';
			default: return 'neutral';
		}
	}

	if (loading) return html`<${LoadingSpinner} message="Loading deployments..." />`;

	return html`
		<div>
			<${PageHeader} title="Deployments" description="Manage your backend instances">
				<${Button} icon=${Plus} onClick=${() => setShowCreateForm(true)}>Create Deployment<//>
			<//>

			${showCreateForm ? html`
				<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px', padding: '1.5rem', marginBottom: '1.5rem' }}>
					<h3 style=${{ fontSize: '1rem', fontWeight: 600, color: '#1e293b', marginBottom: '1rem' }}>New Deployment</h3>
					<form onSubmit=${handleCreate}>
						<div>
							<label style=${{ display: 'block', fontSize: '0.813rem', fontWeight: 500, color: '#1e293b', marginBottom: '0.375rem' }}>Name</label>
							<input type="text" value=${newName} onInput=${(e: any) => setNewName(e.target.value)}
								placeholder="my-backend" required
								style=${{ width: '100%', padding: '0.5rem 0.75rem', border: '1px solid #e2e8f0', borderRadius: '8px', fontSize: '0.813rem', outline: 'none', boxSizing: 'border-box' }} />
						</div>
						<div style=${{ display: 'flex', gap: '0.5rem', marginTop: '1rem' }}>
							<${Button} type="submit" loading=${creating}>Create<//>
							<${Button} variant="secondary" onClick=${() => { setShowCreateForm(false); setNewName(''); }}>Cancel<//>
						</div>
					</form>
				</div>
			` : null}

			${deployments.length === 0 && !showCreateForm ? html`
				<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px' }}>
					<${EmptyState} icon=${Server} title="No deployments yet" description="Deploy your first Solobase backend instance to get started.">
						<${Button} icon=${Rocket} onClick=${() => setShowCreateForm(true)}>Create Deployment<//>
					<//>
				</div>
			` : null}

			${deployments.length > 0 ? html`
				<div style=${{ display: 'grid', gap: '0.5rem' }}>
					${deployments.map((d: any) => html`
						<div key=${d.id} style=${{
							display: 'flex', justifyContent: 'space-between', alignItems: 'center',
							background: 'white', border: '1px solid #e2e8f0', borderRadius: '8px',
							padding: '0.875rem 1rem'
						}}>
							<div style=${{ display: 'flex', alignItems: 'center', gap: '1rem', flex: 1 }}>
								<${Server} size=${18} style=${{ color: '#64748b', flexShrink: 0 }} />
								<div style=${{ minWidth: 0 }}>
									<div style=${{ fontWeight: 600, fontSize: '0.875rem', color: '#1e293b' }}>${d.name}</div>
									<div style=${{ fontSize: '0.75rem', color: '#64748b', marginTop: '0.125rem' }}>
										Created ${d.created_at ? new Date(d.created_at).toLocaleDateString() : ''}
									</div>
								</div>
							</div>
							<div style=${{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
								<${StatusBadge} status=${d.status || 'pending'} variant=${getStatusVariant(d.status || 'pending')} />
								<button onClick=${() => handleDelete(d.id)} disabled=${deleting === d.id}
									style=${{
										background: 'none', border: '1px solid #fecaca', borderRadius: '6px',
										padding: '0.25rem 0.5rem', fontSize: '0.75rem', color: '#dc2626',
										cursor: deleting === d.id ? 'not-allowed' : 'pointer',
										display: 'inline-flex', alignItems: 'center', gap: '0.25rem',
										opacity: deleting === d.id ? 0.5 : 1
									}}>
									<${Trash2} size=${12} /> ${deleting === d.id ? '...' : 'Delete'}
								</button>
							</div>
						</div>
					`)}
				</div>
			` : null}
		</div>
	`;
}

// ─── API Keys Tab ────────────────────────────────────────────────────
function ApiKeysTab() {
	const [keys, setKeys] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [newKeyName, setNewKeyName] = useState('');
	const [createdKey, setCreatedKey] = useState<string | null>(null);
	const [creating, setCreating] = useState(false);

	const fetchKeys = useCallback(async () => {
		try {
			const data: any = await api.get('/auth/api-keys');
			setKeys(Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : []);
		} catch {
			setKeys([]);
		}
		setLoading(false);
	}, []);

	useEffect(() => { fetchKeys(); }, [fetchKeys]);

	async function createKey(e: Event) {
		e.preventDefault();
		if (!newKeyName.trim()) return;
		setCreating(true);
		try {
			const res: any = await api.post('/auth/api-keys', { name: newKeyName.trim() });
			setCreatedKey(res.key || res.data?.key);
			setNewKeyName('');
			await fetchKeys();
		} catch (err: any) {
			toasts.error(err.message || 'Failed to create API key');
		}
		setCreating(false);
	}

	async function revokeKey(id: string) {
		try {
			await api.delete(`/auth/api-keys/${id}`);
			toasts.success('API key revoked');
			await fetchKeys();
		} catch (err: any) {
			toasts.error(err.message || 'Failed to revoke API key');
		}
	}

	if (loading) return html`<${LoadingSpinner} message="Loading API keys..." />`;

	return html`
		<div>
			<${PageHeader} title="API Keys" description="Manage your API keys for programmatic access" />

			${createdKey ? html`
				<div style=${{ background: '#f0fdf4', border: '1px solid #bbf7d0', borderRadius: '8px', padding: '1rem', marginBottom: '1rem' }}>
					<p style=${{ fontSize: '0.813rem', fontWeight: 600, color: '#166534', marginBottom: '0.5rem' }}>New API key created! Copy it now -- you won't see it again.</p>
					<div style=${{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
						<code style=${{ fontSize: '0.813rem', background: 'white', padding: '0.375rem 0.5rem', borderRadius: '4px', border: '1px solid #e2e8f0', wordBreak: 'break-all', flex: 1 }}>${createdKey}</code>
						<button onClick=${() => { navigator.clipboard.writeText(createdKey); toasts.success('Copied to clipboard'); }}
							style=${{ background: 'none', border: '1px solid #bbf7d0', borderRadius: '6px', padding: '0.25rem 0.5rem', fontSize: '0.75rem', color: '#166534', cursor: 'pointer' }}>Copy</button>
						<button onClick=${() => setCreatedKey(null)}
							style=${{ background: 'none', border: 'none', color: '#166534', fontSize: '0.75rem', cursor: 'pointer' }}>Dismiss</button>
					</div>
				</div>
			` : null}

			<form onSubmit=${createKey} style=${{ display: 'flex', gap: '0.5rem', marginBottom: '1.5rem' }}>
				<input type="text" value=${newKeyName} onInput=${(e: any) => setNewKeyName(e.target.value)}
					placeholder="Key name (e.g. ci-deploy)"
					style=${{ flex: 1, padding: '0.5rem 0.75rem', border: '1px solid #e2e8f0', borderRadius: '8px', fontSize: '0.813rem', outline: 'none' }} />
				<${Button} type="submit" loading=${creating} icon=${Plus}>Create Key<//>
			</form>

			${keys.length === 0 ? html`
				<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px' }}>
					<${EmptyState} icon=${Key} title="No API keys yet" description="Create an API key for programmatic access to your account." />
				</div>
			` : html`
				<div style=${{ display: 'grid', gap: '0.5rem' }}>
					${keys.map((k: any) => html`
						<div key=${k.id} style=${{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'white', border: '1px solid #e2e8f0', borderRadius: '8px', padding: '0.75rem 1rem' }}>
							<div>
								<div style=${{ fontWeight: 600, fontSize: '0.875rem', color: '#1e293b' }}>${k.name}</div>
								<div style=${{ fontSize: '0.75rem', color: '#64748b' }}>${k.key_prefix || 'sb_***'} · Created ${k.created_at ? new Date(k.created_at).toLocaleDateString() : ''}</div>
							</div>
							<button onClick=${() => revokeKey(k.id)}
								style=${{ background: 'none', border: '1px solid #fecaca', borderRadius: '6px', padding: '0.25rem 0.5rem', fontSize: '0.75rem', color: '#dc2626', cursor: 'pointer' }}>Revoke</button>
						</div>
					`)}
				</div>
			`}
		</div>
	`;
}

// ─── Settings Tab ────────────────────────────────────────────────────
function SettingsTab() {
	const user = currentUser.value;
	const [name, setName] = useState(user?.name || '');
	const [saving, setSaving] = useState(false);
	const [loaded, setLoaded] = useState(false);

	useEffect(() => {
		api.get('/auth/me').then((data: any) => {
			const userData = data?.user || data?.data?.user || data;
			if (userData?.name) {
				setName(userData.name);
			}
			setLoaded(true);
		}).catch(() => setLoaded(true));
	}, []);

	async function handleSave(e: Event) {
		e.preventDefault();
		setSaving(true);
		try {
			await api.put('/auth/me', { name });
			toasts.success('Profile updated successfully');
			// Re-check auth to update the global user state
			await checkAuth();
		} catch (err: any) {
			toasts.error(err.message || 'Failed to update profile');
		}
		setSaving(false);
	}

	if (!loaded) return html`<${LoadingSpinner} message="Loading settings..." />`;

	return html`
		<div>
			<${PageHeader} title="Account Settings" description="Manage your profile and preferences" />
			<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px', padding: '1.5rem', maxWidth: '500px' }}>
				<form onSubmit=${handleSave}>
					<div style=${{ marginBottom: '1rem' }}>
						<label style=${{ display: 'block', fontSize: '0.813rem', fontWeight: 500, color: '#1e293b', marginBottom: '0.375rem' }}>Email</label>
						<input type="email" value=${user?.email || ''} disabled
							style=${{ width: '100%', padding: '0.5rem 0.75rem', border: '1px solid #e2e8f0', borderRadius: '8px', fontSize: '0.813rem', background: '#f8fafc', color: '#64748b', boxSizing: 'border-box' }} />
					</div>
					<div style=${{ marginBottom: '1.5rem' }}>
						<label style=${{ display: 'block', fontSize: '0.813rem', fontWeight: 500, color: '#1e293b', marginBottom: '0.375rem' }}>Display Name</label>
						<input type="text" value=${name} onInput=${(e: any) => setName(e.target.value)}
							placeholder="Your name"
							style=${{ width: '100%', padding: '0.5rem 0.75rem', border: '1px solid #e2e8f0', borderRadius: '8px', fontSize: '0.813rem', outline: 'none', boxSizing: 'border-box' }} />
					</div>
					<${Button} type="submit" loading=${saving}>${saving ? 'Saving...' : 'Save Changes'}<//>
				</form>
			</div>
		</div>
	`;
}

// ─── Dashboard Nav ───────────────────────────────────────────────────
function DashboardNav({ active, onNavigate }: { active: string, onNavigate: (page: string) => void }) {
	const tabs = [
		{ id: 'overview', label: 'Overview', icon: Activity },
		{ id: 'deployments', label: 'Deployments', icon: Server },
		{ id: 'api-keys', label: 'API Keys', icon: Key },
		{ id: 'settings', label: 'Settings', icon: Settings },
	];

	return html`
		<nav style=${{ padding: '0 1.5rem', background: 'white', borderBottom: '1px solid #e2e8f0', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
			<${TabNavigation} tabs=${tabs} activeTab=${active} onTabChange=${onNavigate} />
			<a href="/blocks/products/frontend/user/index.html" style=${{ display: 'flex', alignItems: 'center', gap: '0.25rem', fontSize: '0.813rem', color: '#0ea5e9', textDecoration: 'none', fontWeight: 600, padding: '0.5rem 0' }}>
				<${Package} size=${14} /> Products
			</a>
		</nav>
	`;
}

// ─── Main Dashboard ──────────────────────────────────────────────────
function Dashboard() {
	const [page, setPage] = useState(() => window.location.hash.slice(1) || 'overview');

	useEffect(() => {
		window.location.hash = page;
	}, [page]);

	useEffect(() => {
		function onHash() { setPage(window.location.hash.slice(1) || 'overview'); }
		window.addEventListener('hashchange', onHash);
		return () => window.removeEventListener('hashchange', onHash);
	}, []);

	return html`
		<div style=${{ minHeight: '100vh', background: '#f8fafc' }}>
			<${DashboardHeader} />
			<${DashboardNav} active=${page} onNavigate=${setPage} />
			<main style=${{ padding: '1.5rem', maxWidth: '1200px', margin: '0 auto' }}>
				${page === 'overview' ? html`<${OverviewTab} />` : null}
				${page === 'deployments' ? html`<${DeploymentsTab} />` : null}
				${page === 'api-keys' ? html`<${ApiKeysTab} />` : null}
				${page === 'settings' ? html`<${SettingsTab} />` : null}
			</main>
			<${ToastContainer} />
		</div>
	`;
}

// ─── App Entry Point ─────────────────────────────────────────────────
export function App() {
	return html`
		<${AuthGuard}>
			<${Dashboard} />
		<//>
	`;
}
