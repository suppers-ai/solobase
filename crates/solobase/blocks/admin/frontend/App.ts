import { html, BlockShell, PageHeader, StatCard, SearchInput, DataTable, LoadingSpinner, TabNavigation, api, currentUser } from '@solobase/ui';
import { useState, useEffect, useRef } from 'preact/hooks';
import { LayoutDashboard, Users, ShoppingCart, DollarSign, HardDrive, Layers, ExternalLink, Play, Database, FolderOpen, Eye, EyeOff, Plus, Trash2, Shield, Key, Copy, AlertTriangle } from 'lucide-preact';

function DashboardTab() {
	const [stats, setStats] = useState<any>(null);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		Promise.all([
			api.get('/admin/users?page=1&pageSize=1').catch(() => ({ total: 0 })),
			api.getStorageBuckets().catch(() => ({ data: [] })),
			api.getExtensions().catch(() => []),
			api.get('/admin/b/products/stats').catch(() => ({})),
		]).then(([usersRes, storageRes, extRes, productStats]) => {
			const blocks = Array.isArray(extRes) ? extRes : (extRes as any)?.data || [];
			setStats({
				users: (usersRes as any)?.total || (usersRes as any)?.records?.length || 0,
				buckets: Array.isArray((storageRes as any)?.data) ? (storageRes as any).data.length : Array.isArray(storageRes) ? (storageRes as any).length : 0,
				blocks: blocks.length,
				totalProducts: (productStats as any)?.total_products || 0,
				totalPurchases: (productStats as any)?.total_purchases || 0,
				totalRevenue: (productStats as any)?.total_revenue || 0,
			});
			setLoading(false);
		});
	}, []);

	if (loading) return html`<${LoadingSpinner} message="Loading dashboard..." />`;

	const revenue = typeof stats?.totalRevenue === 'number' ? `$${stats.totalRevenue.toFixed(2)}` : '$0.00';

	return html`
		<div>
			<${PageHeader} title="Dashboard" description="Overview of your Solobase instance" />
			<div style=${{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(220px, 1fr))', gap: '1rem', marginBottom: '2rem' }}>
				<${StatCard} title="Total Users" value=${stats?.users || 0} icon=${Users} />
				<${StatCard} title="Storage Buckets" value=${stats?.buckets || 0} icon=${HardDrive} />
				<${StatCard} title="Active Blocks" value=${stats?.blocks || 0} icon=${Layers} />
				<${StatCard} title="Products" value=${stats?.totalProducts || 0} icon=${ShoppingCart} />
				<${StatCard} title="Purchases" value=${stats?.totalPurchases || 0} icon=${ShoppingCart} />
				<${StatCard} title="Revenue" value=${revenue} icon=${DollarSign} />
			</div>
		</div>
	`;
}

