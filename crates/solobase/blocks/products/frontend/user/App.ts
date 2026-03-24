import {
	html, api, checkAuth, isAuthenticated, authLoading, currentUser, userRoles, logout,
	LoadingSpinner, PageHeader, StatCard, DataTable, SearchInput, Button, Modal, ConfirmDialog,
	EmptyState, StatusBadge, TabNavigation, ToastContainer, toasts
} from '@solobase/ui';
import { useState, useEffect, useCallback } from 'preact/hooks';
import {
	Package, FolderOpen, CreditCard, Receipt, LogOut, Shield, ArrowLeft,
	Plus, Edit2, Trash2, Eye
} from 'lucide-preact';

/** Flatten a wafer Record { id, data: { ... } } into a plain object { id, ... }. */
function flatRecord(r: any) { return r?.data ? { id: r.id, ...r.data } : r; }
function flatRecords(arr: any[]) { return arr.map(flatRecord); }

const inputStyle = { width: '100%', padding: '0.5rem 0.75rem', border: '1px solid #e2e8f0', borderRadius: '8px', fontSize: '0.813rem', outline: 'none', boxSizing: 'border-box' as const };
const labelStyle = { display: 'block', fontSize: '0.813rem', fontWeight: 500, color: '#1e293b', marginBottom: '0.375rem' };
const fieldStyle = { marginBottom: '1rem' };

// ─── Auth Guard ──────────────────────────────────────────────────────
function AuthGuard({ children }: { children: any }) {
	const [checked, setChecked] = useState(false);

	useEffect(() => {
		checkAuth().then(() => setChecked(true));
	}, []);

	if (!checked || authLoading.value) {
		return html`<div style=${{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh' }}><${LoadingSpinner} message="Loading..." /></div>`;
	}

	if (!isAuthenticated.value) {
		window.location.href = '/blocks/dashboard/frontend/index.html';
		return null;
	}

	return children;
}

// ─── Header ──────────────────────────────────────────────────────────
function ProductsHeader() {
	const user = currentUser.value;
	const roles = userRoles.value;
	const isAdmin = Array.isArray(roles) && roles.includes('admin');

	return html`
		<header style=${{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '1rem 1.5rem', background: 'white', borderBottom: '1px solid #e2e8f0' }}>
			<div style=${{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
				<a href="/blocks/dashboard/frontend/index.html" style=${{ display: 'flex', alignItems: 'center', gap: '0.25rem', fontSize: '0.813rem', color: '#64748b', textDecoration: 'none' }}>
					<${ArrowLeft} size=${16} /> Dashboard
				</a>
				<img src="/images/logo_long.png" alt="Solobase" style=${{ height: '32px', width: 'auto' }} />
			</div>
			<div style=${{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
				${isAdmin ? html`
					<a href="/blocks/admin/frontend/index.html" style=${{ display: 'flex', alignItems: 'center', gap: '0.25rem', fontSize: '0.813rem', color: '#fe6627', textDecoration: 'none', fontWeight: 600 }}>
						<${Shield} size=${16} /> Admin
					</a>
				` : null}
				<span style=${{ fontSize: '0.813rem', color: '#64748b' }}>${user?.email || ''}</span>
				<button onClick=${() => { logout(); }} style=${{ background: 'none', border: 'none', cursor: 'pointer', color: '#64748b', display: 'flex', alignItems: 'center', gap: '0.25rem', fontSize: '0.813rem' }}>
					<${LogOut} size=${16} /> Logout
				</button>
			</div>
		</header>
	`;
}

// ─── My Products Tab ─────────────────────────────────────────────────
const emptyProduct = { name: '', description: '', group_id: '', base_price: 0, currency: 'USD', status: 'draft' };

