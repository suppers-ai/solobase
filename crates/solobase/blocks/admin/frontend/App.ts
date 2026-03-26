import { html, BlockShell, PageHeader, StatCard, api, TabNavigation, SearchInput, DataTable, LoadingSpinner } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { LayoutDashboard, Users, Database, Package, Settings, HardDrive, Layers, ShoppingCart, DollarSign } from 'lucide-preact';

function DashboardTab() {
	const [stats, setStats] = useState<any>(null);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		Promise.all([
			api.get('/admin/users?page=1&pageSize=1').catch(() => ({ total: 0 })),
			api.getStorageBuckets().catch(() => ({ data: [] })),
			api.getExtensions().catch(() => ({ data: [] })),
			api.get('/admin/b/products/stats').catch(() => ({})),
		]).then(([usersRes, storageRes, extRes, productStats]) => {
			setStats({
				users: (usersRes as any)?.total || (usersRes as any)?.records?.length || 0,
				buckets: Array.isArray((storageRes as any)?.data) ? (storageRes as any).data.length : Array.isArray(storageRes) ? (storageRes as any).length : 0,
				extensions: Array.isArray((extRes as any)?.data) ? (extRes as any).data.length : Array.isArray(extRes) ? (extRes as any).length : 0,
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
				<${StatCard} title="Extensions" value=${stats?.extensions || 0} icon=${Package} />
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
	const [settings, setSettings] = useState<any>(null);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		api.getSettings().then(res => {
			if (!res.error) setSettings(res.data);
			setLoading(false);
		});
	}, []);

	if (loading) return html`<${LoadingSpinner} message="Loading settings..." />`;

	return html`
		<div>
			<${PageHeader} title="Settings" description="Instance configuration" />
			<div style=${{ background: 'white', border: '1px solid var(--border-color, #e2e8f0)', borderRadius: '12px', padding: '1.5rem' }}>
				<pre style=${{ fontSize: '0.813rem', overflow: 'auto', maxHeight: '500px', color: 'var(--text-primary, #1e293b)' }}>
					${settings ? JSON.stringify(settings, null, 2) : 'No settings available'}
				</pre>
			</div>
		</div>
	`;
}

function BlocksTab() {
	const [extensions, setExtensions] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		api.getExtensions().then(res => {
			if (!res.error) {
				const data = Array.isArray(res.data) ? res.data : [];
				setExtensions(data);
			}
			setLoading(false);
		});
	}, []);

	if (loading) return html`<${LoadingSpinner} message="Loading blocks..." />`;

	return html`
		<div>
			<${PageHeader} title="Blocks" description="Installed WAFER blocks and extensions" />
			<div style=${{ display: 'grid', gap: '0.75rem' }}>
				${extensions.length === 0 ? html`<p style=${{ color: 'var(--text-secondary, #64748b)' }}>No extensions installed</p>` : null}
				${extensions.map((ext: any) => html`
					<div key=${ext.name} style=${{
						background: 'white',
						border: '1px solid var(--border-color, #e2e8f0)',
						borderRadius: '8px',
						padding: '1rem 1.25rem',
						display: 'flex',
						justifyContent: 'space-between',
						alignItems: 'center'
					}}>
						<div>
							<div style=${{ fontWeight: 600, color: 'var(--text-primary, #1e293b)' }}>${ext.name}</div>
							<div style=${{ fontSize: '0.813rem', color: 'var(--text-secondary, #64748b)', marginTop: '0.125rem' }}>${ext.version || ''} - ${ext.summary || ''}</div>
						</div>
						<span style=${{
							fontSize: '0.75rem',
							padding: '0.25rem 0.5rem',
							borderRadius: '9999px',
							background: ext.enabled !== false ? '#dcfce7' : '#f3f4f6',
							color: ext.enabled !== false ? '#166534' : '#6b7280'
						}}>${ext.enabled !== false ? 'Active' : 'Disabled'}</span>
					</div>
				`)}
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

	const tabs = [
		{ id: 'dashboard', label: 'Dashboard', icon: LayoutDashboard },
		{ id: 'users', label: 'Users', icon: Users },
		{ id: 'blocks', label: 'Blocks', icon: Layers },
		{ id: 'settings', label: 'Settings', icon: Settings },
	];

	return html`
		<${BlockShell} title="Admin">
			<${TabNavigation} tabs=${tabs} activeTab=${tab} onTabChange=${setTab} />
			${tab === 'dashboard' ? html`<${DashboardTab} />` : null}
			${tab === 'users' ? html`<${UsersTab} />` : null}
			${tab === 'blocks' ? html`<${BlocksTab} />` : null}
			${tab === 'settings' ? html`<${SettingsTab} />` : null}
		<//>
	`;
}
