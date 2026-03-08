import { html, BlockShell, PageHeader, TabNavigation, DataTable, SearchInput, Button, Modal, LoadingSpinner, api } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { Shield, Users, Key } from 'lucide-preact';

function RolesTab() {
	const [roles, setRoles] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		api.get('/admin/iam/roles').then((data: any) => {
			setRoles(Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : []);
			setLoading(false);
		}).catch(() => setLoading(false));
	}, []);

	const columns = [
		{ key: 'name', label: 'Role Name', sortable: true },
		{ key: 'description', label: 'Description' },
		{ key: 'created_at', label: 'Created', sortable: true, render: (v: string) => v ? new Date(v).toLocaleDateString() : '-' },
	];

	if (loading) return html`<${LoadingSpinner} message="Loading roles..." />`;

	return html`
		<div>
			<${PageHeader} title="Roles" description="Manage access control roles" />
			<${DataTable} columns=${columns} data=${roles} emptyMessage="No roles defined" />
		</div>
	`;
}

function PermissionsTab() {
	const [permissions, setPermissions] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		api.get('/admin/iam/permissions').then((data: any) => {
			setPermissions(Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : []);
			setLoading(false);
		}).catch(() => setLoading(false));
	}, []);

	const columns = [
		{ key: 'name', label: 'Permission', sortable: true },
		{ key: 'resource', label: 'Resource' },
		{ key: 'action', label: 'Action' },
		{ key: 'description', label: 'Description' },
	];

	if (loading) return html`<${LoadingSpinner} message="Loading permissions..." />`;

	return html`
		<div>
			<${PageHeader} title="Permissions" description="Manage granular permissions" />
			<${DataTable} columns=${columns} data=${permissions} emptyMessage="No permissions defined" />
		</div>
	`;
}

function ApiKeysTab() {
	const [keys, setKeys] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		api.get('/admin/iam/api-keys').then((data: any) => {
			setKeys(Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : []);
			setLoading(false);
		}).catch(() => setLoading(false));
	}, []);

	const columns = [
		{ key: 'name', label: 'Name', sortable: true },
		{ key: 'key_prefix', label: 'Key Prefix' },
		{ key: 'user_id', label: 'User' },
		{ key: 'created_at', label: 'Created', sortable: true, render: (v: string) => v ? new Date(v).toLocaleDateString() : '-' },
	];

	if (loading) return html`<${LoadingSpinner} message="Loading API keys..." />`;

	return html`
		<div>
			<${PageHeader} title="API Keys" description="Manage API keys across users" />
			<${DataTable} columns=${columns} data=${keys} emptyMessage="No API keys found" />
		</div>
	`;
}

export function App() {
	const [tab, setTab] = useState('roles');

	const tabs = [
		{ id: 'roles', label: 'Roles', icon: Shield },
		{ id: 'permissions', label: 'Permissions', icon: Key },
		{ id: 'api-keys', label: 'API Keys', icon: Key },
	];

	return html`
		<${BlockShell} title="IAM">
			<${TabNavigation} tabs=${tabs} activeTab=${tab} onTabChange=${setTab} />
			${tab === 'roles' ? html`<${RolesTab} />` : null}
			${tab === 'permissions' ? html`<${PermissionsTab} />` : null}
			${tab === 'api-keys' ? html`<${ApiKeysTab} />` : null}
		<//>
	`;
}
