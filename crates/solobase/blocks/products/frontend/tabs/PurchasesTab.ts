import { html } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { CheckCircle, AlertCircle, X, RotateCcw, Check } from 'lucide-preact';
import { authFetch, Modal, SearchInput, Pagination } from '@solobase/ui';

export function PurchasesTab() {
	const [purchases, setPurchases] = useState<any[]>([]);
	const [total, setTotal] = useState(0);
	const [loading, setLoading] = useState(true);
	const [statusFilter, setStatusFilter] = useState('all');
	const [currentPage, setCurrentPage] = useState(1);
	const [pageSize] = useState(20);
	const [showRefundModal, setShowRefundModal] = useState(false);
	const [refundTarget, setRefundTarget] = useState<any>(null);
	const [refundAmount, setRefundAmount] = useState('');
	const [refundReason, setRefundReason] = useState('');
	const [notification, setNotification] = useState<{ message: string; type: string } | null>(null);

	useEffect(() => { loadData(); }, [currentPage]);

	function showNotif(message: string, type = 'info') {
		setNotification({ message, type });
		setTimeout(() => setNotification(null), 3000);
	}

	async function loadData() {
		setLoading(true);
		try {
			const offset = (currentPage - 1) * pageSize;
			const response = await authFetch(`/api/admin/ext/products/purchases?limit=${pageSize}&offset=${offset}`);
			if (response.ok) {
				const data = await response.json();
				setPurchases(data.purchases || []);
				setTotal(data.total || 0);
			}
		} catch { /* ignore */ }
		setLoading(false);
	}

	async function approvePurchase(id: number) {
		try {
			const response = await authFetch(`/api/admin/ext/products/purchases/${id}/approve`, { method: 'POST' });
			if (response.ok) {
				showNotif('Purchase approved', 'success');
				loadData();
			} else { showNotif('Failed to approve purchase', 'error'); }
		} catch { showNotif('Failed to approve purchase', 'error'); }
	}

	async function refundPurchase() {
		if (!refundTarget) return;
		try {
			const response = await authFetch(`/api/admin/ext/products/purchases/${refundTarget.id}/refund`, {
				method: 'POST',
				body: JSON.stringify({
					amount: refundAmount ? parseInt(refundAmount) : 0,
					reason: refundReason,
				}),
			});
			if (response.ok) {
				showNotif('Refund processed', 'success');
				setShowRefundModal(false);
				setRefundTarget(null);
				setRefundAmount('');
				setRefundReason('');
				loadData();
			} else { showNotif('Failed to process refund', 'error'); }
		} catch { showNotif('Failed to process refund', 'error'); }
	}

	function formatCents(cents: number, currency: string) {
		return `${currency || 'USD'} ${(cents / 100).toFixed(2)}`;
	}

	function getStatusClass(status: string) {
		switch (status) {
			case 'paid': case 'paid_pending_approval': return 'status-paid';
			case 'pending': case 'requires_approval': return 'status-pending';
			case 'cancelled': return 'status-cancelled';
			case 'refunded': return 'status-refunded';
			default: return 'status-inactive';
		}
	}

	const filtered = purchases.filter(p =>
		statusFilter === 'all' || p.status === statusFilter
	);

	const totalPages = Math.ceil(total / pageSize);

	return html`
		<>
			${notification ? html`
				<div class="notification notification-${notification.type}">
					<div class="notification-content">
						${notification.type === 'success' ? html`<${CheckCircle} size=${20} />` : html`<${AlertCircle} size=${20} />`}
						<span>${notification.message}</span>
					</div>
					<button class="notification-close" onClick=${() => setNotification(null)} type="button"><${X} size=${16} /></button>
				</div>
			` : null}

			<div class="card">
				<div class="section-header">
					<div class="section-filters">
						<select class="filter-select" value=${statusFilter} onChange=${(e: Event) => setStatusFilter((e.target as HTMLSelectElement).value)}>
							<option value="all">All Status</option>
							<option value="pending">Pending</option>
							<option value="paid">Paid</option>
							<option value="requires_approval">Requires Approval</option>
							<option value="paid_pending_approval">Paid Pending Approval</option>
							<option value="refunded">Refunded</option>
							<option value="cancelled">Cancelled</option>
						</select>
					</div>
				</div>

				<div class="table-container">
					<table class="data-table">
						<thead><tr><th>ID</th><th>User</th><th>Amount</th><th>Status</th><th>Provider</th><th>Date</th><th style=${{ width: '100px' }}>Actions</th></tr></thead>
						<tbody>
							${filtered.map(p => html`
								<tr key=${p.id}>
									<td class="cell-mono">#${p.id}</td>
									<td class="text-muted">${p.userId || '-'}</td>
									<td class="cell-mono">${formatCents(p.totalCents || p.amountCents || 0, p.currency)}</td>
									<td>
										<span class="status-badge ${getStatusClass(p.status)}">
											${p.status?.replace(/_/g, ' ') || '-'}
										</span>
									</td>
									<td class="text-muted">${p.provider || '-'}</td>
									<td class="text-muted">${p.createdAt ? new Date(p.createdAt).toLocaleDateString() : '-'}</td>
									<td><div class="action-buttons">
										${(p.status === 'requires_approval' || p.status === 'paid_pending_approval') ? html`
											<button class="btn-icon-sm" title="Approve" onClick=${() => approvePurchase(p.id)} type="button"><${Check} size=${14} /></button>
										` : null}
										${p.status === 'paid' ? html`
											<button class="btn-icon-sm" title="Refund" onClick=${() => { setRefundTarget(p); setRefundAmount(String(p.totalCents || p.amountCents || 0)); setShowRefundModal(true); }} type="button"><${RotateCcw} size=${14} /></button>
										` : null}
									</div></td>
								</tr>
							`)}
							${filtered.length === 0 ? html`<tr><td class="empty-row" colspan="7">${loading ? 'Loading...' : 'No purchases found'}</td></tr>` : null}
						</tbody>
					</table>
				</div>

				${totalPages > 1 ? html`<${Pagination} currentPage=${currentPage} totalPages=${totalPages} totalItems=${total} pageSize=${pageSize} onChange=${(page: number) => setCurrentPage(page)} />` : null}
			</div>

			${showRefundModal && refundTarget ? html`
				<${Modal} title="Refund Purchase" onClose=${() => { setShowRefundModal(false); setRefundTarget(null); }}>
					<p>Refund purchase <strong>#${refundTarget.id}</strong> (${formatCents(refundTarget.totalCents || refundTarget.amountCents || 0, refundTarget.currency)})</p>
					<div class="form-group">
						<label class="form-label">Refund Amount (cents)</label>
						<input class="form-input" type="number" value=${refundAmount} onInput=${(e: Event) => setRefundAmount((e.target as HTMLInputElement).value)} placeholder="Amount in cents" />
					</div>
					<div class="form-group">
						<label class="form-label">Reason</label>
						<textarea class="form-input" rows="2" value=${refundReason} onInput=${(e: Event) => setRefundReason((e.target as HTMLTextAreaElement).value)} placeholder="Refund reason"></textarea>
					</div>
					<div class="form-actions">
						<button class="btn btn-secondary" onClick=${() => { setShowRefundModal(false); setRefundTarget(null); }} type="button">Cancel</button>
						<button class="btn btn-danger" onClick=${refundPurchase} type="button">Process Refund</button>
					</div>
				</${Modal}>
			` : null}
		<//>
	`;
}