function UsersTab() {
	const [users, setUsers] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [search, setSearch] = useState('');
	const [allRoles, setAllRoles] = useState<string[]>([]);
	const [userRoleAssignments, setUserRoleAssignments] = useState<any[]>([]);

	const fetchUsers = () => {
		setLoading(true);
		Promise.all([
			api.getUsers(1, 100),
			api.get('/admin/iam/roles').catch(() => ({ records: [] })),
			api.get('/admin/iam/user-roles').catch(() => ({ records: [] })),
		]).then(([usersRes, rolesRes, urRes]) => {
			if (!(usersRes as any).error) {
				const records = ((usersRes as any).data as any)?.records || ((usersRes as any).data as any)?.data || [];
				setUsers((Array.isArray(records) ? records : []).map((r: any) => ({ id: r.id, ...r.data })));
			}
			const roles = ((rolesRes as any)?.records || []).map((r: any) => r.data?.name || r.name).filter(Boolean);
			setAllRoles(roles);
			setUserRoleAssignments(((urRes as any)?.records || []).map((r: any) => ({ id: r.id, ...r.data })));
			setLoading(false);
		});
	};

	useEffect(fetchUsers, []);

	const isDisabled = (user: any) => user.disabled === true || user.disabled === 1 || user.disabled === '1' || user.disabled === 'true';

	const toggleDisable = async (user: any) => {
		const currently = isDisabled(user);
		if (!confirm(`${currently ? 'Enable' : 'Disable'} ${user.email}?`)) return;
		await api.updateUser(user.id, { disabled: !currently } as any);
		fetchUsers();
	};

	const addRole = async (userId: string, role: string) => {
		await api.post('/admin/iam/user-roles', { user_id: userId, role });
		fetchUsers();
	};

	const removeRole = async (userId: string, role: string) => {
		const assignment = userRoleAssignments.find((a: any) => a.user_id === userId && a.role === role);
		if (assignment) {
			await api.delete(`/admin/iam/user-roles/${assignment.id}`);
			fetchUsers();
		}
	};

	const deleteUser = async (user: any) => {
		if (!confirm(`Delete ${user.email}? This cannot be undone.`)) return;
		await api.deleteUser(user.id);
		fetchUsers();
	};

	const filtered = search
		? users.filter(u => u.email?.toLowerCase().includes(search.toLowerCase()))
		: users;

	const columns = [
		{ key: 'email', label: 'Email', sortable: true },
		{ key: 'name', label: 'Name', sortable: true, render: (v: string) => v || '-' },
		{ key: 'roles', label: 'Roles', render: (v: any, row: any) => {
			const roles = Array.isArray(v) ? v : v ? [v] : [];
			const available = allRoles.filter(r => !roles.includes(r));
			const isSelf = currentUser.value?.id === row.id;
			return html`<div style=${{ display: 'flex', alignItems: 'center', gap: '0.25rem', flexWrap: 'wrap' }}>
				${roles.map((r: string) => html`<span style=${{
					fontSize: '0.688rem', padding: isSelf ? '0.125rem 0.5rem' : '0.125rem 0.375rem 0.125rem 0.5rem', borderRadius: '9999px', display: 'inline-flex', alignItems: 'center', gap: '0.25rem',
					background: r === 'admin' ? '#dbeafe' : '#f1f5f9', color: r === 'admin' ? '#1d4ed8' : '#64748b'
				}}>${r}${!isSelf ? html`<button onClick=${() => removeRole(row.id, r)} style=${{ background: 'none', border: 'none', cursor: 'pointer', padding: 0, fontSize: '0.75rem', color: 'inherit', opacity: 0.5, lineHeight: 1 }}>\u00d7</button>` : null}</span>`)}
				${!isSelf && available.length > 0 ? html`<select onChange=${(e: any) => { if (e.target.value) { addRole(row.id, e.target.value); e.target.value = ''; } }}
					style=${{ fontSize: '0.625rem', padding: '0.125rem', border: '1px solid #e2e8f0', borderRadius: '4px', color: '#94a3b8', cursor: 'pointer' }}>
					<option value="">+</option>
					${available.map((r: string) => html`<option value=${r}>${r}</option>`)}
				</select>` : null}
			</div>`;
		}},
		{ key: 'disabled', label: 'Status', render: (v: any) => {
			const isDisabled = v === true || v === 1 || v === '1' || v === 'true';
			return html`<span style=${{
				fontSize: '0.688rem', padding: '0.125rem 0.5rem', borderRadius: '9999px',
				background: isDisabled ? '#fef2f2' : '#dcfce7', color: isDisabled ? '#dc2626' : '#166534'
			}}>${isDisabled ? 'Disabled' : 'Active'}</span>`;
		}},
		{ key: 'created_at', label: 'Joined', sortable: true, render: (v: string) => v ? new Date(v).toLocaleDateString() : '-' },
		{ key: '_actions', label: '', render: (_: any, row: any) => {
			if (currentUser.value?.id === row.id) return html`<span style=${{ fontSize: '0.625rem', color: '#94a3b8' }}>you</span>`;
			return html`
				<div style=${{ display: 'flex', gap: '0.375rem' }}>
					<button onClick=${() => toggleDisable(row)}
						style=${{ padding: '0.25rem 0.5rem', fontSize: '0.688rem', border: '1px solid #e2e8f0', borderRadius: '4px', background: 'white', cursor: 'pointer', color: '#64748b' }}>
						${isDisabled(row) ? 'Enable' : 'Disable'}
					</button>
					<button onClick=${() => deleteUser(row)}
						style=${{ padding: '0.25rem 0.5rem', fontSize: '0.688rem', border: '1px solid #fecaca', borderRadius: '4px', background: 'white', cursor: 'pointer', color: '#dc2626' }}>
						Delete
					</button>
				</div>
			`;
		}},
	];

	if (loading) return html`<${LoadingSpinner} message="Loading users..." />`;

	return html`
		<div>
			<${SearchInput} value=${search} onChange=${setSearch} placeholder="Search users..." />
			<${DataTable} columns=${columns} data=${filtered} emptyMessage="No users found" />
		</div>
	`;
}

