import { html, BlockShell, PageHeader, StatCard, SearchInput, DataTable, LoadingSpinner, api } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { LayoutDashboard, Users, ShoppingCart, DollarSign, HardDrive, Layers, ExternalLink } from 'lucide-preact';

function DashboardTab() {
	const [stats, setStats] = useState<any>(null);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		Promise.all([
			api.get('/admin/users?page=1&pageSize=1').catch(() => ({ total: 0 })),
			api.getStorageBuckets().catch(() => ({ data: [] })),
			api.getExtensions().catch(() => []),
			api.get('/admin/b/products/stats').catch(() => ({})),
		]).then(([usersRes, storageRes, extRes, productStats]) => {
			const blocks = Array.isArray(extRes) ? extRes : (extRes as any)?.data || [];
			setStats({
				users: (usersRes as any)?.total || (usersRes as any)?.records?.length || 0,
				buckets: Array.isArray((storageRes as any)?.data) ? (storageRes as any).data.length : Array.isArray(storageRes) ? (storageRes as any).length : 0,
				blocks: blocks.length,
				totalProducts: (productStats as any)?.total_products || 0,
				totalPurchases: (productStats as any)?.total_purchases || 0,
				totalRevenue: (productStats as any)?.total_revenue || 0,
			});
			setLoading(false);
		});
	}, []);

	if (loading) return html`<${LoadingSpinner} message="Loading dashboard..." />`;

	const revenue = typeof stats?.totalRevenue === 'number' ? `$${stats.totalRevenue.toFixed(2)}` : '$0.00';

	return html`
		<div>
			<${PageHeader} title="Dashboard" description="Overview of your Solobase instance" />
			<div style=${{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(220px, 1fr))', gap: '1rem', marginBottom: '2rem' }}>
				<${StatCard} title="Total Users" value=${stats?.users || 0} icon=${Users} />
				<${StatCard} title="Storage Buckets" value=${stats?.buckets || 0} icon=${HardDrive} />
				<${StatCard} title="Active Blocks" value=${stats?.blocks || 0} icon=${Layers} />
				<${StatCard} title="Products" value=${stats?.totalProducts || 0} icon=${ShoppingCart} />
				<${StatCard} title="Purchases" value=${stats?.totalPurchases || 0} icon=${ShoppingCart} />
				<${StatCard} title="Revenue" value=${revenue} icon=${DollarSign} />
			</div>
		</div>
	`;
}

function UsersTab() {
	const [users, setUsers] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [search, setSearch] = useState('');

	useEffect(() => {
		api.getUsers(1, 100).then(res => {
			if (!res.error) {
				const records = (res.data as any)?.records || (res.data as any)?.data || [];
				const data = (Array.isArray(records) ? records : []).map((r: any) => ({ id: r.id, ...r.data }));
				setUsers(data);
			}
			setLoading(false);
		});
	}, []);

	const filtered = search
		? users.filter(u => u.email?.toLowerCase().includes(search.toLowerCase()))
		: users;

	const columns = [
		{ key: 'email', label: 'Email', sortable: true },
		{ key: 'name', label: 'Name', sortable: true },
		{ key: 'roles', label: 'Roles', render: (v: any) => (Array.isArray(v) ? v.join(', ') : v || '-') },
		{ key: 'created_at', label: 'Joined', sortable: true, render: (v: string) => v ? new Date(v).toLocaleDateString() : '-' },
	];

	if (loading) return html`<${LoadingSpinner} message="Loading users..." />`;

	return html`
		<div>
			<${PageHeader} title="Users" description="Manage registered users" />
			<${SearchInput} value=${search} onChange=${setSearch} placeholder="Search users..." />
			<${DataTable} columns=${columns} data=${filtered} emptyMessage="No users found" />
		</div>
	`;
}

