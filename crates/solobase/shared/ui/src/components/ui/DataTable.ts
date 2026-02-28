import { html } from '../../htm';
import { useState } from 'preact/hooks';
import { ChevronUp, ChevronDown } from 'lucide-preact';
import type { ComponentChildren } from 'preact';

interface Column<T = any> {
	key: string;
	label: string;
	sortable?: boolean;
	render?: (value: any, row: T) => ComponentChildren;
	width?: string;
}

interface DataTableProps<T = any> {
	columns: Column<T>[];
	data: T[];
	keyField?: string;
	emptyMessage?: string;
	onRowClick?: (row: T) => void;
}

export function DataTable<T extends Record<string, any>>({
	columns,
	data,
	keyField = 'id',
	emptyMessage = 'No data available',
	onRowClick
}: DataTableProps<T>) {
	const [sortKey, setSortKey] = useState<string | null>(null);
	const [sortDir, setSortDir] = useState<'asc' | 'desc'>('asc');

	function handleSort(key: string) {
		if (sortKey === key) {
			setSortDir(d => d === 'asc' ? 'desc' : 'asc');
		} else {
			setSortKey(key);
			setSortDir('asc');
		}
	}

	const sortedData = sortKey
		? [...data].sort((a, b) => {
			const va = a[sortKey] ?? '';
			const vb = b[sortKey] ?? '';
			const cmp = String(va).localeCompare(String(vb), undefined, { numeric: true });
			return sortDir === 'asc' ? cmp : -cmp;
		})
		: data;

	if (data.length === 0) {
		return html`<div style=${{ textAlign: 'center', padding: '2rem', color: 'var(--text-muted, #94a3b8)', fontSize: '0.875rem' }}>${emptyMessage}</div>`;
	}

	return html`
		<div style=${{ overflowX: 'auto' }}>
			<table style=${{ width: '100%', borderCollapse: 'collapse', fontSize: '0.875rem' }}>
				<thead>
					<tr style=${{ borderBottom: '2px solid var(--border-color, #e2e8f0)' }}>
						${columns.map(col => html`
							<th
								key=${col.key}
								style=${{
									padding: '0.75rem 1rem',
									textAlign: 'left',
									fontWeight: 600,
									color: 'var(--text-secondary, #64748b)',
									fontSize: '0.75rem',
									textTransform: 'uppercase',
									letterSpacing: '0.05em',
									cursor: col.sortable ? 'pointer' : 'default',
									userSelect: 'none',
									width: col.width || 'auto',
									whiteSpace: 'nowrap'
								}}
								onClick=${col.sortable ? () => handleSort(col.key) : undefined}
							>
								<span style=${{ display: 'inline-flex', alignItems: 'center', gap: '0.25rem' }}>
									${col.label}
									${col.sortable && sortKey === col.key
										? sortDir === 'asc'
											? html`<${ChevronUp} size=${14} />`
											: html`<${ChevronDown} size=${14} />`
										: null
									}
								</span>
							</th>
						`)}
					</tr>
				</thead>
				<tbody>
					${sortedData.map(row => html`
						<tr
							key=${row[keyField]}
							style=${{
								borderBottom: '1px solid var(--border-color, #e2e8f0)',
								cursor: onRowClick ? 'pointer' : 'default',
								transition: 'background 0.15s'
							}}
							onClick=${onRowClick ? () => onRowClick(row) : undefined}
							onMouseOver=${(e: Event) => (e.currentTarget as HTMLElement).style.background = 'var(--bg-hover, #f1f5f9)'}
							onMouseOut=${(e: Event) => (e.currentTarget as HTMLElement).style.background = ''}
						>
							${columns.map(col => html`
								<td key=${col.key} style=${{ padding: '0.75rem 1rem', color: 'var(--text-primary, #1e293b)' }}>
									${col.render ? col.render(row[col.key], row) : row[col.key]}
								</td>
							`)}
						</tr>
					`)}
				</tbody>
			</table>
		</div>
	`;
}
