import { html, BlockShell, PageHeader, SearchInput, DataTable, LoadingSpinner, Button, api } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { RefreshCw } from 'lucide-preact';

export function App() {
	const [logs, setLogs] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [search, setSearch] = useState('');

	async function fetchLogs() {
		setLoading(true);
		try {
			const data: any = await api.get('/admin/logs?pageSize=200');
			const records = Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : [];
			setLogs(records.map((r: any) => ({ id: r.id, ...r.data })));
		} catch {
			setLogs([]);
		}
		setLoading(false);
	}

	useEffect(() => { fetchLogs(); }, []);

	const filtered = search
		? logs.filter(l =>
			l.action?.toLowerCase().includes(search.toLowerCase()) ||
			l.resource?.toLowerCase().includes(search.toLowerCase()) ||
			l.user_id?.toLowerCase().includes(search.toLowerCase())
		)
		: logs;

	const columns = [
		{ key: 'action', label: 'Action', sortable: true },
		{ key: 'resource', label: 'Resource', sortable: true },
		{ key: 'user_id', label: 'User', sortable: true, render: (v: string) => v ? v.slice(0, 8) + '...' : '-' },
		{ key: 'ip_address', label: 'IP', sortable: true },
		{ key: 'created_at', label: 'Time', sortable: true, render: (v: string) => v ? new Date(v).toLocaleString() : '-' },
	];

	return html`
		<${BlockShell} title="Logs">
			<${PageHeader} title="Audit Logs" description="User actions and system events">
				<${Button} icon=${RefreshCw} onClick=${fetchLogs} variant="secondary" size="sm">Refresh<//>
			<//>
			<${SearchInput} value=${search} onChange=${setSearch} placeholder="Search by action, resource, or user..." />
			${loading
				? html`<${LoadingSpinner} message="Loading logs..." />`
				: html`<${DataTable} columns=${columns} data=${filtered} emptyMessage="No audit logs yet. Actions will appear here as users interact with the system." />`
			}
		<//>
	`;
}