function MyProductsTab() {
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
				api.get('/b/products/products').catch(() => ({})),
				api.get('/b/products/groups').catch(() => ({})),
			]);
			setProducts(flatRecords(Array.isArray(prodData?.records) ? prodData.records : Array.isArray(prodData) ? prodData : []));
			setGroups(flatRecords(Array.isArray(groupData?.records) ? groupData.records : Array.isArray(groupData) ? groupData : []));
		} catch { /* ignore */ }
		setLoading(false);
	}, []);

	useEffect(() => { load(); }, [load]);

	async function save() {
		if (!editing?.name?.trim()) { toasts.error('Name is required'); return; }
		setSaving(true);
		try {
			if (editing.id) {
				await api.patch(`/b/products/products/${editing.id}`, editing);
				toasts.success('Product updated');
			} else {
				await api.post('/b/products/products', editing);
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
			await api.delete(`/b/products/products/${toDelete.id}`);
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
		{ key: 'group_id', label: 'Group', render: (v: any) => { const g = groups.find((g: any) => String(g.id) === String(v)); return g?.name || '-'; } },
		{ key: 'base_price', label: 'Price', render: (v: any, row: any) => v != null ? `${row.currency || 'USD'} ${Number(v).toFixed(2)}` : '-' },
		{ key: 'status', label: 'Status', render: (v: string) => html`
			<${StatusBadge} status=${v || 'draft'} variant=${v === 'active' ? 'success' : 'neutral'} />
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
			<${PageHeader} title="My Products" description=${`You have ${products.length} product${products.length !== 1 ? 's' : ''}`}>
				<${Button} icon=${Plus} onClick=${() => { setEditing({ ...emptyProduct }); setShowModal(true); }}>New Product<//>
			<//>

			${products.length === 0 && !search ? html`
				<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px' }}>
					<${EmptyState} icon=${Package} title="No products yet" description="Create your first product to get started.">
						<${Button} icon=${Plus} onClick=${() => { setEditing({ ...emptyProduct }); setShowModal(true); }}>Create Product<//>
					<//>
				</div>
			` : html`
				<div style=${{ marginBottom: '1rem' }}>
					<${SearchInput} value=${search} onChange=${setSearch} placeholder="Search products..." />
				</div>
				<${DataTable} columns=${columns} data=${filtered} emptyMessage="No products match your search" />
			`}

			<${Modal} show=${showModal} title=${editing?.id ? 'Edit Product' : 'New Product'} onClose=${() => { setShowModal(false); setEditing(null); }} footer=${footer}>
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
								<option value="">No group</option>
								${groups.map((g: any) => html`<option key=${g.id} value=${g.id}>${g.name}</option>`)}
							</select>
						</div>
						<div style=${{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem', ...fieldStyle }}>
							<div>
								<label style=${labelStyle}>Base Price</label>
								<input style=${inputStyle} type="number" step="0.01" min="0" value=${editing.base_price} onInput=${(e: any) => setEditing({ ...editing, base_price: parseFloat(e.target.value) || 0 })} />
							</div>
							<div>
								<label style=${labelStyle}>Currency</label>
								<input style=${inputStyle} value=${editing.currency} onInput=${(e: any) => setEditing({ ...editing, currency: e.target.value })} placeholder="USD" />
							</div>
						</div>
						<div style=${fieldStyle}>
							<label style=${labelStyle}>Status</label>
							<select style=${inputStyle} value=${editing.status} onChange=${(e: any) => setEditing({ ...editing, status: e.target.value })}>
								<option value="draft">Draft</option>
								<option value="active">Active</option>
							</select>
						</div>
					</div>
				` : null}
			<//>

			<${ConfirmDialog}
				show=${showDelete}
				title="Delete Product"
				message=${`Are you sure you want to delete "${toDelete?.name}"? This action cannot be undone.`}
				confirmText="Delete"
				variant="danger"
				onConfirm=${handleDelete}
				onCancel=${() => { setShowDelete(false); setToDelete(null); }}
			/>
		</div>
	`;
}

// ─── My Groups Tab ───────────────────────────────────────────────────
const emptyGroup = { name: '', description: '' };

function MyGroupsTab() {
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
			const data = await api.get('/b/products/groups');
			setGroups(flatRecords(Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : []));
		} catch { /* ignore */ }
		setLoading(false);
	}, []);

	useEffect(() => { load(); }, [load]);

	async function save() {
		if (!editing?.name?.trim()) { toasts.error('Name is required'); return; }
		setSaving(true);
		try {
			if (editing.id) {
				await api.patch(`/b/products/groups/${editing.id}`, editing);
				toasts.success('Group updated');
			} else {
				await api.post('/b/products/groups', editing);
				toasts.success('Group created');
			}
			setShowModal(false);
			setEditing(null);
			await load();
		} catch (err: any) {
			toasts.error(err.message || 'Failed to save group');
		}
		setSaving(false);
	}

	async function handleDelete() {
		if (!toDelete) return;
		try {
			await api.delete(`/b/products/groups/${toDelete.id}`);
			toasts.success('Group deleted');
			setShowDelete(false);
			setToDelete(null);
			await load();
		} catch (err: any) {
			toasts.error(err.message || 'Failed to delete group');
		}
	}

	const filtered = search ? groups.filter(g => g.name?.toLowerCase().includes(search.toLowerCase())) : groups;

	const columns = [
		{ key: 'name', label: 'Name', sortable: true },
		{ key: 'description', label: 'Description', render: (v: string) => v || '-' },
		{ key: 'created_at', label: 'Created', sortable: true, render: (v: string) => v ? new Date(v).toLocaleDateString() : '-' },
		{ key: '_actions', label: '', width: '80px', render: (_: any, row: any) => html`
			<div style=${{ display: 'flex', gap: '0.25rem' }}>
				<button onClick=${(e: Event) => { e.stopPropagation(); setEditing({ ...row }); setShowModal(true); }} style=${{ background: 'none', border: 'none', cursor: 'pointer', color: '#64748b', padding: '0.25rem' }} type="button"><${Edit2} size=${14} /></button>
				<button onClick=${(e: Event) => { e.stopPropagation(); setToDelete(row); setShowDelete(true); }} style=${{ background: 'none', border: 'none', cursor: 'pointer', color: '#dc2626', padding: '0.25rem' }} type="button"><${Trash2} size=${14} /></button>
			</div>
		` },
	];

	if (loading) return html`<${LoadingSpinner} message="Loading groups..." />`;

	const footer = html`
		<${Button} variant="secondary" onClick=${() => { setShowModal(false); setEditing(null); }}>Cancel<//>
		<${Button} onClick=${save} loading=${saving}>${editing?.id ? 'Save Changes' : 'Create Group'}<//>
	`;

	return html`
		<div>
			<${PageHeader} title="My Groups" description=${`You have ${groups.length} group${groups.length !== 1 ? 's' : ''}`}>
				<${Button} icon=${Plus} onClick=${() => { setEditing({ ...emptyGroup }); setShowModal(true); }}>New Group<//>
			<//>

			${groups.length === 0 && !search ? html`
				<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px' }}>
					<${EmptyState} icon=${FolderOpen} title="No groups yet" description="Groups help you organize your products. Create your first group.">
						<${Button} icon=${Plus} onClick=${() => { setEditing({ ...emptyGroup }); setShowModal(true); }}>Create Group<//>
					<//>
				</div>
			` : html`
				<div style=${{ marginBottom: '1rem' }}>
					<${SearchInput} value=${search} onChange=${setSearch} placeholder="Search groups..." />
				</div>
				<${DataTable} columns=${columns} data=${filtered} emptyMessage="No groups match your search" />
			`}

			<${Modal} show=${showModal} title=${editing?.id ? 'Edit Group' : 'New Group'} onClose=${() => { setShowModal(false); setEditing(null); }} footer=${footer}>
				${editing ? html`
					<div>
						<div style=${fieldStyle}>
							<label style=${labelStyle}>Name *</label>
							<input style=${inputStyle} value=${editing.name} onInput=${(e: any) => setEditing({ ...editing, name: e.target.value })} placeholder="Group name" />
						</div>
						<div style=${fieldStyle}>
							<label style=${labelStyle}>Description</label>
							<textarea style=${{ ...inputStyle, minHeight: '60px' }} value=${editing.description} onInput=${(e: any) => setEditing({ ...editing, description: e.target.value })} placeholder="Group description"></textarea>
						</div>
					</div>
				` : null}
			<//>

			<${ConfirmDialog}
				show=${showDelete}
				title="Delete Group"
				message=${`Are you sure you want to delete "${toDelete?.name}"? This action cannot be undone.`}
				confirmText="Delete"
				variant="danger"
				onConfirm=${handleDelete}
				onCancel=${() => { setShowDelete(false); setToDelete(null); }}
			/>
		</div>
	`;
}

// ─── Plans Tab ───────────────────────────────────────────────────────
function PlansTab() {
	const [products, setProducts] = useState<any[]>([]);
	const [purchases, setPurchases] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [subscribing, setSubscribing] = useState<string | null>(null);

	useEffect(() => {
		Promise.all([
			api.get('/b/products/catalog').catch(() => ({})),
			api.get('/b/products/purchases').catch(() => ({})),
		]).then(([catalogData, purchaseData]: any[]) => {
			setProducts(flatRecords(Array.isArray(catalogData?.records) ? catalogData.records : Array.isArray(catalogData) ? catalogData : []));
			setPurchases(flatRecords(Array.isArray(purchaseData?.records) ? purchaseData.records : Array.isArray(purchaseData) ? purchaseData : []));
			setLoading(false);
		});
	}, []);

	const completedPurchases = purchases.filter((p: any) => p.status === 'completed');
	const currentPlanId = completedPurchases.length > 0
		? completedPurchases[completedPurchases.length - 1].product_id
		: null;

	async function handleSubscribe(productId: string) {
		setSubscribing(productId);
		try {
			const purchaseRes: any = await api.post('/b/products/purchases', {
				items: [{ product_id: productId, quantity: 1, variables: {} }]
			});
			const purchaseId = purchaseRes.id || purchaseRes.data?.id;
			if (!purchaseId) {
				toasts.error('Failed to create purchase');
				setSubscribing(null);
				return;
			}
			const checkoutRes: any = await api.post('/b/products/checkout', {
				purchase_id: purchaseId,
				success_url: window.location.href,
				cancel_url: window.location.href
			});
			const checkoutUrl = checkoutRes.checkout_url || checkoutRes.data?.checkout_url;
			if (checkoutUrl) {
				window.location.href = checkoutUrl;
			} else {
				toasts.success('Purchase created successfully');
				setSubscribing(null);
			}
		} catch (err: any) {
			toasts.error(err.message || 'Failed to subscribe');
			setSubscribing(null);
		}
	}

	if (loading) return html`<${LoadingSpinner} message="Loading plans..." />`;

	const fallbackPlans = [
		{ id: 'free', name: 'Free', description: 'Perfect for getting started', price: 0, features: ['1 deployment', '100MB storage', 'Community support'] },
		{ id: 'pro', name: 'Pro', description: 'For growing applications', price: 29, features: ['5 deployments', '10GB storage', 'Priority support', 'Custom domains'] },
		{ id: 'enterprise', name: 'Enterprise', description: 'For large-scale applications', price: 99, features: ['Unlimited deployments', '100GB storage', 'Dedicated support', 'SLA guarantee', 'SSO'] },
	];

	const plans = products.length > 0 ? products : fallbackPlans;

	function isCurrentPlan(plan: any): boolean {
		if (plan.id === currentPlanId) return true;
		if (!currentPlanId && (plan.price === 0 || plan.id === 'free')) return true;
		return false;
	}

	function getButtonLabel(plan: any): string {
		if (isCurrentPlan(plan)) return 'Current Plan';
		if (plan.price === 0 || plan.id === 'free') return currentPlanId ? 'Downgrade' : 'Current Plan';
		return 'Subscribe';
	}

	return html`
		<div>
			<${PageHeader} title="Choose a Plan" description="Select the plan that best fits your needs" />
			<div style=${{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))', gap: '1.5rem' }}>
				${plans.map((plan: any) => {
					const isCurrent = isCurrentPlan(plan);
					const isPro = plan.name === 'Pro';
					const isPaid = plan.price > 0 && plan.id !== 'free';
					const buttonLabel = getButtonLabel(plan);
					const isSubscribing = subscribing === plan.id;

					return html`
						<div key=${plan.id} style=${{
							background: 'white', border: isPro ? '2px solid #fe6627' : '1px solid #e2e8f0',
							borderRadius: '12px', padding: '1.5rem', display: 'flex', flexDirection: 'column', position: 'relative'
						}}>
							${isCurrent ? html`
								<span style=${{ position: 'absolute', top: '-10px', left: '50%', transform: 'translateX(-50%)', background: '#22c55e', color: 'white', fontSize: '0.688rem', fontWeight: 600, padding: '0.125rem 0.75rem', borderRadius: '9999px' }}>Current Plan</span>
							` : isPro ? html`
								<span style=${{ position: 'absolute', top: '-10px', left: '50%', transform: 'translateX(-50%)', background: '#fe6627', color: 'white', fontSize: '0.688rem', fontWeight: 600, padding: '0.125rem 0.75rem', borderRadius: '9999px' }}>Popular</span>
							` : null}
							<h3 style=${{ fontSize: '1.25rem', fontWeight: 700, color: '#1e293b' }}>${plan.name}</h3>
							<p style=${{ fontSize: '0.813rem', color: '#64748b', marginTop: '0.25rem', marginBottom: '1rem' }}>${plan.description}</p>
							<div style=${{ fontSize: '2rem', fontWeight: 700, color: '#1e293b', marginBottom: '1.5rem' }}>
								${plan.price != null ? (plan.price === 0 ? 'Free' : `$${plan.price}`) : 'Custom'}
								${plan.price > 0 ? html`<span style=${{ fontSize: '0.875rem', fontWeight: 400, color: '#64748b' }}>/mo</span>` : null}
							</div>
							<ul style=${{ listStyle: 'none', padding: 0, margin: '0 0 1.5rem', flex: 1 }}>
								${(plan.features || []).map((f: string) => html`
									<li key=${f} style=${{ fontSize: '0.813rem', color: '#374151', padding: '0.25rem 0', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
										<span style=${{ color: '#22c55e', fontWeight: 700 }}>✓</span> ${f}
									</li>
								`)}
							</ul>
							<button
								onClick=${() => { if (!isCurrent && isPaid) handleSubscribe(plan.id); }}
								disabled=${isCurrent || isSubscribing}
								style=${{
									width: '100%', padding: '0.625rem',
									border: isPro && !isCurrent ? 'none' : '1px solid #e2e8f0',
									borderRadius: '8px', fontSize: '0.813rem', fontWeight: 600,
									cursor: isCurrent || isSubscribing ? 'default' : 'pointer',
									background: isCurrent ? '#f1f5f9' : isPro ? 'linear-gradient(135deg, #fe6627, #fc4c03)' : 'white',
									color: isCurrent ? '#64748b' : isPro ? 'white' : '#1e293b',
									opacity: isSubscribing ? 0.7 : 1
								}}>
								${isSubscribing ? 'Processing...' : buttonLabel}
							</button>
						</div>
					`;
				})}
			</div>
		</div>
	`;
}

// ─── Purchases Tab ───────────────────────────────────────────────────
function PurchasesTab() {
	const [purchases, setPurchases] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		api.get('/b/products/purchases').then((data: any) => {
			const records = flatRecords(Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : []);
			setPurchases(records);
			setLoading(false);
		}).catch(() => setLoading(false));
	}, []);

	if (loading) return html`<${LoadingSpinner} message="Loading purchases..." />`;

	const completed = purchases.filter(p => p.status === 'completed');
	const currentPlan = completed.length > 0 ? completed[completed.length - 1] : null;

	return html`
		<div>
			<${PageHeader} title="My Purchases" description="Your purchase history and current plan" />

			<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px', padding: '1.5rem', marginBottom: '2rem' }}>
				<h3 style=${{ fontSize: '1rem', fontWeight: 600, color: '#1e293b', marginBottom: '1rem' }}>Current Plan</h3>
				${currentPlan ? html`
					<div style=${{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))', gap: '1rem' }}>
						<${StatCard} title="Plan" value=${currentPlan.product_name || currentPlan.name || 'Paid'} icon=${CreditCard} />
						<${StatCard} title="Amount" value=${currentPlan.total_amount != null ? '$' + Number(currentPlan.total_amount).toFixed(2) : '-'} icon=${Receipt} />
						<${StatCard} title="Since" value=${currentPlan.created_at ? new Date(currentPlan.created_at).toLocaleDateString() : '-'} icon=${CreditCard} />
					</div>
				` : html`
					<div style=${{ textAlign: 'center', padding: '2rem', color: '#64748b' }}>
						<p style=${{ fontSize: '0.875rem', marginBottom: '1rem' }}>You're on the Free plan.</p>
						<${Button} onClick=${() => { window.location.hash = 'plans'; }}>Browse Plans<//>
					</div>
				`}
			</div>

			${purchases.length > 0 ? html`
				<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px', padding: '1.5rem' }}>
					<h3 style=${{ fontSize: '1rem', fontWeight: 600, color: '#1e293b', marginBottom: '1rem' }}>Purchase History</h3>
					<div style=${{ display: 'grid', gap: '0.5rem' }}>
						${purchases.map((p: any) => html`
							<div key=${p.id} style=${{
								display: 'flex', justifyContent: 'space-between', alignItems: 'center',
								padding: '0.75rem 1rem', border: '1px solid #f1f5f9', borderRadius: '8px'
							}}>
								<div>
									<div style=${{ fontWeight: 600, fontSize: '0.875rem', color: '#1e293b' }}>${p.product_name || 'Purchase #' + p.id}</div>
									<div style=${{ fontSize: '0.75rem', color: '#64748b' }}>${p.created_at ? new Date(p.created_at).toLocaleDateString() : ''}</div>
								</div>
								<div style=${{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
									<span style=${{ fontSize: '0.875rem', fontWeight: 600, color: '#1e293b' }}>
										${p.total_amount != null ? '$' + Number(p.total_amount).toFixed(2) : '-'}
									</span>
									<${StatusBadge} status=${p.status || 'unknown'} variant=${p.status === 'completed' ? 'success' : p.status === 'pending' ? 'warning' : 'neutral'} />
								</div>
							</div>
						`)}
					</div>
				</div>
			` : html`
				<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px' }}>
					<${EmptyState} icon=${Receipt} title="No purchases yet" description="Your purchase history will appear here." />
				</div>
			`}
		</div>
	`;
}

// ─── Main App ────────────────────────────────────────────────────────
function ProductsApp() {
	const [page, setPage] = useState(() => window.location.hash.slice(1) || 'products');

	useEffect(() => { window.location.hash = page; }, [page]);
	useEffect(() => {
		function onHash() { setPage(window.location.hash.slice(1) || 'products'); }
		window.addEventListener('hashchange', onHash);
		return () => window.removeEventListener('hashchange', onHash);
	}, []);

	const tabs = [
		{ id: 'products', label: 'My Products', icon: Package },
		{ id: 'groups', label: 'My Groups', icon: FolderOpen },
		{ id: 'plans', label: 'Plans', icon: CreditCard },
		{ id: 'purchases', label: 'Purchases', icon: Receipt },
	];

	return html`
		<div style=${{ minHeight: '100vh', background: '#f8fafc' }}>
			<${ProductsHeader} />
			<nav style=${{ padding: '0 1.5rem', background: 'white', borderBottom: '1px solid #e2e8f0' }}>
				<${TabNavigation} tabs=${tabs} activeTab=${page} onTabChange=${setPage} />
			</nav>
			<main style=${{ padding: '1.5rem', maxWidth: '1200px', margin: '0 auto' }}>
				${page === 'products' ? html`<${MyProductsTab} />` : null}
				${page === 'groups' ? html`<${MyGroupsTab} />` : null}
				${page === 'plans' ? html`<${PlansTab} />` : null}
				${page === 'purchases' ? html`<${PurchasesTab} />` : null}
			</main>
			<${ToastContainer} />
		</div>
	`;
}

export function App() {
	return html`
		<${AuthGuard}>
			<${ProductsApp} />
		<//>
	`;
}
