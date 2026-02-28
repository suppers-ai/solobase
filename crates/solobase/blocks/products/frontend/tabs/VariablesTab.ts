import { html } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { Plus, Edit2, Trash2, Lock, CheckCircle, AlertCircle, X } from 'lucide-preact';
import { authFetch, Modal, SearchInput } from '@solobase/ui';

export function VariablesTab() {
	const [variables, setVariables] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [searchQuery, setSearchQuery] = useState('');
	const [showModal, setShowModal] = useState(false);
	const [showDeleteModal, setShowDeleteModal] = useState(false);
	const [editing, setEditing] = useState<any>(null);
	const [toDelete, setToDelete] = useState<any>(null);
	const [notification, setNotification] = useState<{ message: string; type: string } | null>(null);

	const emptyVar = { name: '', displayName: '', description: '', type: 'user', valueType: 'number', defaultValue: 0, status: 'active' };

	useEffect(() => { loadData(); }, []);

	function showNotif(message: string, type = 'info') {
		setNotification({ message, type });
		setTimeout(() => setNotification(null), 3000);
	}

	async function loadData() {
		setLoading(true);
		try {
			const response = await authFetch('/api/admin/ext/products/variables');
			if (response.ok) setVariables(await response.json() || []);
		} catch { /* ignore */ }
		setLoading(false);
	}

	async function saveVariable() {
		if (!editing) return;
		try {
			const method = editing.id ? 'PUT' : 'POST';
			const url = editing.id
				? `/api/admin/ext/products/variables/${editing.id}`
				: '/api/admin/ext/products/variables';
			const response = await authFetch(url, {
				method,
				body: JSON.stringify(editing),
			});
			if (response.ok) {
				showNotif(`Variable ${editing.id ? 'updated' : 'created'}`, 'success');
				setShowModal(false);
				setEditing(null);
				loadData();
			} else { showNotif('Failed to save variable', 'error'); }
		} catch { showNotif('Failed to save variable', 'error'); }
	}

	async function deleteVariable() {
		if (!toDelete) return;
		try {
			const response = await authFetch(`/api/admin/ext/products/variables/${toDelete.id}`, { method: 'DELETE' });
			if (response.ok) {
				showNotif('Variable deleted', 'success');
				setShowDeleteModal(false);
				setToDelete(null);
				loadData();
			} else { showNotif('Failed to delete variable', 'error'); }
		} catch { showNotif('Failed to delete variable', 'error'); }
	}

	const filtered = variables.filter(v =>
		!searchQuery || v.name?.toLowerCase().includes(searchQuery.toLowerCase()) ||
		v.displayName?.toLowerCase().includes(searchQuery.toLowerCase())
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
						<${SearchInput} value=${searchQuery} onChange=${setSearchQuery} placeholder="Search variables..." />
					</div>
					<div class="table-actions">
						<button class="btn btn-primary" onClick=${() => { setEditing({ ...emptyVar }); setShowModal(true); }} type="button">
							<${Plus} size=${16} /> Add Variable
						</button>
					</div>
				</div>

				<div class="table-container">
					<table class="data-table">
						<thead><tr><th>Name</th><th>Display Name</th><th>Type</th><th>Value Type</th><th>Default</th><th>Status</th><th style=${{ width: '80px' }}>Actions</th></tr></thead>
						<tbody>
							${filtered.map(v => html`
								<tr key=${v.id}>
									<td><span class="cell-mono">${v.name}</span></td>
									<td><span class="cell-name">${v.displayName || '-'}</span></td>
									<td>
										<span class="status-badge ${v.type === 'system' ? 'status-pending' : 'status-active'}">
											${v.type === 'system' ? html`<${Lock} size=${12} /> system` : 'user'}
										</span>
									</td>
									<td class="text-muted">${v.valueType || '-'}</td>
									<td class="cell-mono">${v.defaultValue != null ? String(v.defaultValue) : '-'}</td>
									<td>
										<span class="status-badge ${v.status === 'active' ? 'status-active' : 'status-inactive'}">
											${v.status || 'active'}
										</span>
									</td>
									<td><div class="action-buttons">
										${v.type !== 'system' ? html`
											<button class="btn-icon-sm" title="Edit" onClick=${() => { setEditing({ ...v }); setShowModal(true); }} type="button"><${Edit2} size=${14} /></button>
											<button class="btn-icon-sm" title="Delete" onClick=${() => { setToDelete(v); setShowDeleteModal(true); }} type="button"><${Trash2} size=${14} /></button>
										` : null}
									</div></td>
								</tr>
							`)}
							${filtered.length === 0 ? html`<tr><td class="empty-row" colspan="7">${loading ? 'Loading...' : 'No variables found'}</td></tr>` : null}
						</tbody>
					</table>
				</div>
			</div>

			${showModal && editing ? html`
				<${Modal} title=${editing.id ? 'Edit Variable' : 'Add Variable'} onClose=${() => { setShowModal(false); setEditing(null); }}>
					<div class="form-group">
						<label class="form-label">Name *</label>
						<input class="form-input" value=${editing.name} onInput=${(e: Event) => setEditing({ ...editing, name: (e.target as HTMLInputElement).value })} placeholder="variable_name" style=${{ fontFamily: "'SF Mono', 'Fira Code', monospace" }} />
					</div>
					<div class="form-group">
						<label class="form-label">Display Name</label>
						<input class="form-input" value=${editing.displayName} onInput=${(e: Event) => setEditing({ ...editing, displayName: (e.target as HTMLInputElement).value })} placeholder="Display Name" />
					</div>
					<div class="form-group">
						<label class="form-label">Description</label>
						<textarea class="form-input" rows="2" value=${editing.description} onInput=${(e: Event) => setEditing({ ...editing, description: (e.target as HTMLTextAreaElement).value })} placeholder="Variable description"></textarea>
					</div>
					<div class="form-group">
						<label class="form-label">Value Type</label>
						<select class="form-select" value=${editing.valueType} onChange=${(e: Event) => setEditing({ ...editing, valueType: (e.target as HTMLSelectElement).value })}>
							<option value="number">Number</option>
							<option value="string">String</option>
							<option value="boolean">Boolean</option>
						</select>
					</div>
					<div class="form-group">
						<label class="form-label">Default Value</label>
						<input class="form-input" value=${editing.defaultValue != null ? String(editing.defaultValue) : ''} onInput=${(e: Event) => {
							const val = (e.target as HTMLInputElement).value;
							const num = parseFloat(val);
							setEditing({ ...editing, defaultValue: isNaN(num) ? val : num });
						}} placeholder="Default value" />
					</div>
					<div class="form-group">
						<label class="form-label">Status</label>
						<select class="form-select" value=${editing.status} onChange=${(e: Event) => setEditing({ ...editing, status: (e.target as HTMLSelectElement).value })}>
							<option value="active">Active</option>
							<option value="pending">Pending</option>
						</select>
					</div>
					<div class="form-actions">
						<button class="btn btn-secondary" onClick=${() => { setShowModal(false); setEditing(null); }} type="button">Cancel</button>
						<button class="btn btn-primary" onClick=${saveVariable} type="button">${editing.id ? 'Save Changes' : 'Create Variable'}</button>
					</div>
				</${Modal}>
			` : null}

			${showDeleteModal && toDelete ? html`
				<${Modal} title="Delete Variable" onClose=${() => { setShowDeleteModal(false); setToDelete(null); }}>
					<p>Are you sure you want to delete <strong>${toDelete.displayName || toDelete.name}</strong>?</p>
					<p class="text-danger">This action cannot be undone.</p>
					<div class="form-actions">
						<button class="btn btn-secondary" onClick=${() => { setShowDeleteModal(false); setToDelete(null); }} type="button">Cancel</button>
						<button class="btn btn-danger" onClick=${deleteVariable} type="button">Delete</button>
					</div>
				</${Modal}>
			` : null}
		<//>
	`;
}
