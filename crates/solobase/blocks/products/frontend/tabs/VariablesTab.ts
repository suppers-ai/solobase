import { html, PageHeader, DataTable, SearchInput, Button, Modal, ConfirmDialog, LoadingSpinner, api, toasts } from '@solobase/ui';
import { useState, useEffect, useCallback } from 'preact/hooks';
import { Plus, Edit2, Trash2, Lock } from 'lucide-preact';

const inputStyle = { width: '100%', padding: '0.5rem 0.75rem', border: '1px solid #e2e8f0', borderRadius: '8px', fontSize: '0.813rem', outline: 'none', boxSizing: 'border-box' as const };
const labelStyle = { display: 'block', fontSize: '0.813rem', fontWeight: 500, color: '#1e293b', marginBottom: '0.375rem' };
const fieldStyle = { marginBottom: '1rem' };

const emptyVar = { name: '', display_name: '', description: '', type: 'user', value_type: 'number', default_value: 0, status: 'active' };

export function VariablesTab() {
	const [variables, setVariables] = useState<any[]>([]);
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
			const data = await api.get('/admin/b/products/variables');
			setVariables(Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : []);
		} catch { /* ignore */ }
		setLoading(false);
	}, []);

	useEffect(() => { load(); }, [load]);

	async function save() {
		if (!editing?.name?.trim()) { toasts.error('Name is required'); return; }
		setSaving(true);
		try {
			if (editing.id) {
				await api.put(`/admin/b/products/variables/${editing.id}`, editing);
				toasts.success('Variable updated');
			} else {
				await api.post('/admin/b/products/variables', editing);
				toasts.success('Variable created');
			}
			setShowModal(false);
			setEditing(null);
			await load();
		} catch (err: any) {
			toasts.error(err.message || 'Failed to save variable');
		}
		setSaving(false);
	}

	async function handleDelete() {
		if (!toDelete) return;
		try {
			await api.delete(`/admin/b/products/variables/${toDelete.id}`);
			toasts.success('Variable deleted');
			setShowDelete(false);
			setToDelete(null);
			await load();
		} catch (err: any) {
			toasts.error(err.message || 'Failed to delete variable');
		}
	}

	const filtered = search ? variables.filter(v =>
		v.name?.toLowerCase().includes(search.toLowerCase()) ||
		v.display_name?.toLowerCase().includes(search.toLowerCase())
	) : variables;

	const columns = [
		{ key: 'name', label: 'Name', sortable: true, render: (v: string) => html`<code style=${{ fontSize: '0.75rem', background: '#f1f5f9', padding: '0.125rem 0.375rem', borderRadius: '4px' }}>${v}</code>` },
		{ key: 'display_name', label: 'Display Name', render: (v: string) => v || '-' },
		{ key: 'type', label: 'Type', render: (v: string) => html`
			<span style=${{ fontSize: '0.75rem', padding: '0.125rem 0.5rem', borderRadius: '9999px', background: v === 'system' ? '#fefce8' : '#dcfce7', color: v === 'system' ? '#854d0e' : '#166534', display: 'inline-flex', alignItems: 'center', gap: '0.25rem' }}>
				${v === 'system' ? html`<${Lock} size=${10} />` : null} ${v || 'user'}
			</span>
		` },
		{ key: 'value_type', label: 'Value Type', render: (v: string) => v || '-' },
		{ key: 'default_value', label: 'Default', render: (v: any) => v != null ? html`<code style=${{ fontSize: '0.75rem' }}>${String(v)}</code>` : '-' },
		{ key: 'status', label: 'Status', render: (v: string) => html`
			<span style=${{ fontSize: '0.75rem', padding: '0.125rem 0.5rem', borderRadius: '9999px', background: v === 'active' ? '#dcfce7' : '#f3f4f6', color: v === 'active' ? '#166534' : '#6b7280' }}>${v || 'active'}</span>
		` },
		{ key: '_actions', label: '', width: '80px', render: (_: any, row: any) => row.type === 'system' ? null : html`
			<div style=${{ display: 'flex', gap: '0.25rem' }}>
				<button onClick=${(e: Event) => { e.stopPropagation(); setEditing({ ...row }); setShowModal(true); }} style=${{ background: 'none', border: 'none', cursor: 'pointer', color: '#64748b', padding: '0.25rem' }} type="button"><${Edit2} size=${14} /></button>
				<button onClick=${(e: Event) => { e.stopPropagation(); setToDelete(row); setShowDelete(true); }} style=${{ background: 'none', border: 'none', cursor: 'pointer', color: '#dc2626', padding: '0.25rem' }} type="button"><${Trash2} size=${14} /></button>
			</div>
		` },
	];

	if (loading) return html`<${LoadingSpinner} message="Loading variables..." />`;

	const footer = html`
		<${Button} variant="secondary" onClick=${() => { setShowModal(false); setEditing(null); }}>Cancel<//>
		<${Button} onClick=${save} loading=${saving}>${editing?.id ? 'Save Changes' : 'Create Variable'}<//>
	`;

	return html`
		<div>
			<${PageHeader} title="Variables" description="Define variables for pricing formulas">
				<${Button} icon=${Plus} onClick=${() => { setEditing({ ...emptyVar }); setShowModal(true); }}>Add Variable<//>
			<//>
			<${SearchInput} value=${search} onChange=${setSearch} placeholder="Search variables..." />
			<${DataTable} columns=${columns} data=${filtered} emptyMessage="No variables defined" />

			<${Modal} show=${showModal} title=${editing?.id ? 'Edit Variable' : 'Add Variable'} onClose=${() => { setShowModal(false); setEditing(null); }} footer=${footer}>
				${editing ? html`
					<div>
						<div style=${fieldStyle}>
							<label style=${labelStyle}>Name *</label>
							<input style=${{ ...inputStyle, fontFamily: "'SF Mono', 'Fira Code', monospace" }} value=${editing.name} onInput=${(e: any) => setEditing({ ...editing, name: e.target.value })} placeholder="variable_name" />
						</div>
						<div style=${fieldStyle}>
							<label style=${labelStyle}>Display Name</label>
							<input style=${inputStyle} value=${editing.display_name} onInput=${(e: any) => setEditing({ ...editing, display_name: e.target.value })} placeholder="Display Name" />
						</div>
						<div style=${fieldStyle}>
							<label style=${labelStyle}>Description</label>
							<textarea style=${{ ...inputStyle, minHeight: '60px' }} value=${editing.description} onInput=${(e: any) => setEditing({ ...editing, description: e.target.value })} placeholder="Variable description"></textarea>
						</div>
						<div style=${{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem', ...fieldStyle }}>
							<div>
								<label style=${labelStyle}>Value Type</label>
								<select style=${inputStyle} value=${editing.value_type} onChange=${(e: any) => setEditing({ ...editing, value_type: e.target.value })}>
									<option value="number">Number</option>
									<option value="string">String</option>
									<option value="boolean">Boolean</option>
								</select>
							</div>
							<div>
								<label style=${labelStyle}>Default Value</label>
								<input style=${inputStyle} value=${editing.default_value != null ? String(editing.default_value) : ''} onInput=${(e: any) => {
									const val = e.target.value;
									const num = parseFloat(val);
									setEditing({ ...editing, default_value: isNaN(num) ? val : num });
								}} placeholder="Default value" />
							</div>
						</div>
						<div style=${fieldStyle}>
							<label style=${labelStyle}>Status</label>
							<select style=${inputStyle} value=${editing.status} onChange=${(e: any) => setEditing({ ...editing, status: e.target.value })}>
								<option value="active">Active</option>
								<option value="pending">Pending</option>
							</select>
						</div>
					</div>
				` : null}
			<//>

			<${ConfirmDialog}
				show=${showDelete}
				title="Delete Variable"
				message=${`Are you sure you want to delete "${toDelete?.display_name || toDelete?.name}"? This action cannot be undone.`}
				confirmText="Delete"
				onConfirm=${handleDelete}
				onCancel=${() => { setShowDelete(false); setToDelete(null); }}
			/>
		</div>
	`;
}
