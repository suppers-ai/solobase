import {
	html, api, checkAuth, isAuthenticated, authLoading, currentUser, userRoles, logout,
	LoadingSpinner, PageHeader, StatCard, EmptyState, StatusBadge, TabNavigation,
	ToastContainer, toasts, Button, Modal, DataTable, FilterBar
} from '@solobase/ui';
import { useState, useEffect, useCallback } from 'preact/hooks';
import { Key, Settings, LogOut, CreditCard, Server, Plus, Trash2, Rocket, Shield, ExternalLink, Package, BarChart3, Clock, XCircle, Activity } from 'lucide-preact';

// ─── Auth Guard ──────────────────────────────────────────────────────
function AuthGuard({ children }: { children: any }) {
	const [checked, setChecked] = useState(false);

	useEffect(() => {
		checkAuth().then((authenticated) => {
			if (!authenticated) {
				window.location.href = '/auth/login?redirect=' + encodeURIComponent(window.location.href);
			} else {
				setChecked(true);
			}
		});
	}, []);

	if (!checked) {
		return html`<div style=${{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh' }}><${LoadingSpinner} message="Loading..." /></div>`;
	}

	return children;
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
				<span style=${{ fontSize: '0.813rem', color: '#64748b' }}>${user?.email || ''}</span>
				<button onClick=${() => { logout(); window.location.href = '/auth/login'; }} style=${{ background: 'none', border: 'none', cursor: 'pointer', color: '#64748b', display: 'flex', alignItems: 'center', gap: '0.25rem', fontSize: '0.813rem' }}>
					<${LogOut} size=${16} /> Logout
				</button>
			</div>
		</header>
	`;
}

// ─── Usage Bar ──────────────────────────────────────────────────────
// ─── Plan limits (must match worker/types.ts PLANS) ─────────────────
const PLAN_LIMITS: Record<string, { requests: number, r2: number, d1: number, maxCreated: number, maxActive: number }> = {
	free: { requests: 0, r2: 0, d1: 0, maxCreated: 2, maxActive: 0 },
	starter: { requests: 500000, r2: 2 * 1024 * 1024 * 1024, d1: 500 * 1024 * 1024, maxCreated: 2, maxActive: 2 },
	pro: { requests: 3000000, r2: 20 * 1024 * 1024 * 1024, d1: 5 * 1024 * 1024 * 1024, maxCreated: 10, maxActive: 10 },
	platform: { requests: Infinity, r2: Infinity, d1: Infinity, maxCreated: Infinity, maxActive: Infinity },
};

// ─── Overview Tab ────────────────────────────────────────────────────
function OverviewTab() {
	const user = currentUser.value;
	const [planName, setPlanName] = useState<string>('...');
	const [projectCount, setProjectCount] = useState<string>('...');
	const [apiKeyCount, setApiKeyCount] = useState<string>('0');


	useEffect(() => {
		// Fetch subscription/plan + usage in one call
		api.get('/b/products/subscription').then((data: any) => {
			const sub = data?.subscription;
			if (sub?.status === 'active' || sub?.plan) {
				const p = sub.plan || 'free';
				setPlanName(p === 'pro' ? 'Pro' : p === 'starter' ? 'Starter' : 'Free');
			} else {
				setPlanName('Free');
			}
		}).catch(() => setPlanName('Free'));

		// Fetch projects count
		api.get('/b/projects').then((data: any) => {
			const records = Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : [];
			setProjectCount(String(records.length));
		}).catch(() => setProjectCount('0'));

		api.get('/auth/api-keys').then((data: any) => {
			const keys = Array.isArray(data) ? data : Array.isArray(data?.keys) ? data.keys : [];
			setApiKeyCount(String(keys.length));
		}).catch(() => setApiKeyCount('0'));

	}, []);

	const displayName = user?.name || user?.email?.split('@')[0] || 'there';

	return html`
		<div>
			<${PageHeader} title=${`Welcome back, ${displayName}`} description="Here's an overview of your account" />
			<div style=${{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(220px, 1fr))', gap: '1rem', marginBottom: '2rem' }}>
				<${StatCard} title="Plan" value=${planName} icon=${CreditCard} />
				<${StatCard} title="Projects" value=${projectCount} icon=${Server} />
				<${StatCard} title="API Keys" value=${apiKeyCount} icon=${Key} />
			</div>

			<div style=${{ display: 'flex', gap: '0.75rem' }}>
				<a href="#projects" onClick=${(e: any) => { e.preventDefault(); window.location.hash = 'projects'; }} style=${{ display: 'inline-flex', alignItems: 'center', gap: '0.5rem', padding: '0.625rem 1.25rem', background: '#fe6627', color: 'white', borderRadius: '8px', fontSize: '0.875rem', fontWeight: 600, textDecoration: 'none' }}><${Rocket} size=${16} /> Create Project</a>
				<a href="https://solobase.dev/docs/" style=${{ padding: '0.625rem 1.25rem', border: '1px solid #e2e8f0', borderRadius: '8px', fontSize: '0.875rem', color: '#1e293b', textDecoration: 'none' }}>Read Docs</a>
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
	const [activating, setActivating] = useState<string | null>(null);
	const [deactivating, setDeactivating] = useState<string | null>(null);
	const [subdomainError, setSubdomainError] = useState<string | null>(null);
	const [showDeleted, setShowDeleted] = useState(false);

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

	useEffect(() => {
		fetchProjects();
		// Also fetch subscription to get the plan
		api.get('/b/products/subscription').then((data: any) => {
			const sub = data?.subscription;
			if (sub?.status === 'active' || sub?.plan) {
				const p = sub.plan || 'free';
				setPlan(p);
			}
		}).catch(() => {});
	}, [fetchProjects]);

	function validateSubdomain(value: string): string | null {
		if (!value) return null;
		if (value.length < 3) return 'Subdomain must be at least 3 characters';
		if (value.length > 63) return 'Subdomain must be 63 characters or fewer';
		if (!/^[a-z]/.test(value)) return 'Subdomain must start with a lowercase letter';
		if (value.endsWith('-')) return 'Subdomain cannot end with a hyphen';
		if (value.includes('--')) return 'Subdomain cannot contain consecutive hyphens';
		if (!/^[a-z0-9-]+$/.test(value)) return 'Subdomain must only contain lowercase letters, numbers, and hyphens';
		const reserved = ['admin','api','app','auth','billing','blog','cdn','cloud','console','dashboard','dev','docs','help','internal','login','mail','manage','platform','settings','staging','status','support','test','www'];
		if (reserved.includes(value)) return `Subdomain '${value}' is reserved`;
		return null;
	}

	function handleSubdomainInput(e: any) {
		const raw: string = e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, '');
		setNewName(raw);
		setSubdomainError(validateSubdomain(raw));
	}

	async function handleCreate(e: Event) {
		e.preventDefault();
		if (!newName.trim()) return;
		const err = validateSubdomain(newName);
		if (err) { setSubdomainError(err); return; }
		setCreating(true);
		try {
			const result: any = await api.post('/b/projects', { name: newName.trim() });
			const status = result?.data?.status || result?.status || 'inactive';
			if (status === 'active') {
				toasts.success('Project created and activated!');
			} else {
				toasts.success('Project created.');
			}
			setNewName('');
			setSubdomainError(null);
			setShowCreateForm(false);
			await fetchProjects();
		} catch (err: any) {
			const msg = err.message || 'Failed to create project';
			if (msg.toLowerCase().includes('already taken') || msg.toLowerCase().includes('alreadyexists') || msg.toLowerCase().includes('already exists')) {
				setSubdomainError('This subdomain is already taken');
			} else {
				setSubdomainError(msg);
			}
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

	async function handleActivate(id: string) {
		setActivating(id);
		try {
			await api.put(`/b/projects/${id}`, { action: 'activate' });
			toasts.success('Project activated!');
			await fetchProjects();
		} catch (err: any) {
			const msg = err.message || 'Failed to activate project';
			toasts.error(msg);
		}
		setActivating(null);
	}

	async function handleDeactivate(id: string) {
		setDeactivating(id);
		try {
			await api.put(`/b/projects/${id}`, { action: 'deactivate' });
			toasts.success('Project deactivated. Resources will be retained for 30 days.');
			await fetchProjects();
		} catch (err: any) {
			toasts.error(err.message || 'Failed to deactivate project');
		}
		setDeactivating(null);
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

	const limits = PLAN_LIMITS[plan] || PLAN_LIMITS['free'];
	const hasPendingProjects = projects.some((d: any) => (d.data?.status || d.status) === 'pending');
	const hasInactiveProjects = projects.some((d: any) => (d.data?.status || d.status) === 'inactive');

	function formatGracePeriod(gracePeriodEnd: string | undefined): string | null {
		if (!gracePeriodEnd) return null;
		const end = new Date(gracePeriodEnd);
		const now = new Date();
		const daysLeft = Math.ceil((end.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));
		if (daysLeft <= 0) return 'Grace period expired';
		return `${daysLeft} day${daysLeft === 1 ? '' : 's'} until resources are deleted`;
	}

	return html`
		<div>
			<${PageHeader} title="Projects" description="Manage your backend instances">
				<${Button} icon=${Plus} onClick=${() => setShowCreateForm(true)}>Create Project<//>
			<//>

			${(hasPendingProjects || hasInactiveProjects) && plan === 'free' ? html`
				<div style=${{ background: '#fffbeb', border: '1px solid #fed7aa', borderRadius: '8px', padding: '0.875rem 1rem', marginBottom: '1rem', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
					<div>
						<div style=${{ fontWeight: 600, fontSize: '0.813rem', color: '#92400e' }}>Upgrade to activate your projects</div>
						<div style=${{ fontSize: '0.75rem', color: '#a16207', marginTop: '0.25rem' }}>Free plans cannot activate projects. Subscribe to a paid plan to go live.</div>
					</div>
					<a href="https://solobase.dev/pricing/" target="_blank" rel="noopener"
						style=${{ display: 'inline-flex', alignItems: 'center', gap: '0.25rem', padding: '0.375rem 0.75rem', background: '#f59e0b', color: 'white', borderRadius: '6px', fontSize: '0.75rem', fontWeight: 600, textDecoration: 'none', whiteSpace: 'nowrap' }}>
						<${CreditCard} size=${12} /> View Plans
					</a>
				</div>
			` : null}

			${showCreateForm ? html`
				<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px', padding: '1.5rem', marginBottom: '1.5rem' }}>
					<h3 style=${{ fontSize: '1rem', fontWeight: 600, color: '#1e293b', marginBottom: '1rem' }}>New Project</h3>
					<form onSubmit=${handleCreate}>
						<div>
							<label style=${{ display: 'block', fontSize: '0.813rem', fontWeight: 500, color: '#1e293b', marginBottom: '0.375rem' }}>Subdomain</label>
							<input type="text" value=${newName} onInput=${handleSubdomainInput}
								placeholder="my-app" required
								style=${{ width: '100%', padding: '0.5rem 0.75rem', border: `1px solid ${subdomainError ? '#ef4444' : '#e2e8f0'}`, borderRadius: '8px', fontSize: '0.813rem', outline: 'none', boxSizing: 'border-box' }} />
							${newName ? html`
								<div style=${{ fontSize: '0.813rem', color: '#64748b', marginTop: '0.375rem' }}>
									Your project will be available at <span style=${{ color: '#fe6627', fontWeight: 600 }}>${newName}.solobase.dev</span>
								</div>
							` : null}
							${subdomainError ? html`
								<div style=${{ fontSize: '0.75rem', color: '#ef4444', marginTop: '0.25rem' }}>${subdomainError}</div>
							` : null}
							<div style=${{ fontSize: '0.75rem', color: '#94a3b8', marginTop: '0.5rem' }}>This will be your project's permanent URL and cannot be changed.</div>
						</div>
						<div style=${{ display: 'flex', gap: '0.5rem', marginTop: '1rem' }}>
							<${Button} type="submit" loading=${creating} disabled=${!!subdomainError || !newName || creating}>${creating ? 'Creating...' : 'Create'}<//>
							<${Button} variant="secondary" disabled=${creating} onClick=${() => { setShowCreateForm(false); setNewName(''); setSubdomainError(null); }}>Cancel<//>
						</div>
					</form>
				</div>
			` : null}

			${projects.length === 0 && !showCreateForm ? html`
				<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px' }}>
					<${EmptyState} icon=${Server} title="No projects yet" description="Create your first Solobase backend instance to get started.">
						<${Button} icon=${Rocket} onClick=${() => setShowCreateForm(true)}>Create Project<//>
					<//>
				</div>
			` : null}

			${projects.length > 0 ? html`
				${projects.some((d: any) => (d.data?.status || d.status) === 'deleted') ? html`
					<div style=${{ display: 'flex', justifyContent: 'flex-end', marginBottom: '0.5rem' }}>
						<label style=${{ display: 'inline-flex', alignItems: 'center', gap: '0.375rem', fontSize: '0.75rem', color: '#64748b', cursor: 'pointer' }}>
							<input type="checkbox" checked=${showDeleted} onChange=${(e: Event) => setShowDeleted((e.target as HTMLInputElement).checked)}
								style=${{ accentColor: '#fe6627' }} />
							Show deleted projects
						</label>
					</div>
				` : null}
				<div style=${{ display: 'grid', gap: '0.5rem' }}>
					${projects.filter((d: any) => showDeleted || (d.data?.status || d.status) !== 'deleted').map((d: any) => {
						const status = d.data?.status || d.status || 'pending';
						const gracePeriodEnd = d.data?.grace_period_end;
						const graceInfo = status === 'inactive' ? formatGracePeriod(gracePeriodEnd) : null;
						const canActivate = plan !== 'free' && (status === 'pending' || status === 'inactive');
						return html`
						<div key=${d.id} style=${{
							display: 'flex', justifyContent: 'space-between', alignItems: 'center',
							background: 'white', border: '1px solid #e2e8f0', borderRadius: '8px',
							padding: '0.875rem 1rem',
							opacity: status === 'pending' || status === 'inactive' ? 0.85 : 1
						}}>
							<div style=${{ display: 'flex', alignItems: 'center', gap: '1rem', flex: 1 }}>
								<${Server} size=${18} style=${{ color: status === 'active' ? '#16a34a' : status === 'pending' ? '#ca8a04' : '#94a3b8', flexShrink: 0 }} />
								<div style=${{ minWidth: 0 }}>
									<div style=${{ fontWeight: 600, fontSize: '0.875rem', color: '#1e293b' }}>${d.data?.name || d.name || 'Unnamed'}</div>
									<div style=${{ fontSize: '0.75rem', color: '#64748b', marginTop: '0.125rem' }}>
										${status === 'active' ? html`<span style=${{ color: '#fe6627' }}>${(d.data?.slug || d.data?.name || '').toLowerCase().replace(/\s+/g, '-')}.solobase.dev</span> · ` : ''}Created ${(d.data?.created_at || d.created_at) ? new Date(d.data?.created_at || d.created_at).toLocaleDateString() : ''}
									</div>
									${graceInfo ? html`
										<div style=${{ display: 'inline-flex', alignItems: 'center', gap: '0.25rem', fontSize: '0.688rem', color: '#dc2626', marginTop: '0.25rem' }}>
											<${Clock} size=${10} /><span>${graceInfo}</span>
										</div>
									` : null}
								</div>
							</div>
							<div style=${{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
								<${StatusBadge} status=${status} variant=${getStatusVariant(status)} />
								${canActivate ? html`
									<button onClick=${() => handleActivate(d.id)} disabled=${activating === d.id}
										style=${{
											background: '#16a34a', border: 'none', borderRadius: '6px',
											padding: '0.25rem 0.625rem', fontSize: '0.75rem', color: 'white',
											cursor: activating === d.id ? 'not-allowed' : 'pointer',
											display: 'inline-flex', alignItems: 'center', gap: '0.25rem',
											fontWeight: 500, opacity: activating === d.id ? 0.5 : 1
										}}>
										<${Rocket} size=${12} /> ${activating === d.id ? '...' : 'Activate'}
									</button>
								` : null}
								${(status === 'pending' || status === 'inactive') && plan === 'free' ? html`
									<a href="https://solobase.dev/pricing/" target="_blank" rel="noopener"
										style=${{ display: 'inline-flex', alignItems: 'center', gap: '0.25rem', padding: '0.25rem 0.5rem', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.75rem', color: '#fe6627', textDecoration: 'none', fontWeight: 500 }}>
										Upgrade to activate
									</a>
								` : null}
								${status === 'active' ? html`
									<button onClick=${() => handleDeactivate(d.id)} disabled=${deactivating === d.id}
										style=${{
											background: 'none', border: '1px solid #fed7aa', borderRadius: '6px',
											padding: '0.25rem 0.5rem', fontSize: '0.75rem', color: '#ca8a04',
											cursor: deactivating === d.id ? 'not-allowed' : 'pointer',
											display: 'inline-flex', alignItems: 'center', gap: '0.25rem',
											opacity: deactivating === d.id ? 0.5 : 1
										}}>
										<${XCircle} size=${12} /> ${deactivating === d.id ? '...' : 'Deactivate'}
									</button>
								` : null}
								${status !== 'deleted' ? html`
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
							` : null}
							</div>
						</div>
					`;})}
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
					${keys.map((k: any) => {
						const name = k.data?.name || k.name || 'Unnamed';
						const keyPrefix = k.data?.key_prefix || k.key_prefix || 'sb_***';
						const createdAt = k.data?.created_at || k.created_at;
						return html`
						<div key=${k.id} style=${{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'white', border: '1px solid #e2e8f0', borderRadius: '8px', padding: '0.75rem 1rem' }}>
							<div>
								<div style=${{ fontWeight: 600, fontSize: '0.875rem', color: '#1e293b' }}>${name}</div>
								<div style=${{ fontSize: '0.75rem', color: '#64748b' }}>${keyPrefix} · Created ${createdAt ? new Date(createdAt).toLocaleDateString() : ''}</div>
							</div>
							<button onClick=${() => revokeKey(k.id)}
								style=${{ background: 'none', border: '1px solid #fecaca', borderRadius: '6px', padding: '0.25rem 0.5rem', fontSize: '0.75rem', color: '#dc2626', cursor: 'pointer' }}>Revoke</button>
						</div>
					`; })}
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
	const [planName, setPlanName] = useState('...');

	useEffect(() => {
		api.get('/auth/me').then((data: any) => {
			const userData = data?.user || data?.data?.user || data;
			if (userData?.name) {
				setName(userData.name);
			}
			setLoaded(true);
		}).catch(() => setLoaded(true));

		api.get('/b/products/subscription').then((data: any) => {
			const sub = data?.subscription;
			if (sub?.status === 'active') {
				setPlanName(sub.plan === 'pro' ? 'Pro' : 'Starter');
			} else {
				setPlanName('Free');
			}
		}).catch(() => setPlanName('Free'));
	}, []);

	async function handleSave(e: Event) {
		e.preventDefault();
		setSaving(true);
		try {
			await api.put('/auth/me', { name });
			toasts.success('Profile updated successfully');
			await checkAuth();
		} catch (err: any) {
			toasts.error(err.message || 'Failed to update profile');
		}
		setSaving(false);
	}

	if (!loaded) return html`<${LoadingSpinner} message="Loading settings..." />`;

	const labelStyle = { display: 'block', fontSize: '0.813rem', fontWeight: 500, color: '#1e293b', marginBottom: '0.375rem' };
	const inputStyle = { width: '100%', padding: '0.5rem 0.75rem', border: '1px solid #e2e8f0', borderRadius: '8px', fontSize: '0.813rem', outline: 'none', boxSizing: 'border-box' };

	return html`
		<div>
			<${PageHeader} title="Account Settings" description="Manage your profile and preferences" />

			<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px', padding: '1.5rem', maxWidth: '500px', marginBottom: '1.5rem' }}>
				<h3 style=${{ fontSize: '0.875rem', fontWeight: 600, color: '#1e293b', marginBottom: '1rem' }}>Current Plan</h3>
				<div style=${{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
					<div>
						<span style=${{ fontSize: '1.25rem', fontWeight: 700, color: '#1e293b' }}>${planName}</span>
					</div>
					<a href="/blocks/products/frontend/user/" style=${{ padding: '0.5rem 1rem', background: '#fe6627', color: 'white', borderRadius: '8px', fontSize: '0.813rem', fontWeight: 600, textDecoration: 'none' }}>Manage Plan</a>
				</div>
			</div>

			<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px', padding: '1.5rem', maxWidth: '500px', marginBottom: '1.5rem' }}>
				<h3 style=${{ fontSize: '0.875rem', fontWeight: 600, color: '#1e293b', marginBottom: '1rem' }}>Profile</h3>
				<form onSubmit=${handleSave}>
					<div style=${{ marginBottom: '1rem' }}>
						<label style=${labelStyle}>Email</label>
						<input type="email" value=${user?.email || ''} disabled
							style=${{ ...inputStyle, background: '#f8fafc', color: '#64748b' }} />
					</div>
					<div style=${{ marginBottom: '1.5rem' }}>
						<label style=${labelStyle}>Display Name</label>
						<input type="text" value=${name} onInput=${(e: any) => setName(e.target.value)}
							placeholder="Your name"
							style=${inputStyle} />
					</div>
					<${Button} type="submit" loading=${saving}>${saving ? 'Saving...' : 'Save Changes'}<//>
				</form>
			</div>

			<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px', padding: '1.5rem', maxWidth: '500px' }}>
				<h3 style=${{ fontSize: '0.875rem', fontWeight: 600, color: '#1e293b', marginBottom: '0.5rem' }}>Password</h3>
				<p style=${{ fontSize: '0.813rem', color: '#64748b', marginBottom: '1rem', marginTop: 0 }}>Update your password to keep your account secure.</p>
				<a href="/auth/change-password"
					style=${{ display: 'inline-flex', alignItems: 'center', gap: '0.375rem', padding: '0.5rem 1rem', background: '#1e293b', color: 'white', borderRadius: '8px', fontSize: '0.813rem', fontWeight: 600, textDecoration: 'none' }}>
					<${Shield} size=${14} /> Change Password
				</a>
			</div>
		</div>
	`;
}

// ─── Admin Tab (admin-only) ──────────────────────────────────────────

const ADMIN_STATUS_STYLES: Record<string, { bg: string; color: string }> = {
	active: { bg: '#dcfce7', color: '#166534' },
	pending: { bg: '#fefce8', color: '#854d0e' },
	inactive: { bg: '#fee2e2', color: '#991b1b' },
	stopped: { bg: '#fee2e2', color: '#991b1b' },
	deleted: { bg: '#f1f5f9', color: '#475569' },
};

function adminStatusBadge(status: string) {
	const s = ADMIN_STATUS_STYLES[status] || ADMIN_STATUS_STYLES.deleted;
	return html`
		<span style=${{
			fontSize: '0.75rem',
			padding: '0.125rem 0.5rem',
			borderRadius: '9999px',
			background: s.bg,
			color: s.color
		}}>${status || 'unknown'}</span>
	`;
}

function AdminProjectsSubTab() {
	const [deployments, setDeployments] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [search, setSearch] = useState('');
	const [statusFilter, setStatusFilter] = useState('all');
	const [selected, setSelected] = useState<any>(null);
	const [page, setPage] = useState(1);
	const pageSize = 50;

	useEffect(() => {
		setLoading(true);
		const params = new URLSearchParams({ page: String(page), pageSize: String(pageSize) });
		if (statusFilter !== 'all') params.set('status', statusFilter);
		api.get(`/admin/b/projects?${params}`).then((data: any) => {
			const records = Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : [];
			// Flatten: { id, data: { name, ... } } → { id, name, ... }
			setDeployments(records.map((r: any) => ({ id: r.id, ...r.data })));
			setLoading(false);
		}).catch(() => setLoading(false));
	}, [page, statusFilter]);

	const filtered = search
		? deployments.filter(d => d.name?.toLowerCase().includes(search.toLowerCase()))
		: deployments;

	const columns = [
		{ key: 'name', label: 'Name', sortable: true },
		{ key: 'subdomain', label: 'Subdomain', sortable: true, render: (v: string) => v ? `${v}.solobase.dev` : '-' },
		{ key: 'status', label: 'Status', render: (v: string) => adminStatusBadge(v) },
		{ key: 'plan', label: 'Plan', sortable: true },
		{ key: 'created_at', label: 'Created', sortable: true, render: (v: string) => v ? new Date(v).toLocaleDateString() : '-' },
	];

	if (loading) return html`<${LoadingSpinner} message="Loading projects..." />`;

	return html`
		<div>
			<${FilterBar} search=${search} onSearchChange=${setSearch} searchPlaceholder="Search by name...">
				<select
					value=${statusFilter}
					onChange=${(e: Event) => { setStatusFilter((e.target as HTMLSelectElement).value); setPage(1); }}
					style=${{
						padding: '0.5rem 0.75rem',
						borderRadius: '8px',
						border: '1px solid #e2e8f0',
						fontSize: '0.875rem',
						background: 'white',
						color: '#1e293b',
						cursor: 'pointer'
					}}
				>
					<option value="all">All Statuses</option>
					<option value="active">Active</option>
					<option value="pending">Pending</option>
					<option value="inactive">Inactive</option>
					<option value="stopped">Stopped</option>
					<option value="deleted">Deleted</option>
				</select>
			<//>
			<${DataTable}
				columns=${columns}
				data=${filtered}
				emptyMessage="No projects found"
				onRowClick=${(row: any) => setSelected(row)}
			/>
			${selected ? html`
				<${Modal} show=${true} title=${selected.name || 'Project Details'} maxWidth="560px" onClose=${() => setSelected(null)}>
					<div style=${{ fontSize: '0.875rem' }}>
						${selected.subdomain ? html`
							<div style=${{ background: '#f8fafc', borderRadius: '8px', padding: '0.75rem 1rem', marginBottom: '1rem' }}>
								<a href="https://${selected.subdomain}.solobase.dev" target="_blank" rel="noopener" style=${{ color: '#fe6627', fontWeight: 600, textDecoration: 'none' }}>${selected.subdomain}.solobase.dev</a>
							</div>
						` : null}
						<div style=${{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem', marginBottom: '1rem' }}>
							<div><div style=${{ color: '#94a3b8', fontSize: '0.75rem', marginBottom: '0.25rem' }}>Status</div>${adminStatusBadge(selected.status || 'unknown')}</div>
							<div><div style=${{ color: '#94a3b8', fontSize: '0.75rem', marginBottom: '0.25rem' }}>Plan</div><span style=${{ fontWeight: 500 }}>${selected.plan || 'free'}</span></div>
							<div><div style=${{ color: '#94a3b8', fontSize: '0.75rem', marginBottom: '0.25rem' }}>Created</div>${selected.created_at ? new Date(selected.created_at).toLocaleDateString() : '-'}</div>
							<div><div style=${{ color: '#94a3b8', fontSize: '0.75rem', marginBottom: '0.25rem' }}>Updated</div>${selected.updated_at ? new Date(selected.updated_at).toLocaleDateString() : '-'}</div>
						</div>
						${selected.grace_period_end ? html`
							<div style=${{ background: '#fef2f2', border: '1px solid #fecaca', borderRadius: '8px', padding: '0.75rem', marginBottom: '1rem', fontSize: '0.813rem', color: '#dc2626' }}>
								Grace period ends: ${new Date(selected.grace_period_end).toLocaleDateString()}
							</div>
						` : null}
						${selected.provision_error ? html`
							<div style=${{ background: '#fef2f2', border: '1px solid #fecaca', borderRadius: '8px', padding: '0.75rem', marginBottom: '1rem', fontSize: '0.813rem', color: '#dc2626' }}>
								Provision error: ${selected.provision_error}
							</div>
						` : null}
						<div style=${{ borderTop: '1px solid #e2e8f0', paddingTop: '0.75rem', display: 'grid', gridTemplateColumns: '120px 1fr', gap: '0.375rem', fontSize: '0.813rem', color: '#64748b' }}>
							<div>Project ID</div><div style=${{ color: '#1e293b', wordBreak: 'break-all' }}>${selected.id || '-'}</div>
							<div>User ID</div><div style=${{ color: '#1e293b', wordBreak: 'break-all' }}>${selected.user_id || '-'}</div>
							${selected.tenant_id ? html`<div>Tenant ID</div><div style=${{ color: '#1e293b', wordBreak: 'break-all' }}>${selected.tenant_id}</div>` : null}
						</div>
					</div>
				<//>
			` : null}
		</div>
	`;
}

function AdminStatsSubTab() {
	const [stats, setStats] = useState<any>(null);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		api.get('/admin/b/projects/stats').then((data: any) => {
			setStats(data);
			setLoading(false);
		}).catch(() => setLoading(false));
	}, []);

	if (loading) return html`<${LoadingSpinner} message="Loading stats..." />`;

	return html`
		<div>
			<div style=${{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))', gap: '1rem' }}>
				<${StatCard} title="Total Projects" value=${stats?.total ?? 0} icon=${Rocket} />
				<${StatCard} title="Active" value=${stats?.active ?? 0} icon=${Activity} color="#16a34a" />
				<${StatCard} title="Pending" value=${stats?.pending ?? 0} icon=${Clock} color="#ca8a04" />
				<${StatCard} title="Inactive" value=${stats?.inactive ?? 0} icon=${XCircle} color="#dc2626" />
				<${StatCard} title="Stopped" value=${stats?.stopped ?? 0} icon=${XCircle} color="#991b1b" />
				<${StatCard} title="Deleted" value=${stats?.deleted ?? 0} icon=${Trash2} color="#64748b" />
			</div>
		</div>
	`;
}

function AdminTab() {
	const [subTab, setSubTab] = useState('stats');

	const subTabs = [
		{ id: 'stats', label: 'Stats', icon: BarChart3 },
		{ id: 'projects', label: 'All Projects', icon: Rocket },
	];

	return html`
		<div>
			<div style=${{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '1rem' }}>
				<${PageHeader} title="Admin" description="Manage projects across all users" />
				<a href="/blocks/admin/frontend/" style=${{
					display: 'inline-flex', alignItems: 'center', gap: '0.5rem',
					padding: '0.5rem 1rem', background: '#f1f5f9', color: '#475569',
					borderRadius: '8px', fontSize: '0.813rem', fontWeight: 500,
					textDecoration: 'none', border: '1px solid #e2e8f0', whiteSpace: 'nowrap'
				}}>
					<${Shield} size=${14} />
					Open Admin Panel
				</a>
			</div>
			<div style=${{ marginBottom: '1.5rem' }}>
				<${TabNavigation} tabs=${subTabs} activeTab=${subTab} onTabChange=${setSubTab} />
			</div>
			${subTab === 'projects' ? html`<${AdminProjectsSubTab} />` : null}
			${subTab === 'stats' ? html`<${AdminStatsSubTab} />` : null}
		</div>
	`;
}

// ─── Dashboard Nav ───────────────────────────────────────────────────
function DashboardNav({ active, onNavigate }: { active: string, onNavigate: (page: string) => void }) {
	const roles = userRoles.value;
	const isAdmin = Array.isArray(roles) && roles.includes('admin');

	const tabs = [
		{ id: 'overview', label: 'Overview', icon: Activity },
		{ id: 'projects', label: 'Projects', icon: Server },
		{ id: 'api-keys', label: 'API Keys', icon: Key },
		{ id: 'settings', label: 'Settings', icon: Settings },
		...(isAdmin ? [{ id: 'admin', label: 'Admin', icon: Shield }] : []),
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
				${page === 'admin' ? html`<${AdminTab} />` : null}
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
