import { html, PageHeader, DataTable, SearchInput, Button, Modal, ConfirmDialog, LoadingSpinner, api, toasts } from '@solobase/ui';
import { useState, useEffect, useCallback } from 'preact/hooks';
import { Plus, Edit2, Trash2 } from 'lucide-preact';

const inputStyle = { width: '100%', padding: '0.5rem 0.75rem', border: '1px solid #e2e8f0', borderRadius: '8px', fontSize: '0.813rem', outline: 'none', boxSizing: 'border-box' as const };
const labelStyle = { display: 'block', fontSize: '0.813rem', fontWeight: 500, color: '#1e293b', marginBottom: '0.375rem' };
const fieldStyle = { marginBottom: '1rem' };

const emptyProduct = { name: '', description: '', group_id: '', base_price: 0, currency: 'USD', pricing_formula: '', active: true, status: 'draft' };

export function ProductsTab() {
	const [products, setProducts] = useState<any[]>([]);
	const [groups, setGroups] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [search, setSearch] = useState('');
	const [showModal, setShowModal] = useState(false);
	const [editing, setEditing] = useState<any>(null);
	const [showDelete, setShowDelete] = useState(false);
	const [toDelete, setToDelete] = useState<any>(null);
	const [saving, setSaving] = useState(false);

	const load = useCallback(async () => {
		setLoading(true);
		try {
			const [prodData, groupData] = await Promise.all([
				api.get('/admin/b/products/products').catch(() => ({})),
				api.get('/admin/b/products/groups').catch(() => ({})),
			]);
			setProducts(Array.isArray(prodData?.records) ? prodData.records : Array.isArray(prodData) ? prodData : []);
			setGroups(Array.isArray(groupData?.records) ? groupData.records : Array.isArray(groupData) ? groupData : []);
		} catch { /* ignore */ }
		setLoading(false);
	}, []);

	useEffect(() => { load(); }, [load]);

	async function save() {
		if (!editing?.name?.trim()) { toasts.error('Name is required'); return; }
		setSaving(true);
		try {
			if (editing.id) {
				await api.put(`/admin/b/products/products/${editing.id}`, editing);
				toasts.success('Product updated');
			} else {
				await api.post('/admin/b/products/products', editing);
				toasts.success('Product created');
			}
			setShowModal(false);
			setEditing(null);
			await load();
		} catch (err: any) {
			toasts.error(err.message || 'Failed to save product');
		}
		setSaving(false);
	}

	async function handleDelete() {
		if (!toDelete) return;
		try {
			await api.delete(`/admin/b/products/products/${toDelete.id}`);
			toasts.success('Product deleted');
			setShowDelete(false);
			setToDelete(null);
			await load();
		} catch (err: any) {
			toasts.error(err.message || 'Failed to delete product');
		}
	}

	const filtered = search ? products.filter(p => p.name?.toLowerCase().includes(search.toLowerCase())) : products;

	const columns = [
		{ key: 'name', label: 'Product', sortable: true },
		{ key: 'group_id', label: 'Group', render: (v: any) => { const g = groups.find((g: any) => String(g.id) === String(v)); return g?.name || v || '-'; } },
		{ key: 'base_price', label: 'Price', render: (v: any, row: any) => v != null ? `${row.currency || 'USD'} ${Number(v).toFixed(2)}` : '-' },
		{ key: 'status', label: 'Status', render: (v: string) => html`
			<span style=${{ fontSize: '0.75rem', padding: '0.125rem 0.5rem', borderRadius: '9999px', background: v === 'active' ? '#dcfce7' : '#f3f4f6', color: v === 'active' ? '#166534' : '#6b7280' }}>${v || 'draft'}</span>
		` },
		{ key: 'created_at', label: 'Created', sortable: true, render: (v: string) => v ? new Date(v).toLocaleDateString() : '-' },
		{ key: '_actions', label: '', width: '80px', render: (_: any, row: any) => html`
			<div style=${{ display: 'flex', gap: '0.25rem' }}>
				<button onClick=${(e: Event) => { e.stopPropagation(); setEditing({ ...row }); setShowModal(true); }} style=${{ background: 'none', border: 'none', cursor: 'pointer', color: '#64748b', padding: '0.25rem' }} type="button"><${Edit2} size=${14} /></button>
				<button onClick=${(e: Event) => { e.stopPropagation(); setToDelete(row); setShowDelete(true); }} style=${{ background: 'none', border: 'none', cursor: 'pointer', color: '#dc2626', padding: '0.25rem' }} type="button"><${Trash2} size=${14} /></button>
			</div>
		` },
	];

	if (loading) return html`<${LoadingSpinner} message="Loading products..." />`;

	const footer = html`
		<${Button} variant="secondary" onClick=${() => { setShowModal(false); setEditing(null); }}>Cancel<//>
		<${Button} onClick=${save} loading=${saving}>${editing?.id ? 'Save Changes' : 'Create Product'}<//>
	`;

	return html`
		<div>
			<${PageHeader} title="Products" description="Manage your product catalog">
				<${Button} icon=${Plus} onClick=${() => { setEditing({ ...emptyProduct }); setShowModal(true); }}>Add Product<//>
			<//>
			<${SearchInput} value=${search} onChange=${setSearch} placeholder="Search products..." />
			<${DataTable} columns=${columns} data=${filtered} emptyMessage="No products yet" />

			<${Modal} show=${showModal} title=${editing?.id ? 'Edit Product' : 'Add Product'} onClose=${() => { setShowModal(false); setEditing(null); }} footer=${footer}>
				${editing ? html`
					<div>
						<div style=${fieldStyle}>
							<label style=${labelStyle}>Name *</label>
							<input style=${inputStyle} value=${editing.name} onInput=${(e: any) => setEditing({ ...editing, name: e.target.value })} placeholder="Product name" />
						</div>
						<div style=${fieldStyle}>
							<label style=${labelStyle}>Description</label>
							<textarea style=${{ ...inputStyle, minHeight: '60px' }} value=${editing.description} onInput=${(e: any) => setEditing({ ...editing, description: e.target.value })} placeholder="Product description"></textarea>
						</div>
						<div style=${fieldStyle}>
							<label style=${labelStyle}>Group</label>
							<select style=${inputStyle} value=${editing.group_id} onChange=${(e: any) => setEditing({ ...editing, group_id: e.target.value })}>
								<option value="">Select group...</option>
								${groups.map((g: any) => html`<option value=${g.id}>${g.name}</option>`)}
							</select>
						</div>
						<div style=${{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem', ...fieldStyle }}>
							<div>
								<label style=${labelStyle}>Base Price</label>
								<input style=${inputStyle} type="number" step="0.01" value=${editing.base_price} onInput=${(e: any) => setEditing({ ...editing, base_price: parseFloat(e.target.value) || 0 })} />
							</div>
							<div>
								<label style=${labelStyle}>Currency</label>
								<input style=${inputStyle} value=${editing.currency} onInput=${(e: any) => setEditing({ ...editing, currency: e.target.value })} placeholder="USD" />
							</div>
						</div>
						<div style=${fieldStyle}>
							<label style=${labelStyle}>Pricing Formula</label>
							<input style=${{ ...inputStyle, fontFamily: "'SF Mono', 'Fira Code', monospace" }} value=${editing.pricing_formula} onInput=${(e: any) => setEditing({ ...editing, pricing_formula: e.target.value })} placeholder="e.g. base_price * quantity" />
						</div>
						<div style=${fieldStyle}>
							<label style=${labelStyle}>Status</label>
							<select style=${inputStyle} value=${editing.status} onChange=${(e: any) => setEditing({ ...editing, status: e.target.value })}>
								<option value="draft">Draft</option>
								<option value="active">Active</option>
							</select>
						</div>
						<div>
							<label style=${{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.813rem', cursor: 'pointer' }}>
								<input type="checkbox" checked=${editing.active} onChange=${(e: any) => setEditing({ ...editing, active: e.target.checked })} /> Active
							</label>
						</div>
					</div>
				` : null}
			<//>

			<${ConfirmDialog}
				show=${showDelete}
				title="Delete Product"
				message=${`Are you sure you want to delete "${toDelete?.name}"? This action cannot be undone.`}
				confirmText="Delete"
				onConfirm=${handleDelete}
				onCancel=${() => { setShowDelete(false); setToDelete(null); }}
			/>
		</div>
	`;
}
