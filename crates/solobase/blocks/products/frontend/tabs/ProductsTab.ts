import { html } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { Plus, Edit2, Trash2, CheckCircle, AlertCircle, X } from 'lucide-preact';
import { authFetch, Modal, SearchInput, Pagination } from '@solobase/ui';

export function ProductsTab() {
	const [products, setProducts] = useState<any[]>([]);
	const [groups, setGroups] = useState<any[]>([]);
	const [productTypes, setProductTypes] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [searchQuery, setSearchQuery] = useState('');
	const [showModal, setShowModal] = useState(false);
	const [showDeleteModal, setShowDeleteModal] = useState(false);
	const [editing, setEditing] = useState<any>(null);
	const [toDelete, setToDelete] = useState<any>(null);
	const [notification, setNotification] = useState<{ message: string; type: string } | null>(null);

	const emptyProduct = { name: '', description: '', groupId: 0, productTemplateId: 0, basePrice: 0, currency: 'USD', pricingFormula: '', active: true };

	useEffect(() => { loadData(); }, []);

	function showNotif(message: string, type = 'info') {
		setNotification({ message, type });
		setTimeout(() => setNotification(null), 3000);
	}

	async function loadData() {
		setLoading(true);
		try {
			const [prodRes, groupRes, typeRes] = await Promise.all([
				authFetch('/api/admin/ext/products/groups'),
				authFetch('/api/admin/ext/products/groups'),
				authFetch('/api/admin/ext/products/product-types'),
			]);
			// Products: load all from groups
			if (groupRes.ok) {
				const g = await groupRes.json();
				setGroups(g || []);
			}
			if (typeRes.ok) {
				const t = await typeRes.json();
				setProductTypes(t || []);
			}
			// Load products from user endpoint (admin reuses)
			const allProdsRes = await authFetch('/api/ext/products/products');
			if (allProdsRes.ok) {
				const p = await allProdsRes.json();
				setProducts(p || []);
			}
		} catch { /* ignore */ }
		setLoading(false);
	}

	async function saveProduct() {
		if (!editing) return;
		try {
			const method = editing.id ? 'PUT' : 'POST';
			const url = editing.id
				? `/api/admin/ext/products/products/${editing.id}`
				: '/api/admin/ext/products/products';
			const response = await authFetch(url, {
				method,
				body: JSON.stringify(editing),
			});
			if (response.ok) {
				showNotif(`Product ${editing.id ? 'updated' : 'created'} successfully`, 'success');
				setShowModal(false);
				setEditing(null);
				loadData();
			} else {
				const err = await response.json().catch(() => ({}));
				showNotif(err.message || 'Failed to save product', 'error');
			}
		} catch { showNotif('Failed to save product', 'error'); }
	}

	async function deleteProduct() {
		if (!toDelete) return;
		try {
			const response = await authFetch(`/api/admin/ext/products/products/${toDelete.id}`, { method: 'DELETE' });
			if (response.ok) {
				showNotif('Product deleted', 'success');
				setShowDeleteModal(false);
				setToDelete(null);
				loadData();
			} else { showNotif('Failed to delete product', 'error'); }
		} catch { showNotif('Failed to delete product', 'error'); }
	}

	const filtered = products.filter(p =>
		!searchQuery || p.name?.toLowerCase().includes(searchQuery.toLowerCase())
	);

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
						<${SearchInput} value=${searchQuery} onChange=${setSearchQuery} placeholder="Search products..." />
					</div>
					<div class="table-actions">
						<button class="btn btn-primary" onClick=${() => { setEditing({ ...emptyProduct }); setShowModal(true); }} type="button">
							<${Plus} size=${16} /> Add Product
						</button>
					</div>
				</div>

				<div class="table-container">
					<table class="data-table">
						<thead><tr><th>Name</th><th>Group</th><th>Type</th><th>Base Price</th><th>Status</th><th style=${{ width: '80px' }}>Actions</th></tr></thead>
						<tbody>
							${filtered.map(p => html`
								<tr key=${p.id}>
									<td><span class="cell-name">${p.name}</span></td>
									<td class="text-muted">${p.group?.name || p.groupId || '-'}</td>
									<td class="text-muted">${p.productTemplate?.displayName || p.productTemplateId || '-'}</td>
									<td class="cell-mono">${p.currency || 'USD'} ${(p.basePrice || 0).toFixed(2)}</td>
									<td>
										<span class="status-badge ${p.active ? 'status-active' : 'status-inactive'}">
											${p.active ? 'Active' : 'Inactive'}
										</span>
									</td>
									<td><div class="action-buttons">
										<button class="btn-icon-sm" title="Edit" onClick=${() => { setEditing({ ...p }); setShowModal(true); }} type="button"><${Edit2} size=${14} /></button>
										<button class="btn-icon-sm" title="Delete" onClick=${() => { setToDelete(p); setShowDeleteModal(true); }} type="button"><${Trash2} size=${14} /></button>
									</div></td>
								</tr>
							`)}
							${filtered.length === 0 ? html`<tr><td class="empty-row" colspan="6">${loading ? 'Loading...' : 'No products found'}</td></tr>` : null}
						</tbody>
					</table>
				</div>
			</div>

			${showModal && editing ? html`
				<${Modal} title=${editing.id ? 'Edit Product' : 'Add Product'} onClose=${() => { setShowModal(false); setEditing(null); }}>
					<div class="form-group">
						<label class="form-label">Name *</label>
						<input class="form-input" value=${editing.name} onInput=${(e: Event) => setEditing({ ...editing, name: (e.target as HTMLInputElement).value })} placeholder="Product name" />
					</div>
					<div class="form-group">
						<label class="form-label">Description</label>
						<textarea class="form-input" rows="2" value=${editing.description} onInput=${(e: Event) => setEditing({ ...editing, description: (e.target as HTMLTextAreaElement).value })} placeholder="Product description"></textarea>
					</div>
					<div class="form-group">
						<label class="form-label">Group</label>
						<select class="form-select" value=${editing.groupId} onChange=${(e: Event) => setEditing({ ...editing, groupId: parseInt((e.target as HTMLSelectElement).value) || 0 })}>
							<option value="0">Select group...</option>
							${groups.map((g: any) => html`<option value=${g.id}>${g.name}</option>`)}
						</select>
					</div>
					<div class="form-group">
						<label class="form-label">Product Type</label>
						<select class="form-select" value=${editing.productTemplateId} onChange=${(e: Event) => setEditing({ ...editing, productTemplateId: parseInt((e.target as HTMLSelectElement).value) || 0 })}>
							<option value="0">Select type...</option>
							${productTypes.map((t: any) => html`<option value=${t.id}>${t.displayName || t.name}</option>`)}
						</select>
					</div>
					<div class="form-group">
						<label class="form-label">Base Price</label>
						<input class="form-input" type="number" step="0.01" value=${editing.basePrice} onInput=${(e: Event) => setEditing({ ...editing, basePrice: parseFloat((e.target as HTMLInputElement).value) || 0 })} />
					</div>
					<div class="form-group">
						<label class="form-label">Currency</label>
						<input class="form-input" value=${editing.currency} onInput=${(e: Event) => setEditing({ ...editing, currency: (e.target as HTMLInputElement).value })} placeholder="USD" />
					</div>
					<div class="form-group">
						<label class="form-label">Pricing Formula</label>
						<input class="form-input" value=${editing.pricingFormula} onInput=${(e: Event) => setEditing({ ...editing, pricingFormula: (e.target as HTMLInputElement).value })} placeholder="e.g. base_price * quantity" style=${{ fontFamily: "'SF Mono', 'Fira Code', monospace", fontSize: '0.8125rem' }} />
					</div>
					<div class="form-group">
						<label class="checkbox-label"><input type="checkbox" checked=${editing.active} onChange=${(e: Event) => setEditing({ ...editing, active: (e.target as HTMLInputElement).checked })} /> Active</label>
					</div>
					<div class="form-actions">
						<button class="btn btn-secondary" onClick=${() => { setShowModal(false); setEditing(null); }} type="button">Cancel</button>
						<button class="btn btn-primary" onClick=${saveProduct} type="button">${editing.id ? 'Save Changes' : 'Create Product'}</button>
					</div>
				</${Modal}>
			` : null}

			${showDeleteModal && toDelete ? html`
				<${Modal} title="Delete Product" onClose=${() => { setShowDeleteModal(false); setToDelete(null); }}>
					<p>Are you sure you want to delete <strong>${toDelete.name}</strong>?</p>
					<p class="text-danger">This action cannot be undone.</p>
					<div class="form-actions">
						<button class="btn btn-secondary" onClick=${() => { setShowDeleteModal(false); setToDelete(null); }} type="button">Cancel</button>
						<button class="btn btn-danger" onClick=${deleteProduct} type="button">Delete</button>
					</div>
				</${Modal}>
			` : null}
		<//>
	`;
}