function SettingsTab() {
	const [variables, setVariables] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [editing, setEditing] = useState<string | null>(null);
	const [editValue, setEditValue] = useState('');
	const [saving, setSaving] = useState(false);
	const [revealed, setRevealed] = useState<Set<string>>(new Set());
	const [showAdd, setShowAdd] = useState(false);
	const [newKey, setNewKey] = useState('');
	const [newValue, setNewValue] = useState('');
	const [newName, setNewName] = useState('');

	const loadSettings = () => {
		setLoading(true);
		api.get('/admin/settings/all').then((data: any) => {
			setVariables(Array.isArray(data) ? data : []);
			setLoading(false);
		}).catch(() => setLoading(false));
	};

	useEffect(loadSettings, []);

	const toggleReveal = (key: string) => {
		setRevealed(prev => {
			const next = new Set(prev);
			next.has(key) ? next.delete(key) : next.add(key);
			return next;
		});
	};

	const startEdit = (v: any) => {
		if (v.sensitive && v.warning && !confirm(v.warning + '\n\nDo you want to edit this value?')) return;
		setEditing(v.key);
		setEditValue(v.value || '');
	};

	const handleSave = async (key: string) => {
		setSaving(true);
		try { await api.patch('/admin/settings/' + key, { value: editValue }); } catch {}
		setEditing(null);
		setSaving(false);
		loadSettings();
	};

	const handleAdd = async () => {
		if (!newKey.trim()) return;
		setSaving(true);
		try { await api.post('/admin/settings', { key: newKey, value: newValue, name: newName || newKey }); } catch {}
		setShowAdd(false);
		setNewKey('');
		setNewValue('');
		setNewName('');
		setSaving(false);
		loadSettings();
	};

	const handleDelete = async (key: string) => {
		if (!confirm(`Delete variable "${key}"? This cannot be undone.`)) return;
		try { await api.delete('/admin/settings/' + key); } catch {}
		loadSettings();
	};

	if (loading) return html`<${LoadingSpinner} message="Loading settings..." />`;

	return html`
		<div>
			<${PageHeader} title="Settings" description="Instance configuration variables">
				<button onClick=${() => setShowAdd(!showAdd)}
					style=${{ padding: '0.5rem 1rem', background: '#fe6627', color: 'white', border: 'none', borderRadius: '8px', fontSize: '0.813rem', fontWeight: 600, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '0.375rem' }}>
					<${Plus} size=${14} /> Add Variable
				</button>
			<//>
			${showAdd ? html`
				<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '8px', padding: '1rem', marginBottom: '0.75rem', display: 'flex', gap: '0.5rem', alignItems: 'flex-end', flexWrap: 'wrap' }}>
					<div style=${{ flex: '1 1 150px' }}>
						<label style=${{ fontSize: '0.688rem', fontWeight: 600, color: '#94a3b8', display: 'block', marginBottom: '0.25rem' }}>KEY</label>
						<input value=${newKey} onInput=${(e: any) => setNewKey(e.target.value)} placeholder="MY_VARIABLE"
							style=${{ width: '100%', padding: '0.5rem', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.813rem', fontFamily: 'monospace' }} />
					</div>
					<div style=${{ flex: '1 1 150px' }}>
						<label style=${{ fontSize: '0.688rem', fontWeight: 600, color: '#94a3b8', display: 'block', marginBottom: '0.25rem' }}>NAME</label>
						<input value=${newName} onInput=${(e: any) => setNewName(e.target.value)} placeholder="Display Name"
							style=${{ width: '100%', padding: '0.5rem', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.813rem' }} />
					</div>
					<div style=${{ flex: '2 1 200px' }}>
						<label style=${{ fontSize: '0.688rem', fontWeight: 600, color: '#94a3b8', display: 'block', marginBottom: '0.25rem' }}>VALUE</label>
						<input value=${newValue} onInput=${(e: any) => setNewValue(e.target.value)} placeholder="value"
							style=${{ width: '100%', padding: '0.5rem', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.813rem' }} />
					</div>
					<button onClick=${handleAdd} disabled=${saving || !newKey.trim()}
						style=${{ padding: '0.5rem 1rem', background: '#fe6627', color: 'white', border: 'none', borderRadius: '6px', fontSize: '0.813rem', fontWeight: 600, cursor: 'pointer', whiteSpace: 'nowrap' }}>
						${saving ? 'Adding...' : 'Add'}
					</button>
					<button onClick=${() => setShowAdd(false)}
						style=${{ padding: '0.5rem 0.75rem', background: '#f1f5f9', color: '#64748b', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.813rem', cursor: 'pointer' }}>
						Cancel
					</button>
				</div>
			` : null}
			<div style=${{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
				${variables.map((v: any) => {
					const isRevealed = revealed.has(v.key);
					const displayValue = v.sensitive && !isRevealed ? '••••••••' : (v.value || '(empty)');
					return html`
					<div key=${v.key} style=${{
						background: 'white',
						border: '1px solid var(--border-color, #e2e8f0)',
						borderRadius: '8px',
						padding: '1rem 1.25rem',
					}}>
						<div style=${{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '0.25rem' }}>
							<div>
								<div style=${{ fontWeight: 600, fontSize: '0.875rem', color: 'var(--text-primary, #1e293b)' }}>${v.name || v.key}</div>
								${v.description ? html`<div style=${{ fontSize: '0.75rem', color: 'var(--text-secondary, #64748b)', marginTop: '0.125rem' }}>${v.description}</div>` : null}
							</div>
							<div style=${{ display: 'flex', alignItems: 'center', gap: '0.375rem' }}>
								<code style=${{ fontSize: '0.75rem', color: '#64748b', background: '#f1f5f9', padding: '0.125rem 0.375rem', borderRadius: '4px' }}>${v.key}</code>
								${!v.system ? html`
									<button onClick=${() => handleDelete(v.key)} title="Delete"
										style=${{ padding: '0.25rem', background: 'none', border: 'none', cursor: 'pointer', color: '#dc2626', opacity: 0.5, display: 'flex' }}>
										<${Trash2} size=${14} />
									</button>
								` : null}
							</div>
						</div>
						${v.warning ? html`<div style=${{ fontSize: '0.75rem', color: '#dc2626', marginTop: '0.25rem' }}>${v.warning}</div>` : null}
						${editing === v.key ? html`
							<div style=${{ display: 'flex', gap: '0.5rem', marginTop: '0.5rem' }}>
								<input
									type="text"
									value=${editValue}
									onInput=${(e: any) => setEditValue(e.target.value)}
									style=${{ flex: 1, padding: '0.5rem 0.75rem', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.813rem', fontFamily: v.sensitive ? 'monospace' : 'inherit' }}
									disabled=${saving}
								/>
								<button onClick=${() => handleSave(v.key)} disabled=${saving}
									style=${{ padding: '0.5rem 1rem', background: '#fe6627', color: 'white', border: 'none', borderRadius: '6px', fontSize: '0.813rem', fontWeight: 600, cursor: 'pointer' }}>
									${saving ? 'Saving...' : 'Save'}
								</button>
								<button onClick=${() => setEditing(null)}
									style=${{ padding: '0.5rem 0.75rem', background: '#f1f5f9', color: '#64748b', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.813rem', cursor: 'pointer' }}>
									Cancel
								</button>
							</div>
						` : html`
							<div style=${{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginTop: '0.5rem' }}>
								<code style=${{ fontSize: '0.813rem', color: 'var(--text-primary, #1e293b)', background: '#f8fafc', padding: '0.375rem 0.625rem', borderRadius: '6px', flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
									${displayValue}
								</code>
								${v.sensitive ? html`
									<button onClick=${() => toggleReveal(v.key)} title=${isRevealed ? 'Hide' : 'Reveal'}
										style=${{ padding: '0.375rem', background: 'white', color: '#64748b', border: '1px solid #e2e8f0', borderRadius: '6px', cursor: 'pointer', display: 'flex' }}>
										${isRevealed ? html`<${EyeOff} size=${14} />` : html`<${Eye} size=${14} />`}
									</button>
								` : null}
								<button onClick=${() => startEdit(v)}
									style=${{ padding: '0.375rem 0.75rem', background: 'white', color: '#64748b', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.75rem', cursor: 'pointer', whiteSpace: 'nowrap' }}>
									Edit
								</button>
							</div>
						`}
					</div>
				`})}
				${variables.length === 0 ? html`<p style=${{ color: 'var(--text-secondary, #64748b)' }}>No settings configured</p>` : null}
			</div>
		</div>
	`;
}

function BlocksTab() {
	const [blocks, setBlocks] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		api.getExtensions().then(res => {
			const data = Array.isArray(res) ? res : ((res as any)?.data || []);
			setBlocks(Array.isArray(data) ? data : []);
			setLoading(false);
		}).catch(() => setLoading(false));
	}, []);

	if (loading) return html`<${LoadingSpinner} message="Loading blocks..." />`;

	return html`
		<div>
			<${PageHeader} title="Blocks" description="Registered WAFER blocks in this instance" />
			<div style=${{ marginBottom: '1rem' }}>
				<a href="/debug/inspector/ui" target="_blank" style=${{ display: 'inline-flex', alignItems: 'center', gap: '0.375rem', fontSize: '0.813rem', color: '#fe6627', textDecoration: 'none', fontWeight: 500 }}>
					Open Inspector UI <${ExternalLink} size=${14} />
				</a>
			</div>
			<div style=${{ display: 'grid', gap: '0.5rem' }}>
				${blocks.map((b: any) => html`
					<div key=${b.name} style=${{
						background: 'white',
						border: '1px solid var(--border-color, #e2e8f0)',
						borderRadius: '8px',
						padding: '0.875rem 1.25rem',
						display: 'flex',
						justifyContent: 'space-between',
						alignItems: 'center'
					}}>
						<div>
							<div style=${{ fontWeight: 600, fontSize: '0.875rem', color: 'var(--text-primary, #1e293b)' }}>${b.name}</div>
							<div style=${{ fontSize: '0.75rem', color: 'var(--text-secondary, #64748b)', marginTop: '0.125rem' }}>
								${b.version || ''} ${b.summary ? `\u2014 ${b.summary}` : ''}
							</div>
						</div>
						<div style=${{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
							<span style=${{
								fontSize: '0.688rem',
								padding: '0.125rem 0.5rem',
								borderRadius: '9999px',
								background: '#f1f5f9',
								color: '#64748b',
							}}>${b.interface || ''}</span>
							<span style=${{
								fontSize: '0.688rem',
								padding: '0.125rem 0.5rem',
								borderRadius: '9999px',
								background: '#dcfce7',
								color: '#166534',
							}}>Active</span>
						</div>
					</div>
				`)}
				${blocks.length === 0 ? html`<p style=${{ color: 'var(--text-secondary, #64748b)' }}>No blocks registered</p>` : null}
			</div>
		</div>
	`;
}

function DatabaseTab() {
	const [tables, setTables] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [query, setQuery] = useState('');
	const [results, setResults] = useState<any>(null);
	const [queryError, setQueryError] = useState('');
	const [querying, setQuerying] = useState(false);
	const [selectedTable, setSelectedTable] = useState<string | null>(null);
	const [columns, setColumns] = useState<any[]>([]);

	useEffect(() => {
		api.get('/admin/database/tables').then((data: any) => {
			setTables(Array.isArray(data) ? data : []);
			setLoading(false);
		}).catch(() => setLoading(false));
	}, []);

	const selectTable = async (name: string) => {
		setSelectedTable(name);
		setQuery(`SELECT * FROM ${name} LIMIT 50`);
		try {
			const cols: any = await api.get(`/admin/database/tables/${name}/columns`);
			setColumns(cols?.columns || []);
		} catch { setColumns([]); }
	};

	const runQuery = async () => {
		if (!query.trim()) return;
		setQuerying(true);
		setQueryError('');
		setResults(null);
		try {
			const data: any = await api.post('/admin/database/query', { query });
			if (data?.error) {
				setQueryError(typeof data.error === 'string' ? data.error : data.error.message || 'Query failed');
			} else {
				setResults(data);
			}
		} catch (e: any) {
			setQueryError(e.message || 'Query failed');
		}
		setQuerying(false);
	};

	if (loading) return html`<${LoadingSpinner} message="Loading tables..." />`;

	const resultRows = results?.rows?.map((r: any) => r.data || r) || [];
	const resultColumns = resultRows.length > 0
		? Object.keys(resultRows[0]).filter(k => k !== 'id').map(k => ({
			key: k, label: k, sortable: true,
			render: (v: any) => v == null ? html`<span style=${{ color: '#94a3b8' }}>NULL</span>` : String(v).length > 80 ? String(v).slice(0, 80) + '...' : String(v)
		}))
		: [];

	return html`
		<div>
			<${PageHeader} title="Database" description="Browse tables and run queries" />
			<div style=${{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap', marginBottom: '0.75rem' }}>
				${tables.map((t: any) => html`
					<button key=${t.name} onClick=${() => selectTable(t.name)}
						style=${{
							padding: '0.375rem 0.75rem', border: '1px solid #e2e8f0', borderRadius: '6px', cursor: 'pointer',
							fontSize: '0.75rem', display: 'inline-flex', alignItems: 'center', gap: '0.375rem',
							background: selectedTable === t.name ? '#f1f5f9' : 'white',
							color: selectedTable === t.name ? '#1e293b' : '#64748b',
							fontWeight: selectedTable === t.name ? 600 : 400,
						}}>
						<${Database} size=${12} />${t.name}
						<span style=${{ fontSize: '0.625rem', color: '#94a3b8' }}>(${t.row_count})</span>
					</button>
				`)}
			</div>
			${selectedTable && columns.length > 0 ? html`
				<div style=${{ marginBottom: '0.75rem', background: 'white', border: '1px solid #e2e8f0', borderRadius: '8px', padding: '0.75rem' }}>
					<div style=${{ fontSize: '0.688rem', fontWeight: 600, color: '#94a3b8', textTransform: 'uppercase', marginBottom: '0.375rem' }}>
						${selectedTable} columns
					</div>
					<div style=${{ display: 'flex', flexWrap: 'wrap', gap: '0.375rem' }}>
						${columns.map((c: any) => html`
							<span style=${{ fontSize: '0.688rem', padding: '0.125rem 0.5rem', borderRadius: '4px', background: c.pk ? '#dbeafe' : '#f1f5f9', color: c.pk ? '#1d4ed8' : '#64748b' }}>
								${c.name} <span style=${{ opacity: 0.6 }}>${c.type}</span>
							</span>
						`)}
					</div>
				</div>
			` : null}
			<div style=${{ marginBottom: '0.75rem' }}>
				<textarea
					value=${query}
					onInput=${(e: any) => setQuery(e.target.value)}
					onKeyDown=${(e: any) => { if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') runQuery(); }}
					placeholder="SELECT * FROM table_name LIMIT 50 (Ctrl+Enter to run)"
					style=${{ width: '100%', boxSizing: 'border-box', padding: '0.75rem', border: '1px solid #e2e8f0', borderRadius: '8px', fontSize: '0.813rem', fontFamily: 'monospace', minHeight: '80px', resize: 'vertical' }}
				/>
				<div style=${{ display: 'flex', justifyContent: 'flex-end', marginTop: '0.5rem' }}>
					<button onClick=${runQuery} disabled=${querying}
						style=${{ padding: '0.5rem 1.25rem', background: '#fe6627', color: 'white', border: 'none', borderRadius: '8px', cursor: 'pointer', fontWeight: 600, fontSize: '0.813rem', display: 'flex', alignItems: 'center', gap: '0.375rem' }}>
						<${Play} size=${14} /> ${querying ? 'Running...' : 'Run Query'}
					</button>
				</div>
			</div>
			${queryError ? html`<div style=${{ background: '#fef2f2', border: '1px solid #fecaca', borderRadius: '8px', padding: '0.75rem', marginBottom: '0.75rem', fontSize: '0.813rem', color: '#dc2626' }}>${queryError}</div>` : null}
			${results ? html`
				<div style=${{ fontSize: '0.75rem', color: '#64748b', marginBottom: '0.5rem' }}>${results.row_count ?? resultRows.length} row${(results.row_count ?? resultRows.length) !== 1 ? 's' : ''}</div>
				<div style=${{ overflow: 'auto' }}>
					<${DataTable} columns=${resultColumns} data=${resultRows} emptyMessage="Query returned no results" />
				</div>
			` : null}
		</div>
	`;
}

function StorageTab() {
	const [buckets, setBuckets] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [selectedBucket, setSelectedBucket] = useState<string | null>(null);
	const [objects, setObjects] = useState<any[]>([]);
	const [objectsLoading, setObjectsLoading] = useState(false);

	useEffect(() => {
		api.get('/storage/buckets').then((data: any) => {
			const b = data?.buckets || [];
			setBuckets(b);
			if (b.length > 0) selectBucket(b[0].name);
			setLoading(false);
		}).catch(() => setLoading(false));
	}, []);

	const selectBucket = async (name: string) => {
		setSelectedBucket(name);
		setObjectsLoading(true);
		try {
			const data: any = await api.get(`/storage/buckets/${name}/objects`);
			setObjects(data?.objects || []);
		} catch { setObjects([]); }
		setObjectsLoading(false);
	};

	const formatSize = (bytes: number) => {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	};

	if (loading) return html`<${LoadingSpinner} message="Loading storage..." />`;

	const columns = [
		{ key: 'key', label: 'Name', sortable: true },
		{ key: 'size', label: 'Size', sortable: true, render: (v: number) => formatSize(v) },
		{ key: 'content_type', label: 'Type', sortable: true, render: (v: string) => html`<span style=${{ fontSize: '0.75rem', color: '#64748b' }}>${v}</span>` },
		{ key: 'last_modified', label: 'Modified', sortable: true, render: (v: string) => v ? new Date(v).toLocaleString() : '-' },
		{ key: '_actions', label: '', render: (_: any, row: any) => html`
			<a href=${`/storage/buckets/${selectedBucket}/objects/${row.key}`} target="_blank"
				style=${{ padding: '0.25rem 0.5rem', fontSize: '0.688rem', border: '1px solid #e2e8f0', borderRadius: '4px', background: 'white', color: '#64748b', textDecoration: 'none', display: 'inline-flex', alignItems: 'center', gap: '0.25rem' }}>
				<${ExternalLink} size=${12} /> View
			</a>
		` },
	];

	return html`
		<div>
			<${PageHeader} title="Storage" description="Browse storage buckets and files" />
			<div style=${{ display: 'flex', gap: '0.5rem', marginBottom: '1rem' }}>
				${buckets.map((b: any) => html`
					<button key=${b.name} onClick=${() => selectBucket(b.name)}
						style=${{
							padding: '0.5rem 1rem', border: '1px solid #e2e8f0', borderRadius: '8px', cursor: 'pointer',
							fontSize: '0.813rem', fontWeight: selectedBucket === b.name ? 600 : 400,
							background: selectedBucket === b.name ? '#f1f5f9' : 'white',
							color: selectedBucket === b.name ? '#1e293b' : '#64748b',
						}}>
						<${FolderOpen} size=${14} style=${{ marginRight: '0.375rem', verticalAlign: 'text-bottom' }} />${b.name}
					</button>
				`)}
				${buckets.length === 0 ? html`<p style=${{ color: '#64748b' }}>No storage buckets</p>` : null}
			</div>
			${objectsLoading ? html`<${LoadingSpinner} message="Loading files..." />` :
				selectedBucket ? html`
					<div style=${{ fontSize: '0.75rem', color: '#64748b', marginBottom: '0.5rem' }}>${objects.length} object${objects.length !== 1 ? 's' : ''} in ${selectedBucket}</div>
					<${DataTable} columns=${columns} data=${objects} emptyMessage="Bucket is empty" />
				` : null
			}
		</div>
	`;
}

function RolesPanel() {
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
		const isSystem = role.is_system === true || role.is_system === '1' || role.is_system === 'true';
		if (isSystem) { alert('Cannot delete system role'); return; }
		if (!confirm(`Delete role "${role.name}"?`)) return;
		await api.delete(`/admin/iam/roles/${role.id}`);
		fetchRoles();
	};

	const columns = [
		{ key: 'name', label: 'Role', sortable: true, render: (v: string, row: any) => {
			const isSystem = row.is_system === true || row.is_system === '1' || row.is_system === 'true';
			return html`<div style=${{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
				<span style=${{ fontWeight: 600 }}>${v}</span>
				${isSystem ? html`<span style=${{ fontSize: '0.625rem', padding: '0.0625rem 0.375rem', borderRadius: '4px', background: '#dbeafe', color: '#1d4ed8' }}>system</span>` : null}
			</div>`;
		}},
		{ key: 'description', label: 'Description' },
		{ key: '_actions', label: '', render: (_: any, row: any) => {
			const isSystem = row.is_system === true || row.is_system === '1' || row.is_system === 'true';
			return isSystem ? null : html`<button onClick=${() => deleteRole(row)} style=${{ padding: '0.25rem', background: 'none', border: 'none', cursor: 'pointer', color: '#dc2626', opacity: 0.5, display: 'flex' }}><${Trash2} size=${14} /></button>`;
		}},
	];

	if (loading) return html`<${LoadingSpinner} message="Loading roles..." />`;

	return html`
		<div>
			<div style=${{ display: 'flex', justifyContent: 'flex-end', marginBottom: '0.75rem' }}>
				<button onClick=${() => setShowCreate(!showCreate)}
					style=${{ padding: '0.5rem 1rem', background: '#fe6627', color: 'white', border: 'none', borderRadius: '8px', fontSize: '0.813rem', fontWeight: 600, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '0.375rem' }}>
					<${Plus} size=${14} /> Add Role
				</button>
			</div>
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
						style=${{ padding: '0.5rem 1rem', background: '#fe6627', color: 'white', border: 'none', borderRadius: '6px', fontSize: '0.813rem', fontWeight: 600, cursor: 'pointer', whiteSpace: 'nowrap' }}>Create</button>
					<button onClick=${() => setShowCreate(false)}
						style=${{ padding: '0.5rem 0.75rem', background: '#f1f5f9', color: '#64748b', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.813rem', cursor: 'pointer' }}>Cancel</button>
				</div>
			` : null}
			<${DataTable} columns=${columns} data=${roles} emptyMessage="No roles defined" />
		</div>
	`;
}

function ApiKeysPanel() {
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
			if (data?.key) { setCreatedKey(data.key); setShowCreate(false); setNewName(''); fetchKeys(); }
		} catch {}
	};

	const revokeKey = async (key: any) => {
		if (!confirm(`Revoke API key "${key.name}"?`)) return;
		await api.patch(`/auth/api-keys/${key.id}`, {});
		fetchKeys();
	};

	const deleteKey = async (key: any) => {
		if (!confirm(`Permanently delete API key "${key.name}"?`)) return;
		await api.delete(`/auth/api-keys/${key.id}`);
		fetchKeys();
	};

	const copyKey = () => {
		if (createdKey) { navigator.clipboard.writeText(createdKey); setCopied(true); setTimeout(() => setCopied(false), 2000); }
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
				${!row.revoked_at ? html`<button onClick=${() => revokeKey(row)}
					style=${{ padding: '0.25rem 0.5rem', fontSize: '0.688rem', border: '1px solid #fde68a', borderRadius: '4px', background: 'white', cursor: 'pointer', color: '#92400e' }}>Revoke</button>` : null}
				<button onClick=${() => deleteKey(row)} style=${{ padding: '0.25rem', background: 'none', border: 'none', cursor: 'pointer', color: '#dc2626', opacity: 0.5, display: 'flex' }}><${Trash2} size=${14} /></button>
			</div>
		` },
	];

	if (loading) return html`<${LoadingSpinner} message="Loading API keys..." />`;

	return html`
		<div>
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
			<div style=${{ display: 'flex', justifyContent: 'flex-end', marginBottom: '0.75rem' }}>
				<button onClick=${() => { setShowCreate(!showCreate); setCreatedKey(null); }}
					style=${{ padding: '0.5rem 1rem', background: '#fe6627', color: 'white', border: 'none', borderRadius: '8px', fontSize: '0.813rem', fontWeight: 600, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '0.375rem' }}>
					<${Plus} size=${14} /> Create Key
				</button>
			</div>
			${showCreate ? html`
				<div style=${{ background: 'white', border: '1px solid #e2e8f0', borderRadius: '8px', padding: '1rem', marginBottom: '1rem', display: 'flex', gap: '0.5rem', alignItems: 'flex-end' }}>
					<div style=${{ flex: 1 }}>
						<label style=${{ fontSize: '0.688rem', fontWeight: 600, color: '#94a3b8', display: 'block', marginBottom: '0.25rem' }}>KEY NAME</label>
						<input value=${newName} onInput=${(e: any) => setNewName(e.target.value)} placeholder="my-app-key"
							onKeyDown=${(e: any) => { if (e.key === 'Enter') createKey(); }}
							style=${{ width: '100%', padding: '0.5rem', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.813rem' }} />
					</div>
					<button onClick=${createKey} disabled=${!newName.trim()}
						style=${{ padding: '0.5rem 1rem', background: '#fe6627', color: 'white', border: 'none', borderRadius: '6px', fontSize: '0.813rem', fontWeight: 600, cursor: 'pointer', whiteSpace: 'nowrap' }}>Create</button>
					<button onClick=${() => setShowCreate(false)}
						style=${{ padding: '0.5rem 0.75rem', background: '#f1f5f9', color: '#64748b', border: '1px solid #e2e8f0', borderRadius: '6px', fontSize: '0.813rem', cursor: 'pointer' }}>Cancel</button>
				</div>
			` : null}
			<${DataTable} columns=${columns} data=${keys} emptyMessage="No API keys created yet" />
		</div>
	`;
}

function UsersPage() {
	const [subTab, setSubTab] = useState('users');
	const subTabs = [
		{ id: 'users', label: 'Users', icon: Users },
		{ id: 'roles', label: 'Roles', icon: Shield },
		{ id: 'api-keys', label: 'API Keys', icon: Key },
	];
	return html`
		<div>
			<${PageHeader} title="Users & Access" description="Manage users, roles, and API keys" />
			<${TabNavigation} tabs=${subTabs} activeTab=${subTab} onTabChange=${setSubTab} />
			${subTab === 'users' ? html`<${UsersTab} />` : null}
			${subTab === 'roles' ? html`<${RolesPanel} />` : null}
			${subTab === 'api-keys' ? html`<${ApiKeysPanel} />` : null}
		</div>
	`;
}

export function App() {
	const [tab, setTab] = useState(() => {
		const hash = window.location.hash.slice(1);
		return hash || 'dashboard';
	});

	useEffect(() => {
		window.location.hash = tab;
	}, [tab]);

	useEffect(() => {
		function onHash() { setTab(window.location.hash.slice(1) || 'dashboard'); }
		window.addEventListener('hashchange', onHash);
		return () => window.removeEventListener('hashchange', onHash);
	}, []);

	return html`
		<${BlockShell} title="Admin">
			${tab === 'dashboard' ? html`<${DashboardTab} />` : null}
			${tab === 'users' ? html`<${UsersPage} />` : null}
			${tab === 'database' ? html`<${DatabaseTab} />` : null}
			${tab === 'storage' ? html`<${StorageTab} />` : null}
			${tab === 'blocks' ? html`<${BlocksTab} />` : null}
			${tab === 'settings' ? html`<${SettingsTab} />` : null}
		<//>
	`;
}
