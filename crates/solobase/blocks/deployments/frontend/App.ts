import { html, FeatureShell, PageHeader, TabNavigation, DataTable, SearchInput, StatCard, Modal, LoadingSpinner, FilterBar, api } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { Rocket, BarChart3, Activity, Clock, XCircle, Trash2 } from 'lucide-preact';

const STATUS_STYLES: Record<string, { bg: string; color: string }> = {
	active: { bg: '#dcfce7', color: '#166534' },
	pending: { bg: '#fefce8', color: '#854d0e' },
	stopped: { bg: '#fee2e2', color: '#991b1b' },
	deleted: { bg: '#f1f5f9', color: '#475569' },
};

function statusBadge(status: string) {
	const s = STATUS_STYLES[status] || STATUS_STYLES.deleted;
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

function DeploymentsTab() {
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
		api.get(`/admin/ext/deployments?${params}`).then((data: any) => {
			const records = Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : [];
			setDeployments(records);
			setLoading(false);
		}).catch(() => setLoading(false));
	}, [page, statusFilter]);

	const filtered = search
		? deployments.filter(d => d.name?.toLowerCase().includes(search.toLowerCase()))
		: deployments;

	const columns = [
		{ key: 'name', label: 'Name', sortable: true },
		{ key: 'user_id', label: 'User ID', sortable: true },
		{ key: 'status', label: 'Status', render: (v: string) => statusBadge(v) },
		{ key: 'region', label: 'Region', sortable: true },
		{ key: 'created_at', label: 'Created', sortable: true, render: (v: string) => v ? new Date(v).toLocaleDateString() : '-' },
	];

	if (loading) return html`<${LoadingSpinner} message="Loading deployments..." />`;

	return html`
		<div>
			<${PageHeader} title="Deployments" description="Manage deployments across all users" />
			<${FilterBar} search=${search} onSearchChange=${setSearch} searchPlaceholder="Search by name...">
				<select
					value=${statusFilter}
					onChange=${(e: Event) => { setStatusFilter((e.target as HTMLSelectElement).value); setPage(1); }}
					style=${{
						padding: '0.5rem 0.75rem',
						borderRadius: '8px',
						border: '1px solid var(--border-color, #e2e8f0)',
						fontSize: '0.875rem',
						background: 'white',
						color: 'var(--text-primary, #1e293b)',
						cursor: 'pointer'
					}}
				>
					<option value="all">All Statuses</option>
					<option value="active">Active</option>
					<option value="pending">Pending</option>
					<option value="stopped">Stopped</option>
					<option value="deleted">Deleted</option>
				</select>
			<//>
			<${DataTable}
				columns=${columns}
				data=${filtered}
				emptyMessage="No deployments found"
				onRowClick=${(row: any) => setSelected(row)}
			/>
			${selected ? html`
				<${Modal} show=${true} title="Deployment Details" maxWidth="600px" onClose=${() => setSelected(null)}>
					<div style=${{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
						<div style=${{ display: 'grid', gridTemplateColumns: '140px 1fr', gap: '0.5rem', fontSize: '0.875rem' }}>
							${Object.entries(selected).map(([key, val]: [string, any]) => html`
								<div key=${key} style=${{ fontWeight: 600, color: 'var(--text-secondary, #64748b)' }}>${key}</div>
								<div style=${{ color: 'var(--text-primary, #1e293b)', wordBreak: 'break-all' }}>
									${key === 'status' ? statusBadge(String(val)) : typeof val === 'object' ? JSON.stringify(val, null, 2) : String(val ?? '-')}
								</div>
							`)}
						</div>
					</div>
				<//>
			` : null}
		</div>
	`;
}

function StatsTab() {
	const [stats, setStats] = useState<any>(null);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		api.get('/admin/ext/deployments/stats').then((data: any) => {
			setStats(data);
			setLoading(false);
		}).catch(() => setLoading(false));
	}, []);

	if (loading) return html`<${LoadingSpinner} message="Loading stats..." />`;

	return html`
		<div>
			<${PageHeader} title="Deployment Stats" description="Overview of deployment metrics" />
			<div style=${{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))', gap: '1rem' }}>
				<${StatCard} title="Total Deployments" value=${stats?.total ?? 0} icon=${Rocket} />
				<${StatCard} title="Active" value=${stats?.active ?? 0} icon=${Activity} color="#16a34a" />
				<${StatCard} title="Pending" value=${stats?.pending ?? 0} icon=${Clock} color="#ca8a04" />
				<${StatCard} title="Stopped" value=${stats?.stopped ?? 0} icon=${XCircle} color="#dc2626" />
				<${StatCard} title="Deleted" value=${stats?.deleted ?? 0} icon=${Trash2} color="#64748b" />
			</div>
		</div>
	`;
}

export function App() {
	const [tab, setTab] = useState('deployments');

	const tabs = [
		{ id: 'deployments', label: 'Deployments', icon: Rocket },
		{ id: 'stats', label: 'Stats', icon: BarChart3 },
	];

	return html`
		<${FeatureShell} title="Deployments">
			<${TabNavigation} tabs=${tabs} activeTab=${tab} onTabChange=${setTab} />
			${tab === 'deployments' ? html`<${DeploymentsTab} />` : null}
			${tab === 'stats' ? html`<${StatsTab} />` : null}
		<//>
	`;
}
