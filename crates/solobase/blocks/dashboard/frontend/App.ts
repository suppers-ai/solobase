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
			} else {
				window.location.reload();
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
						${mode === 'login' ? 'Sign in to Solobase Cloud' : 'Create your Solobase account'}
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
							style=${{ width: '100%', padding: '0.75rem', background: 'linear-gradient(135deg, #fe6627, #fc4c03)', color: 'white', border: 'none', borderRadius: '8px', fontSize: '0.875rem', fontWeight: 600, cursor: loading ? 'not-allowed' : 'pointer', opacity: loading ? 0.7 : 1 }}>
							${loading ? (mode === 'signup' ? 'Creating account...' : 'Signing in...') : (mode === 'signup' ? 'Create Account' : 'Sign In')}
						</button>
					</form>

					<div style=${{ textAlign: 'center', marginTop: '1rem', fontSize: '0.813rem', color: '#64748b' }}>
						${mode === 'login'
							? html`Don't have an account? <button onClick=${() => { setMode('signup'); setError(''); }} style=${{ background: 'none', border: 'none', color: '#fe6627', cursor: 'pointer', fontWeight: 600, fontSize: '0.813rem' }}>Sign up</button>`
							: html`Already have an account? <button onClick=${() => { setMode('login'); setError(''); }} style=${{ background: 'none', border: 'none', color: '#fe6627', cursor: 'pointer', fontWeight: 600, fontSize: '0.813rem' }}>Sign in</button>`
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
					<a href="/blocks/admin" style=${{ display: 'flex', alignItems: 'center', gap: '0.25rem', fontSize: '0.813rem', color: '#fe6627', textDecoration: 'none', fontWeight: 600 }}>
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

// ─── Usage Bar ──────────────────────────────────────────────────────
function UsageBar({ label, used, limit, unit }: { label: string, used: number, limit: number, unit: string }) {
	const pct = limit > 0 ? Math.min((used / limit) * 100, 100) : 0;
	const color = pct > 90 ? '#ef4444' : pct > 70 ? '#f59e0b' : '#fe6627';
	const fmt = (n: number) => {
		if (unit === 'bytes') {
			if (n >= 1073741824) return `${(n / 1073741824).toFixed(1)} GB`;
			if (n >= 1048576) return `${(n / 1048576).toFixed(0)} MB`;
			return `${(n / 1024).toFixed(0)} KB`;
		}
		if (n >= 1000000) return `${(n / 1000000).toFixed(1)}M`;
		if (n >= 1000) return `${(n / 1000).toFixed(0)}K`;
		return String(n);
	};

	return html`
		<div style=${{ marginBottom: '1rem' }}>
			<div style=${{ display: 'flex', justifyContent: 'space-between', fontSize: '0.813rem', marginBottom: '0.375rem' }}>
				<span style=${{ fontWeight: 500, color: '#1e293b' }}>${label}</span>
				<span style=${{ color: '#64748b' }}>${fmt(used)} / ${fmt(limit)}</span>
			</div>
			<div style=${{ height: '8px', background: '#e2e8f0', borderRadius: '4px', overflow: 'hidden' }}>
				<div style=${{ height: '100%', width: `${pct}%`, background: color, borderRadius: '4px', transition: 'width 0.3s' }}></div>
			</div>
		</div>
	`;
}

// ─── Plan limits (must match worker/types.ts PLANS) ─────────────────
const PLAN_LIMITS: Record<string, { requests: number, r2: number, d1: number, projects: number }> = {
	free: { requests: 0, r2: 0, d1: 0, projects: 0 },
	starter: { requests: 500000, r2: 2 * 1024 * 1024 * 1024, d1: 500 * 1024 * 1024, projects: 2 },
	pro: { requests: 3000000, r2: 20 * 1024 * 1024 * 1024, d1: 5 * 1024 * 1024 * 1024, projects: Infinity },
};

// ─── Overview Tab ────────────────────────────────────────────────────
function OverviewTab() {
	const user = currentUser.value;
	const [planName, setPlanName] = useState<string>('...');
	const [projectCount, setProjectCount] = useState<string>('...');
	const [apiKeyCount, setApiKeyCount] = useState<string>('...');
	const [usage, setUsage] = useState<any>(null);

	useEffect(() => {
		// Fetch subscription/plan
		api.get('/billing/subscription').then((data: any) => {
			const sub = data?.subscription;
			if (sub?.status === 'active') {
				setPlanName(sub.plan === 'pro' ? 'Pro' : 'Starter');
			} else {
				setPlanName('Free');
			}
		}).catch(() => setPlanName('Free'));

		// Fetch projects count
		api.get('/b/projects').then((data: any) => {
			const records = Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : [];
			setProjectCount(String(records.length));
		}).catch(() => setProjectCount('0'));

		// Fetch API keys count
		api.get('/auth/api-keys').then((data: any) => {
			const records = Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : [];
			setApiKeyCount(String(records.length));
		}).catch(() => setApiKeyCount('0'));

		// Fetch usage
		api.get('/b/usage').then((data: any) => {
			setUsage(data);
		}).catch(() => setUsage(null));
	}, []);

	const displayName = user?.name || user?.email?.split('@')[0] || 'there';
	const plan = planName.toLowerCase();
	const limits = PLAN_LIMITS[plan] || PLAN_LIMITS['starter'];

	return html`
		<div>
			<${PageHeader} title=${`Welcome back, ${displayName}`} description="Here's an overview of your account" />
			<div style=${{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(220px, 1fr))', gap: '1rem', marginBottom: '2rem' }}>
				<${StatCard} title="Plan" value=${planName} icon=${CreditCard} />
				<${StatCard} title="Projects" value=${projectCount} icon=${Server} />
				<${StatCard} title="API Keys" value=${apiKeyCount} icon=${Key} />
				<${StatCard} title="Month" value=${usage?.month || '...'} icon=${Activity} />
			</div>

			${usage ? html`
				<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px', padding: '1.5rem', marginBottom: '2rem' }}>
					<h3 style=${{ fontSize: '1rem', fontWeight: 600, color: '#1e293b', marginBottom: '1rem' }}>Usage This Month</h3>
					<${UsageBar}
						label="API Requests"
						used=${usage.requests?.used || 0}
						limit=${limits.requests + (usage.requests?.addon || 0)}
						unit="count" />
					<${UsageBar}
						label="File Storage"
						used=${usage.storage?.r2_bytes || 0}
						limit=${limits.r2 + (usage.storage?.r2_addon_bytes || 0)}
						unit="bytes" />
					<${UsageBar}
						label="Database Storage"
						used=${usage.storage?.d1_bytes || 0}
						limit=${limits.d1 + (usage.storage?.d1_addon_bytes || 0)}
						unit="bytes" />
					<div style=${{ textAlign: 'right', marginTop: '0.5rem' }}>
						<a href="/pricing/" style=${{ fontSize: '0.75rem', color: '#fe6627', textDecoration: 'none' }}>Upgrade plan →</a>
					</div>
				</div>
			` : null}

			<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px', padding: '2rem', textAlign: 'center' }}>
				<${Rocket} size=${48} style=${{ color: '#fe6627', marginBottom: '1rem' }} />
				<h2 style=${{ fontSize: '1.25rem', fontWeight: 700, color: '#1e293b', marginBottom: '0.5rem' }}>Create a Project</h2>
				<p style=${{ fontSize: '0.875rem', color: '#64748b', maxWidth: '400px', margin: '0 auto 1.5rem', lineHeight: 1.6 }}>
					Create a new backend project and start building your application.
				</p>
				<div style=${{ display: 'flex', gap: '0.75rem', justifyContent: 'center' }}>
					<a href="#projects" onClick=${(e: any) => { e.preventDefault(); window.location.hash = 'projects'; }} style=${{ padding: '0.625rem 1.25rem', background: 'linear-gradient(135deg, #fe6627, #fc4c03)', color: 'white', borderRadius: '8px', fontSize: '0.875rem', fontWeight: 600, textDecoration: 'none' }}>Create Project</a>
					<a href="https://solobase.dev/docs/" style=${{ padding: '0.625rem 1.25rem', border: '1px solid #e2e8f0', borderRadius: '8px', fontSize: '0.875rem', color: '#1e293b', textDecoration: 'none' }}>Read Docs</a>
				</div>
			</div>
		</div>
	`;
}

// ─── Projects Tab ─────────────────────────────────────────────────
function ProjectsTab() {
	const [projects, setProjects] = useState<any[]>([]);
	const [plan, setPlan] = useState<string>('free');
	const [loading, setLoading] = useState(true);
	const [showCreateForm, setShowCreateForm] = useState(false);
	const [newName, setNewName] = useState('');
	const [creating, setCreating] = useState(false);
	const [deleting, setDeleting] = useState<string | null>(null);
	const [lastCreatedStatus, setLastCreatedStatus] = useState<string | null>(null);

	const fetchProjects = useCallback(async () => {
		try {
			const data: any = await api.get('/b/projects');
			const records = Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : [];
			setProjects(records);
			if (data?.plan) setPlan(data.plan);
		} catch {
			setProjects([]);
		}
		setLoading(false);
	}, []);

	useEffect(() => { fetchProjects(); }, [fetchProjects]);

	async function handleCreate(e: Event) {
		e.preventDefault();
		if (!newName.trim()) return;
		setCreating(true);
		setLastCreatedStatus(null);
		try {
			const result: any = await api.post('/b/projects', { name: newName.trim() });
			const status = result?.data?.status || result?.status || 'inactive';
			setLastCreatedStatus(status);
			if (status === 'active') {
				toasts.success('Project created and activated!');
			} else {
				toasts.success('Project created (inactive). Subscribe to a plan to activate it.');
			}
			setNewName('');
			setShowCreateForm(false);
			await fetchProjects();
		} catch (err: any) {
			toasts.error(err.message || 'Failed to create project');
		}
		setCreating(false);
	}

	async function handleDelete(id: string) {
		setDeleting(id);
		try {
			await api.delete(`/b/projects/${id}`);
			toasts.success('Project deleted');
			await fetchProjects();
		} catch (err: any) {
			toasts.error(err.message || 'Failed to delete project');
		}
		setDeleting(null);
	}

	function getStatusVariant(status: string): 'success' | 'warning' | 'danger' | 'neutral' {
		switch (status) {
			case 'active': return 'success';
			case 'inactive': return 'warning';
			case 'pending': return 'warning';
			case 'stopped': return 'danger';
			case 'deleted': return 'neutral';
			default: return 'neutral';
		}
	}

	if (loading) return html`<${LoadingSpinner} message="Loading projects..." />`;

	const hasInactiveProjects = projects.some((d: any) => (d.data?.status || d.status) === 'inactive');

	return html`
		<div>
			<${PageHeader} title="Projects" description="Manage your backend instances">
				<${Button} icon=${Plus} onClick=${() => setShowCreateForm(true)}>Create Project<//>
			<//>

			${lastCreatedStatus === 'inactive' ? html`
				<div style=${{ background: '#fffbeb', border: '1px solid #fed7aa', borderRadius: '8px', padding: '0.875rem 1rem', marginBottom: '1rem', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
					<div>
						<div style=${{ fontWeight: 600, fontSize: '0.813rem', color: '#92400e' }}>Project created (inactive)</div>
						<div style=${{ fontSize: '0.75rem', color: '#a16207', marginTop: '0.25rem' }}>Subscribe to a plan to activate your projects.</div>
					</div>
					<a href="https://solobase.dev/pricing/" target="_blank" rel="noopener"
						style=${{ display: 'inline-flex', alignItems: 'center', gap: '0.25rem', padding: '0.375rem 0.75rem', background: '#f59e0b', color: 'white', borderRadius: '6px', fontSize: '0.75rem', fontWeight: 600, textDecoration: 'none' }}>
						<${CreditCard} size=${12} /> View Plans
					</a>
				</div>
			` : null}

			${showCreateForm ? html`
				<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px', padding: '1.5rem', marginBottom: '1.5rem' }}>
					<h3 style=${{ fontSize: '1rem', fontWeight: 600, color: '#1e293b', marginBottom: '1rem' }}>New Project</h3>
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

			${projects.length === 0 && !showCreateForm ? html`
				<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px' }}>
					<${EmptyState} icon=${Server} title="No projects yet" description="Deploy your first Solobase backend instance to get started.">
						<${Button} icon=${Rocket} onClick=${() => setShowCreateForm(true)}>Create Project<//>
					<//>
				</div>
			` : null}

			${projects.length > 0 ? html`
				<div style=${{ display: 'grid', gap: '0.5rem' }}>
					${projects.map((d: any) => {
						const status = d.data?.status || d.status || 'inactive';
						const canActivate = d.can_activate === true;
						return html`
						<div key=${d.id} style=${{
							display: 'flex', justifyContent: 'space-between', alignItems: 'center',
							background: 'white', border: '1px solid #e2e8f0', borderRadius: '8px',
							padding: '0.875rem 1rem',
							opacity: status === 'inactive' ? 0.85 : 1
						}}>
							<div style=${{ display: 'flex', alignItems: 'center', gap: '1rem', flex: 1 }}>
								<${Server} size=${18} style=${{ color: status === 'inactive' ? '#94a3b8' : '#64748b', flexShrink: 0 }} />
								<div style=${{ minWidth: 0 }}>
									<div style=${{ fontWeight: 600, fontSize: '0.875rem', color: '#1e293b' }}>${d.data?.name || d.name || 'Unnamed'}</div>
									<div style=${{ fontSize: '0.75rem', color: '#64748b', marginTop: '0.125rem' }}>
										${status === 'active' ? html`<span style=${{ color: '#fe6627' }}>${(d.data?.slug || d.data?.name || '').toLowerCase().replace(/\s+/g, '-')}.solobase.dev</span> · ` : ''}Created ${(d.data?.created_at || d.created_at) ? new Date(d.data?.created_at || d.created_at).toLocaleDateString() : ''}
									</div>
								</div>
							</div>
							<div style=${{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
								<${StatusBadge} status=${status} variant=${getStatusVariant(status)} />
								${status === 'inactive' && !canActivate ? html`
									<a href="https://solobase.dev/pricing/" target="_blank" rel="noopener"
										style=${{ display: 'inline-flex', alignItems: 'center', gap: '0.25rem', padding: '0.25rem 0.5rem', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.75rem', color: '#fe6627', textDecoration: 'none', fontWeight: 500 }}>
										Upgrade to activate
									</a>
								` : null}
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
					`;})}
				</div>

				${hasInactiveProjects && plan === 'free' ? html`
					<div style=${{ marginTop: '1rem', padding: '0.875rem 1rem', background: '#f0f9ff', border: '1px solid #bae6fd', borderRadius: '8px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
						<div style=${{ fontSize: '0.813rem', color: '#0369a1' }}>
							Some projects are inactive. Subscribe to a plan to activate them.
						</div>
						<a href="https://solobase.dev/pricing/" target="_blank" rel="noopener"
							style=${{ display: 'inline-flex', alignItems: 'center', gap: '0.25rem', padding: '0.375rem 0.75rem', background: '#fe6627', color: 'white', borderRadius: '6px', fontSize: '0.75rem', fontWeight: 600, textDecoration: 'none' }}>
							<${CreditCard} size=${12} /> View Plans
						</a>
					</div>
				` : null}
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
		{ id: 'projects', label: 'Projects', icon: Server },
		{ id: 'api-keys', label: 'API Keys', icon: Key },
		{ id: 'settings', label: 'Settings', icon: Settings },
	];

	return html`
		<nav style=${{ padding: '0 1.5rem', background: 'white', borderBottom: '1px solid #e2e8f0' }}>
			<${TabNavigation} tabs=${tabs} activeTab=${active} onTabChange=${onNavigate} />
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
				${page === 'projects' ? html`<${ProjectsTab} />` : null}
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
