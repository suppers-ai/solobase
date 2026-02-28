import { html } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { Plus, Edit2, Trash2, CheckCircle, AlertCircle, X } from 'lucide-preact';
import { authFetch, Modal, SearchInput } from '@solobase/ui';

export function GroupsTab() {
	const [groups, setGroups] = useState<any[]>([]);
	const [groupTypes, setGroupTypes] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [searchQuery, setSearchQuery] = useState('');
	const [showModal, setShowModal] = useState(false);
	const [showDeleteModal, setShowDeleteModal] = useState(false);
	const [editing, setEditing] = useState<any>(null);
	const [toDelete, setToDelete] = useState<any>(null);
	const [notification, setNotification] = useState<{ message: string; type: string } | null>(null);

	const emptyGroup = { name: '', description: '', groupTemplateId: 0 };

	useEffect(() => { loadData(); }, []);

	function showNotif(message: string, type = 'info') {
		setNotification({ message, type });
		setTimeout(() => setNotification(null), 3000);
	}

	async function loadData() {
		setLoading(true);
		try {
			const [groupRes, typeRes] = await Promise.all([
				authFetch('/api/admin/ext/products/groups'),
				authFetch('/api/admin/ext/products/group-types'),
			]);
			if (groupRes.ok) setGroups(await groupRes.json() || []);
			if (typeRes.ok) setGroupTypes(await typeRes.json() || []);
		} catch { /* ignore */ }
		setLoading(false);
	}

	async function saveGroup() {
		if (!editing) return;
		try {
			const method = editing.id ? 'PUT' : 'POST';
			const url = editing.id
				? `/api/ext/products/groups/${editing.id}`
				: '/api/ext/products/groups';
			const response = await authFetch(url, {
				method,
				body: JSON.stringify(editing),
			});
			if (response.ok) {
				showNotif(`Group ${editing.id ? 'updated' : 'created'}`, 'success');
				setShowModal(false);
				setEditing(null);
				loadData();
			} else {
				showNotif('Failed to save group', 'error');
			}
		} catch { showNotif('Failed to save group', 'error'); }
	}

	async function deleteGroup() {
		if (!toDelete) return;
		try {
			const response = await authFetch(`/api/ext/products/groups/${toDelete.id}`, { method: 'DELETE' });
			if (response.ok) {
				showNotif('Group deleted', 'success');
				setShowDeleteModal(false);
				setToDelete(null);
				loadData();
			} else { showNotif('Failed to delete group', 'error'); }
		} catch { showNotif('Failed to delete group', 'error'); }
	}

	const filtered = groups.filter(g =>
		!searchQuery || g.name?.toLowerCase().includes(searchQuery.toLowerCase())
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
						<${SearchInput} value=${searchQuery} onChange=${setSearchQuery} placeholder="Search groups..." />
					</div>
					<div class="table-actions">
						<button class="btn btn-primary" onClick=${() => { setEditing({ ...emptyGroup }); setShowModal(true); }} type="button">
							<${Plus} size=${16} /> Add Group
						</button>
					</div>
				</div>

				<div class="table-container">
					<table class="data-table">
						<thead><tr><th>Name</th><th>Type</th><th>Owner</th><th>Created</th><th style=${{ width: '80px' }}>Actions</th></tr></thead>
						<tbody>
							${filtered.map(g => html`
								<tr key=${g.id}>
									<td><span class="cell-name">${g.name}</span></td>
									<td class="text-muted">${g.groupTemplate?.displayName || g.groupTemplateId || '-'}</td>
									<td class="text-muted">${g.userId || '-'}</td>
									<td class="text-muted">${g.createdAt ? new Date(g.createdAt).toLocaleDateString() : '-'}</td>
									<td><div class="action-buttons">
										<button class="btn-icon-sm" title="Edit" onClick=${() => { setEditing({ ...g }); setShowModal(true); }} type="button"><${Edit2} size=${14} /></button>
										<button class="btn-icon-sm" title="Delete" onClick=${() => { setToDelete(g); setShowDeleteModal(true); }} type="button"><${Trash2} size=${14} /></button>
									</div></td>
								</tr>
							`)}
							${filtered.length === 0 ? html`<tr><td class="empty-row" colspan="5">${loading ? 'Loading...' : 'No groups found'}</td></tr>` : null}
						</tbody>
					</table>
				</div>
			</div>

			${showModal && editing ? html`
				<${Modal} title=${editing.id ? 'Edit Group' : 'Add Group'} onClose=${() => { setShowModal(false); setEditing(null); }}>
					<div class="form-group">
						<label class="form-label">Name *</label>
						<input class="form-input" value=${editing.name} onInput=${(e: Event) => setEditing({ ...editing, name: (e.target as HTMLInputElement).value })} placeholder="Group name" />
					</div>
					<div class="form-group">
						<label class="form-label">Description</label>
						<textarea class="form-input" rows="2" value=${editing.description} onInput=${(e: Event) => setEditing({ ...editing, description: (e.target as HTMLTextAreaElement).value })} placeholder="Group description"></textarea>
					</div>
					<div class="form-group">
						<label class="form-label">Group Type</label>
						<select class="form-select" value=${editing.groupTemplateId} onChange=${(e: Event) => setEditing({ ...editing, groupTemplateId: parseInt((e.target as HTMLSelectElement).value) || 0 })}>
							<option value="0">Select type...</option>
							${groupTypes.map((t: any) => html`<option value=${t.id}>${t.displayName || t.name}</option>`)}
						</select>
					</div>
					<div class="form-actions">
						<button class="btn btn-secondary" onClick=${() => { setShowModal(false); setEditing(null); }} type="button">Cancel</button>
						<button class="btn btn-primary" onClick=${saveGroup} type="button">${editing.id ? 'Save Changes' : 'Create Group'}</button>
					</div>
				</${Modal}>
			` : null}

			${showDeleteModal && toDelete ? html`
				<${Modal} title="Delete Group" onClose=${() => { setShowDeleteModal(false); setToDelete(null); }}>
					<p>Are you sure you want to delete <strong>${toDelete.name}</strong>?</p>
					<p class="text-danger">This action cannot be undone.</p>
					<div class="form-actions">
						<button class="btn btn-secondary" onClick=${() => { setShowDeleteModal(false); setToDelete(null); }} type="button">Cancel</button>
						<button class="btn btn-danger" onClick=${deleteGroup} type="button">Delete</button>
					</div>
				</${Modal}>
			` : null}
		<//>
	`;
}
