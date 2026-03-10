import { html, PageHeader, DataTable, SearchInput, Button, Modal, ConfirmDialog, LoadingSpinner, api, toasts } from '@solobase/ui';
import { useState, useEffect, useCallback } from 'preact/hooks';
import { Plus, Edit2, Trash2, Play } from 'lucide-preact';

const inputStyle = { width: '100%', padding: '0.5rem 0.75rem', border: '1px solid #e2e8f0', borderRadius: '8px', fontSize: '0.813rem', outline: 'none', boxSizing: 'border-box' as const };
const monoInput = { ...inputStyle, fontFamily: "'SF Mono', 'Fira Code', monospace" };
const labelStyle = { display: 'block', fontSize: '0.813rem', fontWeight: 500, color: '#1e293b', marginBottom: '0.375rem' };
const fieldStyle = { marginBottom: '1rem' };

const emptyTemplate = { name: '', display_name: '', description: '', price_formula: '', condition_formula: '', category: '', status: 'active' };

export function PricingTab() {
	const [templates, setTemplates] = useState<any[]>([]);
	const [variables, setVariables] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [search, setSearch] = useState('');
	const [showModal, setShowModal] = useState(false);
	const [editing, setEditing] = useState<any>(null);
	const [showDelete, setShowDelete] = useState(false);
	const [toDelete, setToDelete] = useState<any>(null);
	const [saving, setSaving] = useState(false);

	// Formula sandbox
	const [formula, setFormula] = useState('');
	const [varValues, setVarValues] = useState<Record<string, string>>({});
	const [testResult, setTestResult] = useState<any>(null);
	const [testing, setTesting] = useState(false);

	const load = useCallback(async () => {
		setLoading(true);
		try {
			const [tmplData, varData] = await Promise.all([
				api.get('/admin/b/products/pricing').catch(() => ({})),
				api.get('/admin/b/products/variables').catch(() => ({})),
			]);
			setTemplates(Array.isArray(tmplData?.records) ? tmplData.records : Array.isArray(tmplData) ? tmplData : []);
			const vars = Array.isArray(varData?.records) ? varData.records : Array.isArray(varData) ? varData : [];
			setVariables(vars);
			const defaults: Record<string, string> = {};
			vars.forEach((v: any) => { defaults[v.name] = String(v.default_value ?? ''); });
			setVarValues(prev => ({ ...defaults, ...prev }));
		} catch { /* ignore */ }
		setLoading(false);
	}, []);

	useEffect(() => { load(); }, [load]);

	async function save() {
		if (!editing?.name?.trim()) { toasts.error('Name is required'); return; }
		setSaving(true);
		try {
			if (editing.id) {
				await api.put(`/admin/b/products/pricing/${editing.id}`, editing);
				toasts.success('Template updated');
			} else {
				await api.post('/admin/b/products/pricing', editing);
				toasts.success('Template created');
			}
			setShowModal(false);
			setEditing(null);
			await load();
		} catch (err: any) {
			toasts.error(err.message || 'Failed to save template');
		}
		setSaving(false);
	}

	async function handleDelete() {
		if (!toDelete) return;
		try {
			await api.delete(`/admin/b/products/pricing/${toDelete.id}`);
			toasts.success('Template deleted');
			setShowDelete(false);
			setToDelete(null);
			await load();
		} catch (err: any) {
			toasts.error(err.message || 'Failed to delete template');
		}
	}

	async function testFormula_() {
		if (!formula.trim()) return;
		setTesting(true);
		setTestResult(null);
		try {
			const vars: Record<string, any> = {};
			for (const [key, val] of Object.entries(varValues)) {
				const num = parseFloat(val);
				vars[key] = isNaN(num) ? val : num;
			}
			// Use calculate-price endpoint as a proxy for formula testing
			const result = await api.post('/b/products/calculate-price', { formula, variables: vars, quantity: 1 });
			setTestResult({ success: true, ...result });
		} catch (err: any) {
			setTestResult({ success: false, error: err.message || 'Evaluation failed' });
		}
		setTesting(false);
	}

	const filtered = search ? templates.filter(t =>
		t.name?.toLowerCase().includes(search.toLowerCase()) ||
		t.display_name?.toLowerCase().includes(search.toLowerCase())
	) : templates;

	const columns = [
		{ key: 'name', label: 'Template', sortable: true, render: (v: string, row: any) => row.display_name || v },
		{ key: 'price_formula', label: 'Formula', render: (v: string) => v ? html`<code style=${{ fontSize: '0.75rem', background: '#f1f5f9', padding: '0.125rem 0.375rem', borderRadius: '4px' }}>${v}</code>` : '-' },
		{ key: 'category', label: 'Category', render: (v: string) => v || '-' },
		{ key: 'status', label: 'Status', render: (v: string) => html`
			<span style=${{ fontSize: '0.75rem', padding: '0.125rem 0.5rem', borderRadius: '9999px', background: v === 'active' ? '#dcfce7' : '#f3f4f6', color: v === 'active' ? '#166534' : '#6b7280' }}>${v || 'active'}</span>
		` },
		{ key: 'created_at', label: 'Created', sortable: true, render: (v: string) => v ? new Date(v).toLocaleDateString() : '-' },
		{ key: '_actions', label: '', width: '80px', render: (_: any, row: any) => html`
			<div style=${{ display: 'flex', gap: '0.25rem' }}>
				<button onClick=${(e: Event) => { e.stopPropagation(); setEditing({ ...row }); setShowModal(true); }} style=${{ background: 'none', border: 'none', cursor: 'pointer', color: '#64748b', padding: '0.25rem' }} type="button"><${Edit2} size=${14} /></button>
				<button onClick=${(e: Event) => { e.stopPropagation(); setToDelete(row); setShowDelete(true); }} style=${{ background: 'none', border: 'none', cursor: 'pointer', color: '#dc2626', padding: '0.25rem' }} type="button"><${Trash2} size=${14} /></button>
			</div>
		` },
	];

	if (loading) return html`<${LoadingSpinner} message="Loading pricing templates..." />`;

	const footer = html`
		<${Button} variant="secondary" onClick=${() => { setShowModal(false); setEditing(null); }}>Cancel<//>
		<${Button} onClick=${save} loading=${saving}>${editing?.id ? 'Save Changes' : 'Create Template'}<//>
	`;

	return html`
		<div>
			<${PageHeader} title="Pricing Templates" description="Define pricing formulas and conditions">
				<${Button} icon=${Plus} onClick=${() => { setEditing({ ...emptyTemplate }); setShowModal(true); }}>Add Template<//>
			<//>
			<${SearchInput} value=${search} onChange=${setSearch} placeholder="Search pricing templates..." />
			<${DataTable} columns=${columns} data=${filtered} emptyMessage="No pricing templates defined" />

			<!-- Formula Sandbox -->
			<div style=${{ marginTop: '2rem', background: 'white', border: '1px solid #e2e8f0', borderRadius: '12px', padding: '1.5rem' }}>
				<h3 style=${{ fontSize: '1rem', fontWeight: 600, color: '#1e293b', marginBottom: '1rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
					<${Play} size=${18} /> Formula Test Sandbox
				</h3>
				<div style=${fieldStyle}>
					<label style=${labelStyle}>Formula</label>
					<textarea style=${{ ...monoInput, minHeight: '60px' }} value=${formula} onInput=${(e: any) => setFormula(e.target.value)} placeholder="e.g. base_price * quantity * (1 - discount / 100)"></textarea>
				</div>
				${variables.length > 0 ? html`
					<div style=${{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(150px, 1fr))', gap: '0.75rem', marginBottom: '1rem' }}>
						${variables.map((v: any) => html`
							<div key=${v.name}>
								<label style=${labelStyle}>${v.display_name || v.name}</label>
								<input style=${inputStyle} type=${v.value_type === 'number' ? 'number' : 'text'}
									value=${varValues[v.name] ?? ''} onInput=${(e: any) => setVarValues({ ...varValues, [v.name]: e.target.value })}
									placeholder=${v.default_value != null ? String(v.default_value) : ''} />
							</div>
						`)}
					</div>
				` : null}
				<${Button} icon=${Play} onClick=${testFormula_} loading=${testing} disabled=${!formula.trim()}>
					${testing ? 'Testing...' : 'Test Formula'}
				<//>
				${testResult ? html`
					<div style=${{ marginTop: '1rem', padding: '0.75rem', borderRadius: '8px', background: testResult.success ? '#f0fdf4' : '#fef2f2', border: testResult.success ? '1px solid #bbf7d0' : '1px solid #fecaca', fontSize: '0.875rem' }}>
						${testResult.success
							? html`<div style=${{ fontWeight: 600, color: '#166534' }}>Result: ${testResult.unit_price ?? testResult.total ?? JSON.stringify(testResult)}</div>`
							: html`<div style=${{ color: '#dc2626' }}>Error: ${testResult.error}</div>`
						}
					</div>
				` : null}
			</div>

			<${Modal} show=${showModal} title=${editing?.id ? 'Edit Pricing Template' : 'Add Pricing Template'} onClose=${() => { setShowModal(false); setEditing(null); }} footer=${footer}>
				${editing ? html`
					<div>
						<div style=${fieldStyle}>
							<label style=${labelStyle}>Name *</label>
							<input style=${inputStyle} value=${editing.name} onInput=${(e: any) => setEditing({ ...editing, name: e.target.value })} placeholder="template_name" />
						</div>
						<div style=${fieldStyle}>
							<label style=${labelStyle}>Display Name</label>
							<input style=${inputStyle} value=${editing.display_name} onInput=${(e: any) => setEditing({ ...editing, display_name: e.target.value })} placeholder="Display Name" />
						</div>
						<div style=${fieldStyle}>
							<label style=${labelStyle}>Description</label>
							<textarea style=${{ ...inputStyle, minHeight: '60px' }} value=${editing.description} onInput=${(e: any) => setEditing({ ...editing, description: e.target.value })} placeholder="Template description"></textarea>
						</div>
						<div style=${fieldStyle}>
							<label style=${labelStyle}>Price Formula *</label>
							<input style=${monoInput} value=${editing.price_formula} onInput=${(e: any) => setEditing({ ...editing, price_formula: e.target.value })} placeholder="e.g. base_price * quantity" />
						</div>
						<div style=${fieldStyle}>
							<label style=${labelStyle}>Condition Formula</label>
							<input style=${monoInput} value=${editing.condition_formula} onInput=${(e: any) => setEditing({ ...editing, condition_formula: e.target.value })} placeholder="e.g. quantity > 10" />
						</div>
						<div style=${{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem', ...fieldStyle }}>
							<div>
								<label style=${labelStyle}>Category</label>
								<input style=${inputStyle} value=${editing.category} onInput=${(e: any) => setEditing({ ...editing, category: e.target.value })} placeholder="e.g. bulk, seasonal" />
							</div>
							<div>
								<label style=${labelStyle}>Status</label>
								<select style=${inputStyle} value=${editing.status} onChange=${(e: any) => setEditing({ ...editing, status: e.target.value })}>
									<option value="active">Active</option>
									<option value="pending">Pending</option>
								</select>
							</div>
						</div>
					</div>
				` : null}
			<//>

			<${ConfirmDialog}
				show=${showDelete}
				title="Delete Pricing Template"
				message=${`Are you sure you want to delete "${toDelete?.display_name || toDelete?.name}"? This action cannot be undone.`}
				confirmText="Delete"
				onConfirm=${handleDelete}
				onCancel=${() => { setShowDelete(false); setToDelete(null); }}
			/>
		</div>
	`;
}
