import { html, BlockShell, PageHeader, TabNavigation, DataTable, LoadingSpinner, api } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { Shield, Key, Plus, Trash2, Copy, AlertTriangle } from 'lucide-preact';

function RolesTab() {
	const [roles, setRoles] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [showCreate, setShowCreate] = useState(false);
	const [newName, setNewName] = useState('');
	const [newDesc, setNewDesc] = useState('');

	const fetchRoles = () => {
		setLoading(true);
		api.get('/admin/iam/roles').then((data: any) => {
			const records = Array.isArray(data?.records) ? data.records : [];
			setRoles(records.map((r: any) => ({ id: r.id, ...r.data })));
			setLoading(false);
		}).catch(() => setLoading(false));
	};

	useEffect(fetchRoles, []);

	const createRole = async () => {
		if (!newName.trim()) return;
		await api.post('/admin/iam/roles', { name: newName, description: newDesc });
		setShowCreate(false);
		setNewName('');
		setNewDesc('');
		fetchRoles();
	};

	const deleteRole = async (role: any) => {
		if (role.is_system === true || role.is_system === '1' || role.is_system === 'true') {
			alert('Cannot delete system role');
			return;
		}
		if (!confirm(`Delete role "${role.name}"?`)) return;
		await api.delete(`/admin/iam/roles/${role.id}`);
		fetchRoles();
	};

	const columns = [
		{ key: 'name', label: 'Role', sortable: true, render: (v: string, row: any) => html`
			<div style=${{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
				<span style=${{ fontWeight: 600 }}>${v}</span>
				${(row.is_system === true || row.is_system === '1' || row.is_system === 'true') ? html`
					<span style=${{ fontSize: '0.625rem', padding: '0.0625rem 0.375rem', borderRadius: '4px', background: '#dbeafe', color: '#1d4ed8' }}>system</span>
				` : null}
			</div>
		` },
		{ key: 'description', label: 'Description' },
		{ key: '_actions', label: '', render: (_: any, row: any) => {
			const isSystem = row.is_system === true || row.is_system === '1' || row.is_system === 'true';
			return isSystem ? null : html`
				<button onClick=${() => deleteRole(row)}
					style=${{ padding: '0.25rem', background: 'none', border: 'none', cursor: 'pointer', color: '#dc2626', opacity: 0.5, display: 'flex' }}>
					<${Trash2} size=${14} />
				</button>
			`;
		}},
	];

	if (loading) return html`<${LoadingSpinner} message="Loading roles..." />`;

	return html`
		<div>
			<${PageHeader} title="Roles" description="Manage access control roles">
				<button onClick=${() => setShowCreate(!showCreate)}
					style=${{ padding: '0.5rem 1rem', background: '#fe6627', color: 'white', border: 'none', borderRadius: '8px', fontSize: '0.813rem', fontWeight: 600, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '0.375rem' }}>
					<${Plus} size=${14} /> Add Role
				</button>
			<//>
			${showCreate ? html`
				<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '8px', padding: '1rem', marginBottom: '1rem', display: 'flex', gap: '0.5rem', alignItems: 'flex-end' }}>
					<div style=${{ flex: 1 }}>
						<label style=${{ fontSize: '0.688rem', fontWeight: 600, color: '#94a3b8', display: 'block', marginBottom: '0.25rem' }}>NAME</label>
						<input value=${newName} onInput=${(e: any) => setNewName(e.target.value)} placeholder="editor"
							style=${{ width: '100%', padding: '0.5rem', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.813rem' }} />
					</div>
					<div style=${{ flex: 2 }}>
						<label style=${{ fontSize: '0.688rem', fontWeight: 600, color: '#94a3b8', display: 'block', marginBottom: '0.25rem' }}>DESCRIPTION</label>
						<input value=${newDesc} onInput=${(e: any) => setNewDesc(e.target.value)} placeholder="Can edit content"
							style=${{ width: '100%', padding: '0.5rem', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.813rem' }} />
					</div>
					<button onClick=${createRole} disabled=${!newName.trim()}
						style=${{ padding: '0.5rem 1rem', background: '#fe6627', color: 'white', border: 'none', borderRadius: '6px', fontSize: '0.813rem', fontWeight: 600, cursor: 'pointer', whiteSpace: 'nowrap' }}>
						Create
					</button>
					<button onClick=${() => setShowCreate(false)}
						style=${{ padding: '0.5rem 0.75rem', background: '#f1f5f9', color: '#64748b', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.813rem', cursor: 'pointer' }}>
						Cancel
					</button>
				</div>
			` : null}
			<${DataTable} columns=${columns} data=${roles} emptyMessage="No roles defined" />
		</div>
	`;
}

function ApiKeysTab() {
	const [keys, setKeys] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [showCreate, setShowCreate] = useState(false);
	const [newName, setNewName] = useState('');
	const [createdKey, setCreatedKey] = useState<string | null>(null);
	const [copied, setCopied] = useState(false);

	const fetchKeys = () => {
		setLoading(true);
		api.get('/auth/api-keys').then((data: any) => {
			const records = Array.isArray(data?.records) ? data.records : [];
			setKeys(records.map((r: any) => ({ id: r.id, ...r.data })));
			setLoading(false);
		}).catch(() => setLoading(false));
	};

	useEffect(fetchKeys, []);

	const createKey = async () => {
		if (!newName.trim()) return;
		try {
			const data: any = await api.post('/auth/api-keys', { name: newName });
			if (data?.key) {
				setCreatedKey(data.key);
				setShowCreate(false);
				setNewName('');
				fetchKeys();
			}
		} catch {}
	};

	const revokeKey = async (key: any) => {
		if (!confirm(`Revoke API key "${key.name}"? It will no longer work for authentication.`)) return;
		await api.patch(`/auth/api-keys/${key.id}`, {});
		fetchKeys();
	};

	const deleteKey = async (key: any) => {
		if (!confirm(`Permanently delete API key "${key.name}"?`)) return;
		await api.delete(`/auth/api-keys/${key.id}`);
		fetchKeys();
	};

	const copyKey = () => {
		if (createdKey) {
			navigator.clipboard.writeText(createdKey);
			setCopied(true);
			setTimeout(() => setCopied(false), 2000);
		}
	};

	const columns = [
		{ key: 'name', label: 'Name', sortable: true },
		{ key: 'key_prefix', label: 'Key', render: (v: string) => html`<code style=${{ fontSize: '0.75rem', color: '#64748b' }}>${v}...</code>` },
		{ key: 'created_at', label: 'Created', sortable: true, render: (v: string) => v ? new Date(v).toLocaleDateString() : '-' },
		{ key: 'revoked_at', label: 'Status', render: (v: string) => html`<span style=${{
			fontSize: '0.688rem', padding: '0.125rem 0.5rem', borderRadius: '9999px',
			background: v ? '#fef2f2' : '#dcfce7', color: v ? '#dc2626' : '#166534'
		}}>${v ? 'Revoked' : 'Active'}</span>` },
		{ key: '_actions', label: '', render: (_: any, row: any) => html`
			<div style=${{ display: 'flex', gap: '0.375rem' }}>
				${!row.revoked_at ? html`
					<button onClick=${() => revokeKey(row)}
						style=${{ padding: '0.25rem 0.5rem', fontSize: '0.688rem', border: '1px solid #fde68a', borderRadius: '4px', background: 'white', cursor: 'pointer', color: '#92400e' }}>
						Revoke
					</button>
				` : null}
				<button onClick=${() => deleteKey(row)}
					style=${{ padding: '0.25rem', background: 'none', border: 'none', cursor: 'pointer', color: '#dc2626', opacity: 0.5, display: 'flex' }}>
					<${Trash2} size=${14} />
				</button>
			</div>
		` },
	];

	if (loading) return html`<${LoadingSpinner} message="Loading API keys..." />`;

	return html`
		<div>
			<${PageHeader} title="API Keys" description="Create and manage API keys for programmatic access">
				<button onClick=${() => { setShowCreate(!showCreate); setCreatedKey(null); }}
					style=${{ padding: '0.5rem 1rem', background: '#fe6627', color: 'white', border: 'none', borderRadius: '8px', fontSize: '0.813rem', fontWeight: 600, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '0.375rem' }}>
					<${Plus} size=${14} /> Create Key
				</button>
			<//>
			${createdKey ? html`
				<div style=${{ background: '#fffbeb', border: '1px solid #fde68a', borderRadius: '8px', padding: '1rem', marginBottom: '1rem' }}>
					<div style=${{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '0.5rem', fontSize: '0.813rem', fontWeight: 600, color: '#92400e' }}>
						<${AlertTriangle} size=${16} /> Save this key now — it won't be shown again
					</div>
					<div style=${{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
						<code style=${{ flex: 1, padding: '0.5rem 0.75rem', background: 'white', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.813rem', fontFamily: 'monospace', wordBreak: 'break-all' }}>${createdKey}</code>
						<button onClick=${copyKey}
							style=${{ padding: '0.5rem 0.75rem', background: 'white', border: '1px solid #e2e8f0', borderRadius: '6px', cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '0.25rem', fontSize: '0.75rem', color: copied ? '#16a34a' : '#64748b', whiteSpace: 'nowrap' }}>
							<${Copy} size=${14} /> ${copied ? 'Copied!' : 'Copy'}
						</button>
					</div>
				</div>
			` : null}
			${showCreate ? html`
				<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '8px', padding: '1rem', marginBottom: '1rem', display: 'flex', gap: '0.5rem', alignItems: 'flex-end' }}>
					<div style=${{ flex: 1 }}>
						<label style=${{ fontSize: '0.688rem', fontWeight: 600, color: '#94a3b8', display: 'block', marginBottom: '0.25rem' }}>KEY NAME</label>
						<input value=${newName} onInput=${(e: any) => setNewName(e.target.value)} placeholder="my-app-key"
							onKeyDown=${(e: any) => { if (e.key === 'Enter') createKey(); }}
							style=${{ width: '100%', padding: '0.5rem', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.813rem' }} />
					</div>
					<button onClick=${createKey} disabled=${!newName.trim()}
						style=${{ padding: '0.5rem 1rem', background: '#fe6627', color: 'white', border: 'none', borderRadius: '6px', fontSize: '0.813rem', fontWeight: 600, cursor: 'pointer', whiteSpace: 'nowrap' }}>
						Create
					</button>
					<button onClick=${() => setShowCreate(false)}
						style=${{ padding: '0.5rem 0.75rem', background: '#f1f5f9', color: '#64748b', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.813rem', cursor: 'pointer' }}>
						Cancel
					</button>
				</div>
			` : null}
			<${DataTable} columns=${columns} data=${keys} emptyMessage="No API keys created yet" />
		</div>
	`;
}

export function App() {
	const [tab, setTab] = useState('roles');

	const tabs = [
		{ id: 'roles', label: 'Roles', icon: Shield },
		{ id: 'api-keys', label: 'API Keys', icon: Key },
	];

	return html`
		<${BlockShell} title="IAM">
			<${TabNavigation} tabs=${tabs} activeTab=${tab} onTabChange=${setTab} />
			${tab === 'roles' ? html`<${RolesTab} />` : null}
			${tab === 'api-keys' ? html`<${ApiKeysTab} />` : null}
		<//>
	`;
}
