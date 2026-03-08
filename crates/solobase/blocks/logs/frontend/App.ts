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
			const data: any = await api.get('/admin/logs');
			const records = Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : [];
			setLogs(records);
		} catch {
			setLogs([]);
		}
		setLoading(false);
	}

	useEffect(() => { fetchLogs(); }, []);

	const filtered = search
		? logs.filter(l =>
			l.message?.toLowerCase().includes(search.toLowerCase()) ||
			l.level?.toLowerCase().includes(search.toLowerCase()) ||
			l.source?.toLowerCase().includes(search.toLowerCase())
		)
		: logs;

	const columns = [
		{
			key: 'level', label: 'Level', width: '80px', sortable: true,
			render: (v: string) => html`
				<span style=${{
					fontSize: '0.75rem',
					padding: '0.125rem 0.5rem',
					borderRadius: '9999px',
					fontWeight: 600,
					background: v === 'error' ? '#fef2f2' : v === 'warn' ? '#fffbeb' : '#f0fdf4',
					color: v === 'error' ? '#dc2626' : v === 'warn' ? '#d97706' : '#16a34a'
				}}>${v || 'info'}</span>
			`
		},
		{ key: 'message', label: 'Message', sortable: true },
		{ key: 'source', label: 'Source', sortable: true },
		{ key: 'timestamp', label: 'Time', sortable: true, render: (v: string) => v ? new Date(v).toLocaleString() : '-' },
	];

	return html`
		<${BlockShell} title="Logs">
			<${PageHeader} title="System Logs" description="View application logs and events">
				<${Button} icon=${RefreshCw} onClick=${fetchLogs} variant="secondary" size="sm">Refresh<//>
			<//>
			<${SearchInput} value=${search} onChange=${setSearch} placeholder="Search logs..." />
			${loading
				? html`<${LoadingSpinner} message="Loading logs..." />`
				: html`<${DataTable} columns=${columns} data=${filtered} emptyMessage="No logs found" />`
			}
		<//>
	`;
}
