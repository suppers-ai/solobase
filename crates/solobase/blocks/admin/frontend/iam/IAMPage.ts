import { html } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { PageHeader, TabNavigation, LoadingSpinner, ConfirmDialog, authFetch, ErrorHandler } from '@solobase/ui';

interface Role {
	name: string;
	displayName?: string;
	description: string;
	type?: string;
	metadata?: Record<string, unknown>;
}

interface Policy {
	id?: string;
	subject: string;
	resource: string;
	action: string;
	effect: 'allow' | 'deny';
}

interface User {
	id: string;
	email: string;
	firstName?: string;
	lastName?: string;
	roles?: { name: string; displayName?: string }[];
}

const tabs = [
	{ id: 'roles', label: 'Roles' },
	{ id: 'policies', label: 'Policies' },
	{ id: 'users', label: 'User Assignments' },
	{ id: 'test', label: 'Test Permissions' },
	{ id: 'audit', label: 'Audit Log' },
];

export function IAMPage() {
	const [activeTab, setActiveTab] = useState('roles');
	const [roles, setRoles] = useState<Role[]>([]);
	const [policies, setPolicies] = useState<Policy[]>([]);
	const [users, setUsers] = useState<User[]>([]);
	const [loading, setLoading] = useState(true);
	const [showDeleteRoleConfirm, setShowDeleteRoleConfirm] = useState(false);
	const [showDeletePolicyConfirm, setShowDeletePolicyConfirm] = useState(false);
	const [roleToDelete, setRoleToDelete] = useState<Role | null>(null);
	const [policyToDelete, setPolicyToDelete] = useState<Policy | null>(null);

	// Add/edit role modal
	const [showRoleModal, setShowRoleModal] = useState(false);
	const [editingRole, setEditingRole] = useState<Partial<Role>>({ name: '', description: '' });

	// Add policy modal
	const [showPolicyModal, setShowPolicyModal] = useState(false);
	const [editingPolicy, setEditingPolicy] = useState<Partial<Policy>>({ subject: '', resource: '', action: '', effect: 'allow' });

	async function loadRoles() {
		try {
			const response = await authFetch('/api/admin/iam/roles');
			if (response.ok) setRoles(await response.json());
			else setRoles([]);
		} catch { setRoles([]); }
	}

	async function loadPolicies() {
		try {
			const response = await authFetch('/api/admin/iam/policies');
			if (response.ok) setPolicies(await response.json());
			else setPolicies([]);
		} catch { setPolicies([]); }
	}

	async function loadUsers() {
		try {
			const response = await authFetch('/api/admin/iam/users');
			if (response.ok) setUsers(await response.json());
			else setUsers([]);
		} catch { setUsers([]); }
	}

	async function loadData() {
		setLoading(true);
		await Promise.all([loadRoles(), loadPolicies(), loadUsers()]);
		setLoading(false);
	}

	useEffect(() => { loadData(); }, []);

	async function createRole() {
		try {
			const response = await authFetch('/api/admin/iam/roles', {
				method: 'POST',
				body: JSON.stringify(editingRole),
			});
			if (response.ok) { await loadRoles(); setShowRoleModal(false); }
			else ErrorHandler.handle('Failed to create role');
		} catch (e) { ErrorHandler.handle(e); }
	}

	async function confirmDeleteRole() {
		if (!roleToDelete) return;
		setShowDeleteRoleConfirm(false);
		const response = await authFetch(`/api/admin/iam/roles/${roleToDelete.name}`, { method: 'DELETE' });
		if (response.ok) await loadRoles();
		else ErrorHandler.handle('Failed to delete role');
		setRoleToDelete(null);
	}

	async function createPolicy() {
		try {
			const response = await authFetch('/api/admin/iam/policies', {
				method: 'POST',
				body: JSON.stringify(editingPolicy),
			});
			if (response.ok) { await loadPolicies(); setShowPolicyModal(false); }
			else ErrorHandler.handle('Failed to create policy');
		} catch (e) { ErrorHandler.handle(e); }
	}

	async function confirmDeletePolicy() {
		if (!policyToDelete) return;
		setShowDeletePolicyConfirm(false);
		const response = await authFetch(`/api/admin/iam/policies/${policyToDelete.id}`, { method: 'DELETE' });
		if (response.ok) await loadPolicies();
		else ErrorHandler.handle('Failed to delete policy');
		setPolicyToDelete(null);
	}

	return html`
		<>
			<div style=${{ maxWidth: '1200px', margin: '0 auto' }}>
				<${PageHeader} title="Identity & Access Management" description="Manage roles, permissions, and access policies" />

				${loading ? html`
					<div style=${{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', padding: '3rem', gap: '1rem', color: '#666' }}>
						<${LoadingSpinner} size="lg" />
						<p>Loading IAM configuration...</p>
					</div>
				` : html`
					<${TabNavigation} tabs=${tabs} activeTab=${activeTab} onChange=${setActiveTab} />

					<div style=${{ marginTop: '2rem' }}>
						${activeTab === 'roles' ? renderRolesTab() : null}
						${activeTab === 'policies' ? renderPoliciesTab() : null}
						${activeTab === 'users' ? renderUsersTab() : null}
						${activeTab === 'test' ? html`<div style=${{ padding: '2rem', textAlign: 'center', color: '#6b7280' }}>Policy tester - coming soon</div>` : null}
						${activeTab === 'audit' ? html`<div style=${{ padding: '2rem', textAlign: 'center', color: '#6b7280' }}>Audit log - coming soon</div>` : null}
					</div>
				`}
			</div>

			${showDeleteRoleConfirm ? html`
				<${ConfirmDialog}
					title="Delete Role"
					message="Are you sure you want to delete role ${roleToDelete?.displayName || roleToDelete?.name}? This action cannot be undone."
					confirmText="Delete"
					variant="danger"
					onConfirm=${confirmDeleteRole}
					onCancel=${() => setShowDeleteRoleConfirm(false)}
				/>
			` : null}

			${showDeletePolicyConfirm ? html`
				<${ConfirmDialog}
					title="Delete Policy"
					message="Are you sure you want to delete the policy for ${policyToDelete?.subject}?"
					confirmText="Delete"
					variant="danger"
					onConfirm=${confirmDeletePolicy}
					onCancel=${() => setShowDeletePolicyConfirm(false)}
				/>
			` : null}
		<//>
	`;

	function renderRolesTab() {
		return html`
			<div>
				<div style=${{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
					<h3 style=${{ margin: 0 }}>Roles (${roles.length})</h3>
					<button class="btn btn-primary" onClick=${() => { setEditingRole({ name: '', description: '' }); setShowRoleModal(true); }} type="button">Add Role</button>
				</div>
				<div class="table-container">
					<table class="table">
						<thead><tr><th>Name</th><th>Description</th><th>Type</th><th>Actions</th></tr></thead>
						<tbody>
							${roles.map(role => html`
								<tr key=${role.name}>
									<td><strong>${role.displayName || role.name}</strong></td>
									<td>${role.description}</td>
									<td><span class="badge badge-primary">${role.type || 'custom'}</span></td>
									<td>
										<button class="btn btn-sm btn-danger" onClick=${() => { setRoleToDelete(role); setShowDeleteRoleConfirm(true); }} type="button">Delete</button>
									</td>
								</tr>
							`)}
							${roles.length === 0 ? html`<tr><td colspan="4" style=${{ textAlign: 'center', color: '#6b7280', padding: '2rem' }}>No roles defined</td></tr>` : null}
						</tbody>
					</table>
				</div>
			</div>

			${showRoleModal ? html`
				<div class="modal-overlay" onClick=${() => setShowRoleModal(false)}>
					<div class="modal-content" onClick=${(e: Event) => e.stopPropagation()}>
						<h3 style=${{ marginBottom: '1rem' }}>Create Role</h3>
						<div class="form-group">
							<label class="form-label">Name</label>
							<input class="form-input" value=${editingRole.name} onInput=${(e: Event) => setEditingRole({ ...editingRole, name: (e.target as HTMLInputElement).value })} placeholder="role_name" />
						</div>
						<div class="form-group">
							<label class="form-label">Description</label>
							<input class="form-input" value=${editingRole.description} onInput=${(e: Event) => setEditingRole({ ...editingRole, description: (e.target as HTMLInputElement).value })} placeholder="Role description" />
						</div>
						<div class="form-actions">
							<button class="btn btn-secondary" onClick=${() => setShowRoleModal(false)} type="button">Cancel</button>
							<button class="btn btn-primary" onClick=${createRole} type="button">Create</button>
						</div>
					</div>
				</div>
			` : null}
		`;
	}

	function renderPoliciesTab() {
		return html`
			<div>
				<div style=${{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
					<h3 style=${{ margin: 0 }}>Policies (${policies.length})</h3>
					<button class="btn btn-primary" onClick=${() => { setEditingPolicy({ subject: '', resource: '', action: '', effect: 'allow' }); setShowPolicyModal(true); }} type="button">Add Policy</button>
				</div>
				<div class="table-container">
					<table class="table">
						<thead><tr><th>Subject</th><th>Resource</th><th>Action</th><th>Effect</th><th>Actions</th></tr></thead>
						<tbody>
							${policies.map(policy => html`
								<tr key=${policy.id}>
									<td>${policy.subject}</td>
									<td>${policy.resource}</td>
									<td>${policy.action}</td>
									<td><span class="badge ${policy.effect === 'allow' ? 'badge-success' : 'badge-danger'}">${policy.effect}</span></td>
									<td>
										<button class="btn btn-sm btn-danger" onClick=${() => { setPolicyToDelete(policy); setShowDeletePolicyConfirm(true); }} type="button">Delete</button>
									</td>
								</tr>
							`)}
							${policies.length === 0 ? html`<tr><td colspan="5" style=${{ textAlign: 'center', color: '#6b7280', padding: '2rem' }}>No policies defined</td></tr>` : null}
						</tbody>
					</table>
				</div>
			</div>

			${showPolicyModal ? html`
				<div class="modal-overlay" onClick=${() => setShowPolicyModal(false)}>
					<div class="modal-content" onClick=${(e: Event) => e.stopPropagation()}>
						<h3 style=${{ marginBottom: '1rem' }}>Create Policy</h3>
						<div class="form-group">
							<label class="form-label">Subject (role)</label>
							<input class="form-input" value=${editingPolicy.subject} onInput=${(e: Event) => setEditingPolicy({ ...editingPolicy, subject: (e.target as HTMLInputElement).value })} placeholder="admin" />
						</div>
						<div class="form-group">
							<label class="form-label">Resource</label>
							<input class="form-input" value=${editingPolicy.resource} onInput=${(e: Event) => setEditingPolicy({ ...editingPolicy, resource: (e.target as HTMLInputElement).value })} placeholder="/api/users/*" />
						</div>
						<div class="form-group">
							<label class="form-label">Action</label>
							<input class="form-input" value=${editingPolicy.action} onInput=${(e: Event) => setEditingPolicy({ ...editingPolicy, action: (e.target as HTMLInputElement).value })} placeholder="read, write, delete" />
						</div>
						<div class="form-group">
							<label class="form-label">Effect</label>
							<select class="form-select" value=${editingPolicy.effect} onChange=${(e: Event) => setEditingPolicy({ ...editingPolicy, effect: (e.target as HTMLSelectElement).value as 'allow' | 'deny' })}>
								<option value="allow">Allow</option>
								<option value="deny">Deny</option>
							</select>
						</div>
						<div class="form-actions">
							<button class="btn btn-secondary" onClick=${() => setShowPolicyModal(false)} type="button">Cancel</button>
							<button class="btn btn-primary" onClick=${createPolicy} type="button">Create</button>
						</div>
					</div>
				</div>
			` : null}
		`;
	}

	function renderUsersTab() {
		return html`
			<div>
				<h3 style=${{ marginBottom: '1rem' }}>User Role Assignments (${users.length})</h3>
				<div class="table-container">
					<table class="table">
						<thead><tr><th>Email</th><th>Roles</th></tr></thead>
						<tbody>
							${users.map(user => html`
								<tr key=${user.id}>
									<td>${user.email}</td>
									<td>${(user.roles || []).map(r => html`<span class="badge badge-primary" style=${{ marginRight: '0.25rem' }}>${r.displayName || r.name}</span>`)}</td>
								</tr>
							`)}
							${users.length === 0 ? html`<tr><td colspan="2" style=${{ textAlign: 'center', color: '#6b7280', padding: '2rem' }}>No users found</td></tr>` : null}
						</tbody>
					</table>
				</div>
			</div>
		`;
	}
}
