import { html, PageHeader, DataTable, SearchInput, Button, Modal, ConfirmDialog, LoadingSpinner, api, toasts } from '@solobase/ui';
import { useState, useEffect, useCallback } from 'preact/hooks';
import { Plus, Edit2, Trash2 } from 'lucide-preact';

const inputStyle = { width: '100%', padding: '0.5rem 0.75rem', border: '1px solid #e2e8f0', borderRadius: '8px', fontSize: '0.813rem', outline: 'none', boxSizing: 'border-box' as const };
const labelStyle = { display: 'block', fontSize: '0.813rem', fontWeight: 500, color: '#1e293b', marginBottom: '0.375rem' };
const fieldStyle = { marginBottom: '1rem' };

const emptyGroup = { name: '', description: '' };

export function GroupsTab() {
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
			const data = await api.get('/admin/b/products/groups');
			setGroups(Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : []);
		} catch { /* ignore */ }
		setLoading(false);
	}, []);

	useEffect(() => { load(); }, [load]);

	async function save() {
		if (!editing?.name?.trim()) { toasts.error('Name is required'); return; }
		setSaving(true);
		try {
			if (editing.id) {
				await api.put(`/admin/b/products/groups/${editing.id}`, editing);
				toasts.success('Group updated');
			} else {
				await api.post('/admin/b/products/groups', editing);
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
			await api.delete(`/admin/b/products/groups/${toDelete.id}`);
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
		{ key: 'user_id', label: 'Owner', render: (v: string) => v || '-' },
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
			<${PageHeader} title="Groups" description="Organize products into groups">
				<${Button} icon=${Plus} onClick=${() => { setEditing({ ...emptyGroup }); setShowModal(true); }}>Add Group<//>
			<//>
			<${SearchInput} value=${search} onChange=${setSearch} placeholder="Search groups..." />
			<${DataTable} columns=${columns} data=${filtered} emptyMessage="No groups yet" />

			<${Modal} show=${showModal} title=${editing?.id ? 'Edit Group' : 'Add Group'} onClose=${() => { setShowModal(false); setEditing(null); }} footer=${footer}>
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
				onConfirm=${handleDelete}
				onCancel=${() => { setShowDelete(false); setToDelete(null); }}
			/>
		</div>
	`;
}
