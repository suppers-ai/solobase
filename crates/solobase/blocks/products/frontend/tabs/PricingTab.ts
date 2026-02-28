import { html } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { Plus, Edit2, Trash2, Play, CheckCircle, AlertCircle, X } from 'lucide-preact';
import { authFetch, Modal, SearchInput } from '@solobase/ui';

export function PricingTab() {
	const [templates, setTemplates] = useState<any[]>([]);
	const [variables, setVariables] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [searchQuery, setSearchQuery] = useState('');
	const [showModal, setShowModal] = useState(false);
	const [showDeleteModal, setShowDeleteModal] = useState(false);
	const [editing, setEditing] = useState<any>(null);
	const [toDelete, setToDelete] = useState<any>(null);
	const [notification, setNotification] = useState<{ message: string; type: string } | null>(null);

	// Formula sandbox state
	const [formula, setFormula] = useState('');
	const [varValues, setVarValues] = useState<Record<string, string>>({});
	const [testResult, setTestResult] = useState<any>(null);
	const [testing, setTesting] = useState(false);

	const emptyTemplate = { name: '', displayName: '', description: '', priceFormula: '', conditionFormula: '', category: '', status: 'active' };

	useEffect(() => { loadData(); }, []);

	function showNotif(message: string, type = 'info') {
		setNotification({ message, type });
		setTimeout(() => setNotification(null), 3000);
	}

	async function loadData() {
		setLoading(true);
		try {
			const [templatesRes, varsRes] = await Promise.all([
				authFetch('/api/admin/ext/products/pricing-templates'),
				authFetch('/api/admin/ext/products/variables'),
			]);
			if (templatesRes.ok) setTemplates(await templatesRes.json() || []);
			if (varsRes.ok) {
				const v = await varsRes.json() || [];
				setVariables(v);
				// Init var values with defaults
				const defaults: Record<string, string> = {};
				v.forEach((variable: any) => {
					defaults[variable.name] = String(variable.defaultValue ?? '');
				});
				setVarValues(prev => ({ ...defaults, ...prev }));
			}
		} catch { /* ignore */ }
		setLoading(false);
	}

	async function saveTemplate() {
		if (!editing) return;
		try {
			const method = editing.id ? 'PUT' : 'POST';
			const url = editing.id
				? `/api/admin/ext/products/pricing-templates/${editing.id}`
				: '/api/admin/ext/products/pricing-templates';
			const response = await authFetch(url, {
				method,
				body: JSON.stringify(editing),
			});
			if (response.ok) {
				showNotif(`Template ${editing.id ? 'updated' : 'created'}`, 'success');
				setShowModal(false);
				setEditing(null);
				loadData();
			} else { showNotif('Failed to save template', 'error'); }
		} catch { showNotif('Failed to save template', 'error'); }
	}

	async function deleteTemplate() {
		if (!toDelete) return;
		try {
			const response = await authFetch(`/api/admin/ext/products/pricing-templates/${toDelete.id}`, { method: 'DELETE' });
			if (response.ok) {
				showNotif('Template deleted', 'success');
				setShowDeleteModal(false);
				setToDelete(null);
				loadData();
			} else { showNotif('Failed to delete template', 'error'); }
		} catch { showNotif('Failed to delete template', 'error'); }
	}

	async function testFormula() {
		if (!formula.trim()) return;
		setTesting(true);
		setTestResult(null);
		try {
			// Build variables map with numeric parsing
			const vars: Record<string, any> = {};
			for (const [key, val] of Object.entries(varValues)) {
				const num = parseFloat(val);
				vars[key] = isNaN(num) ? val : num;
			}
			const response = await authFetch('/api/admin/ext/products/test-formula', {
				method: 'POST',
				body: JSON.stringify({ formula, variables: vars }),
			});
			if (response.ok) {
				setTestResult(await response.json());
			} else {
				setTestResult({ success: false, error: 'Request failed' });
			}
		} catch {
			setTestResult({ success: false, error: 'Network error' });
		}
		setTesting(false);
	}

	const filtered = templates.filter(t =>
		!searchQuery || t.name?.toLowerCase().includes(searchQuery.toLowerCase()) ||
		t.displayName?.toLowerCase().includes(searchQuery.toLowerCase())
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
						<${SearchInput} value=${searchQuery} onChange=${setSearchQuery} placeholder="Search pricing templates..." />
					</div>
					<div class="table-actions">
						<button class="btn btn-primary" onClick=${() => { setEditing({ ...emptyTemplate }); setShowModal(true); }} type="button">
							<${Plus} size=${16} /> Add Template
						</button>
					</div>
				</div>

				<div class="table-container">
					<table class="data-table">
						<thead><tr><th>Name</th><th>Formula</th><th>Condition</th><th>Category</th><th>Status</th><th style=${{ width: '80px' }}>Actions</th></tr></thead>
						<tbody>
							${filtered.map(t => html`
								<tr key=${t.id}>
									<td><span class="cell-name">${t.displayName || t.name}</span></td>
									<td><span class="cell-mono">${t.priceFormula || '-'}</span></td>
									<td><span class="cell-mono">${t.conditionFormula || 'always'}</span></td>
									<td class="text-muted">${t.category || '-'}</td>
									<td>
										<span class="status-badge ${t.status === 'active' ? 'status-active' : 'status-inactive'}">
											${t.status || 'active'}
										</span>
									</td>
									<td><div class="action-buttons">
										<button class="btn-icon-sm" title="Edit" onClick=${() => { setEditing({ ...t }); setShowModal(true); }} type="button"><${Edit2} size=${14} /></button>
										<button class="btn-icon-sm" title="Delete" onClick=${() => { setToDelete(t); setShowDeleteModal(true); }} type="button"><${Trash2} size=${14} /></button>
									</div></td>
								</tr>
							`)}
							${filtered.length === 0 ? html`<tr><td class="empty-row" colspan="6">${loading ? 'Loading...' : 'No pricing templates found'}</td></tr>` : null}
						</tbody>
					</table>
				</div>
			</div>

			<div class="sandbox-section">
				<h3 class="sandbox-title"><${Play} size=${18} /> Formula Test Sandbox</h3>
				<div class="form-group">
					<label class="form-label">Formula</label>
					<textarea class="formula-input" value=${formula} onInput=${(e: Event) => setFormula((e.target as HTMLTextAreaElement).value)} placeholder="e.g. base_price * quantity * (1 - discount / 100)"></textarea>
				</div>

				${variables.length > 0 ? html`
					<div class="vars-grid">
						${variables.map(v => html`
							<div class="var-field" key=${v.name}>
								<label>${v.displayName || v.name}</label>
								<input
									type=${v.valueType === 'number' ? 'number' : 'text'}
									value=${varValues[v.name] ?? ''}
									onInput=${(e: Event) => setVarValues({ ...varValues, [v.name]: (e.target as HTMLInputElement).value })}
									placeholder=${v.defaultValue != null ? String(v.defaultValue) : ''}
								/>
							</div>
						`)}
					</div>
				` : null}

				<button class="btn btn-primary" onClick=${testFormula} disabled=${testing || !formula.trim()} type="button">
					<${Play} size=${16} /> ${testing ? 'Testing...' : 'Test Formula'}
				</button>

				${testResult ? html`
					<div class="result-box ${testResult.success ? 'result-success' : 'result-error'}">
						${testResult.success ? html`
							<div class="result-value">Result: ${testResult.result}</div>
							<div class="text-muted" style=${{ marginTop: '0.5rem' }}>Formula: ${testResult.formula}</div>
						` : html`
							<div>Error: ${testResult.error}</div>
						`}
					</div>
				` : null}
			</div>

			${showModal && editing ? html`
				<${Modal} title=${editing.id ? 'Edit Pricing Template' : 'Add Pricing Template'} onClose=${() => { setShowModal(false); setEditing(null); }}>
					<div class="form-group">
						<label class="form-label">Name *</label>
						<input class="form-input" value=${editing.name} onInput=${(e: Event) => setEditing({ ...editing, name: (e.target as HTMLInputElement).value })} placeholder="template_name" />
					</div>
					<div class="form-group">
						<label class="form-label">Display Name</label>
						<input class="form-input" value=${editing.displayName} onInput=${(e: Event) => setEditing({ ...editing, displayName: (e.target as HTMLInputElement).value })} placeholder="Display Name" />
					</div>
					<div class="form-group">
						<label class="form-label">Description</label>
						<textarea class="form-input" rows="2" value=${editing.description} onInput=${(e: Event) => setEditing({ ...editing, description: (e.target as HTMLTextAreaElement).value })} placeholder="Template description"></textarea>
					</div>
					<div class="form-group">
						<label class="form-label">Price Formula *</label>
						<input class="form-input" value=${editing.priceFormula} onInput=${(e: Event) => setEditing({ ...editing, priceFormula: (e.target as HTMLInputElement).value })} placeholder="e.g. base_price * quantity" style=${{ fontFamily: "'SF Mono', 'Fira Code', monospace" }} />
					</div>
					<div class="form-group">
						<label class="form-label">Condition Formula</label>
						<input class="form-input" value=${editing.conditionFormula} onInput=${(e: Event) => setEditing({ ...editing, conditionFormula: (e.target as HTMLInputElement).value })} placeholder="e.g. quantity > 10" style=${{ fontFamily: "'SF Mono', 'Fira Code', monospace" }} />
					</div>
					<div class="form-group">
						<label class="form-label">Category</label>
						<input class="form-input" value=${editing.category} onInput=${(e: Event) => setEditing({ ...editing, category: (e.target as HTMLInputElement).value })} placeholder="e.g. bulk, seasonal" />
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
						<button class="btn btn-primary" onClick=${saveTemplate} type="button">${editing.id ? 'Save Changes' : 'Create Template'}</button>
					</div>
				</${Modal}>
			` : null}

			${showDeleteModal && toDelete ? html`
				<${Modal} title="Delete Pricing Template" onClose=${() => { setShowDeleteModal(false); setToDelete(null); }}>
					<p>Are you sure you want to delete <strong>${toDelete.displayName || toDelete.name}</strong>?</p>
					<p class="text-danger">This action cannot be undone.</p>
					<div class="form-actions">
						<button class="btn btn-secondary" onClick=${() => { setShowDeleteModal(false); setToDelete(null); }} type="button">Cancel</button>
						<button class="btn btn-danger" onClick=${deleteTemplate} type="button">Delete</button>
					</div>
				</${Modal}>
			` : null}
		<//>
	`;
}