function SettingsTab() {
	const [variables, setVariables] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [editing, setEditing] = useState<string | null>(null);
	const [editValue, setEditValue] = useState('');
	const [saving, setSaving] = useState(false);

	const loadSettings = () => {
		setLoading(true);
		api.get('/admin/settings/all').then((data: any) => {
			const vars = Array.isArray(data) ? data : [];
			setVariables(vars);
			setLoading(false);
		}).catch(() => setLoading(false));
	};

	useEffect(loadSettings, []);

	const handleSave = async (key: string) => {
		setSaving(true);
		try {
			await api.patch('/admin/settings/' + key, { value: editValue });
			setEditing(null);
			loadSettings();
		} catch (e) {
			// ignore
		}
		setSaving(false);
	};

	if (loading) return html`<${LoadingSpinner} message="Loading settings..." />`;

	return html`
		<div>
			<${PageHeader} title="Settings" description="Instance configuration variables" />
			<div style=${{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
				${variables.map((v: any) => html`
					<div key=${v.key} style=${{
						background: 'white',
						border: '1px solid var(--border-color, #e2e8f0)',
						borderRadius: '8px',
						padding: '1rem 1.25rem',
					}}>
						<div style=${{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '0.25rem' }}>
							<div>
								<div style=${{ fontWeight: 600, fontSize: '0.875rem', color: 'var(--text-primary, #1e293b)' }}>${v.name || v.key}</div>
								${v.description ? html`<div style=${{ fontSize: '0.75rem', color: 'var(--text-secondary, #64748b)', marginTop: '0.125rem' }}>${v.description}</div>` : null}
							</div>
							<code style=${{ fontSize: '0.75rem', color: '#64748b', background: '#f1f5f9', padding: '0.125rem 0.375rem', borderRadius: '4px' }}>${v.key}</code>
						</div>
						${v.warning ? html`<div style=${{ fontSize: '0.75rem', color: '#dc2626', marginTop: '0.25rem' }}>${v.warning}</div>` : null}
						${editing === v.key ? html`
							<div style=${{ display: 'flex', gap: '0.5rem', marginTop: '0.5rem' }}>
								<input
									type=${v.sensitive ? 'password' : 'text'}
									value=${editValue}
									onInput=${(e: any) => setEditValue(e.target.value)}
									style=${{ flex: 1, padding: '0.5rem 0.75rem', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.813rem' }}
									disabled=${saving}
								/>
								<button onClick=${() => handleSave(v.key)} disabled=${saving}
									style=${{ padding: '0.5rem 1rem', background: '#fe6627', color: 'white', border: 'none', borderRadius: '6px', fontSize: '0.813rem', fontWeight: 600, cursor: 'pointer' }}>
									${saving ? 'Saving...' : 'Save'}
								</button>
								<button onClick=${() => setEditing(null)}
									style=${{ padding: '0.5rem 0.75rem', background: '#f1f5f9', color: '#64748b', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.813rem', cursor: 'pointer' }}>
									Cancel
								</button>
							</div>
						` : html`
							<div style=${{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginTop: '0.5rem' }}>
								<code style=${{ fontSize: '0.813rem', color: 'var(--text-primary, #1e293b)', background: '#f8fafc', padding: '0.375rem 0.625rem', borderRadius: '6px', flex: 1, overflow: 'hidden', textOverflow: 'ellipsis' }}>
									${v.sensitive ? '••••••••' : (v.value || '(empty)')}
								</code>
								${!v.sensitive ? html`
									<button onClick=${() => { setEditing(v.key); setEditValue(v.value || ''); }}
										style=${{ padding: '0.375rem 0.75rem', background: 'white', color: '#64748b', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.75rem', cursor: 'pointer', whiteSpace: 'nowrap' }}>
										Edit
									</button>
								` : null}
							</div>
						`}
					</div>
				`)}
				${variables.length === 0 ? html`<p style=${{ color: 'var(--text-secondary, #64748b)' }}>No settings configured</p>` : null}
			</div>
		</div>
	`;
}

function BlocksTab() {
	const [blocks, setBlocks] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		api.getExtensions().then(res => {
			const data = Array.isArray(res) ? res : ((res as any)?.data || []);
			setBlocks(Array.isArray(data) ? data : []);
			setLoading(false);
		}).catch(() => setLoading(false));
	}, []);

	if (loading) return html`<${LoadingSpinner} message="Loading blocks..." />`;

	return html`
		<div>
			<${PageHeader} title="Blocks" description="Registered WAFER blocks in this instance" />
			<div style=${{ marginBottom: '1rem' }}>
				<a href="/debug/inspector/ui" target="_blank" style=${{ display: 'inline-flex', alignItems: 'center', gap: '0.375rem', fontSize: '0.813rem', color: '#fe6627', textDecoration: 'none', fontWeight: 500 }}>
					Open Inspector UI <${ExternalLink} size=${14} />
				</a>
			</div>
			<div style=${{ display: 'grid', gap: '0.5rem' }}>
				${blocks.map((b: any) => html`
					<div key=${b.name} style=${{
						background: 'white',
						border: '1px solid var(--border-color, #e2e8f0)',
						borderRadius: '8px',
						padding: '0.875rem 1.25rem',
						display: 'flex',
						justifyContent: 'space-between',
						alignItems: 'center'
					}}>
						<div>
							<div style=${{ fontWeight: 600, fontSize: '0.875rem', color: 'var(--text-primary, #1e293b)' }}>${b.name}</div>
							<div style=${{ fontSize: '0.75rem', color: 'var(--text-secondary, #64748b)', marginTop: '0.125rem' }}>
								${b.version || ''} ${b.summary ? `\u2014 ${b.summary}` : ''}
							</div>
						</div>
						<div style=${{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
							<span style=${{
								fontSize: '0.688rem',
								padding: '0.125rem 0.5rem',
								borderRadius: '9999px',
								background: '#f1f5f9',
								color: '#64748b',
							}}>${b.interface || ''}</span>
							<span style=${{
								fontSize: '0.688rem',
								padding: '0.125rem 0.5rem',
								borderRadius: '9999px',
								background: '#dcfce7',
								color: '#166534',
							}}>Active</span>
						</div>
					</div>
				`)}
				${blocks.length === 0 ? html`<p style=${{ color: 'var(--text-secondary, #64748b)' }}>No blocks registered</p>` : null}
			</div>
		</div>
	`;
}

export function App() {
	const [tab, setTab] = useState(() => {
		const hash = window.location.hash.slice(1);
		return hash || 'dashboard';
	});

	useEffect(() => {
		window.location.hash = tab;
	}, [tab]);

	useEffect(() => {
		function onHash() { setTab(window.location.hash.slice(1) || 'dashboard'); }
		window.addEventListener('hashchange', onHash);
		return () => window.removeEventListener('hashchange', onHash);
	}, []);

	return html`
		<${BlockShell} title="Admin">
			${tab === 'dashboard' ? html`<${DashboardTab} />` : null}
			${tab === 'users' ? html`<${UsersTab} />` : null}
			${tab === 'blocks' ? html`<${BlocksTab} />` : null}
			${tab === 'settings' ? html`<${SettingsTab} />` : null}
		<//>
	`;
}
