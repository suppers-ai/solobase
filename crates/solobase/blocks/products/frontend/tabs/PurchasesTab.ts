import { html, PageHeader, DataTable, Button, Modal, LoadingSpinner, FilterBar, api, toasts } from '@solobase/ui';
import { useState, useEffect, useCallback } from 'preact/hooks';
import { RotateCcw } from 'lucide-preact';

const inputStyle = { width: '100%', padding: '0.5rem 0.75rem', border: '1px solid #e2e8f0', borderRadius: '8px', fontSize: '0.813rem', outline: 'none', boxSizing: 'border-box' as const };
const labelStyle = { display: 'block', fontSize: '0.813rem', fontWeight: 500, color: '#1e293b', marginBottom: '0.375rem' };
const fieldStyle = { marginBottom: '1rem' };

const STATUS_STYLES: Record<string, { bg: string; color: string }> = {
	completed: { bg: '#dcfce7', color: '#166534' },
	pending: { bg: '#fefce8', color: '#854d0e' },
	refunded: { bg: '#fef2f2', color: '#991b1b' },
	cancelled: { bg: '#f1f5f9', color: '#475569' },
};

export function PurchasesTab() {
	const [purchases, setPurchases] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [search, setSearch] = useState('');
	const [statusFilter, setStatusFilter] = useState('all');
	const [showRefund, setShowRefund] = useState(false);
	const [refundTarget, setRefundTarget] = useState<any>(null);
	const [refundReason, setRefundReason] = useState('');
	const [refunding, setRefunding] = useState(false);

	const load = useCallback(async () => {
		setLoading(true);
		try {
			const params = new URLSearchParams();
			if (statusFilter !== 'all') params.set('status', statusFilter);
			const data = await api.get(`/admin/ext/products/purchases?${params}`);
			const records = Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : [];
			setPurchases(records);
		} catch { /* ignore */ }
		setLoading(false);
	}, [statusFilter]);

	useEffect(() => { load(); }, [load]);

	async function handleRefund() {
		if (!refundTarget) return;
		setRefunding(true);
		try {
			await api.put(`/admin/ext/products/purchases/${refundTarget.id}/refund`, { reason: refundReason });
			toasts.success('Refund processed');
			setShowRefund(false);
			setRefundTarget(null);
			setRefundReason('');
			await load();
		} catch (err: any) {
			toasts.error(err.message || 'Failed to process refund');
		}
		setRefunding(false);
	}

	const filtered = search
		? purchases.filter(p => p.user_id?.toLowerCase().includes(search.toLowerCase()) || p.customer_email?.toLowerCase().includes(search.toLowerCase()))
		: purchases;

	const columns = [
		{ key: 'id', label: 'ID', width: '80px', render: (v: any) => html`<code style=${{ fontSize: '0.75rem' }}>#${v}</code>` },
		{ key: 'user_id', label: 'User', sortable: true, render: (v: string, row: any) => row.customer_email || v || '-' },
		{ key: 'status', label: 'Status', render: (v: string) => {
			const s = STATUS_STYLES[v] || STATUS_STYLES.cancelled;
			return html`<span style=${{ fontSize: '0.75rem', padding: '0.125rem 0.5rem', borderRadius: '9999px', background: s.bg, color: s.color }}>${v || 'unknown'}</span>`;
		} },
		{ key: 'total_amount', label: 'Total', render: (v: any, row: any) => v != null ? `${row.currency || 'USD'} ${Number(v).toFixed(2)}` : '-' },
		{ key: 'payment_provider', label: 'Provider', render: (v: string) => v || '-' },
		{ key: 'created_at', label: 'Date', sortable: true, render: (v: string) => v ? new Date(v).toLocaleDateString() : '-' },
		{ key: '_actions', label: '', width: '60px', render: (_: any, row: any) => row.status === 'completed' ? html`
			<button onClick=${(e: Event) => { e.stopPropagation(); setRefundTarget(row); setShowRefund(true); }}
				style=${{ background: 'none', border: 'none', cursor: 'pointer', color: '#dc2626', padding: '0.25rem' }} type="button" title="Refund">
				<${RotateCcw} size=${14} />
			</button>
		` : null },
	];

	if (loading) return html`<${LoadingSpinner} message="Loading purchases..." />`;

	const refundFooter = html`
		<${Button} variant="secondary" onClick=${() => { setShowRefund(false); setRefundTarget(null); setRefundReason(''); }}>Cancel<//>
		<${Button} variant="danger" onClick=${handleRefund} loading=${refunding}>Process Refund<//>
	`;

	return html`
		<div>
			<${PageHeader} title="Purchases" description="View and manage all orders" />
			<${FilterBar} search=${search} onSearchChange=${setSearch} searchPlaceholder="Search by user...">
				<select
					value=${statusFilter}
					onChange=${(e: any) => setStatusFilter(e.target.value)}
					style=${{ padding: '0.5rem 0.75rem', borderRadius: '8px', border: '1px solid #e2e8f0', fontSize: '0.875rem', background: 'white', cursor: 'pointer' }}
				>
					<option value="all">All Statuses</option>
					<option value="pending">Pending</option>
					<option value="completed">Completed</option>
					<option value="refunded">Refunded</option>
					<option value="cancelled">Cancelled</option>
				</select>
			<//>
			<${DataTable} columns=${columns} data=${filtered} emptyMessage="No purchases yet" />

			<${Modal} show=${showRefund} title="Refund Purchase" onClose=${() => { setShowRefund(false); setRefundTarget(null); }} footer=${refundFooter}>
				${refundTarget ? html`
					<div>
						<p style=${{ fontSize: '0.875rem', color: '#4b5563', marginBottom: '1rem' }}>
							Refund purchase <strong>#${refundTarget.id}</strong>
							${refundTarget.total_amount != null ? ` (${refundTarget.currency || 'USD'} ${Number(refundTarget.total_amount).toFixed(2)})` : ''}
						</p>
						<div style=${fieldStyle}>
							<label style=${labelStyle}>Reason</label>
							<textarea style=${{ ...inputStyle, minHeight: '80px' }} value=${refundReason} onInput=${(e: any) => setRefundReason(e.target.value)} placeholder="Reason for refund"></textarea>
						</div>
					</div>
				` : null}
			<//>
		</div>
	`;
}
