import { html, FeatureShell, PageHeader, TabNavigation, DataTable, SearchInput, Button, Modal, LoadingSpinner, api } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { ShoppingBag, Tag, DollarSign, Package, Plus, CreditCard } from 'lucide-preact';

function ProductsTab() {
	const [products, setProducts] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);
	const [search, setSearch] = useState('');

	useEffect(() => {
		api.get('/admin/ext/products/products').then((data: any) => {
			const records = Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : [];
			setProducts(records);
			setLoading(false);
		}).catch(() => setLoading(false));
	}, []);

	const filtered = search
		? products.filter(p => p.name?.toLowerCase().includes(search.toLowerCase()))
		: products;

	const columns = [
		{ key: 'name', label: 'Product', sortable: true },
		{ key: 'type', label: 'Type', sortable: true },
		{ key: 'status', label: 'Status', render: (v: string) => html`
			<span style=${{
				fontSize: '0.75rem',
				padding: '0.125rem 0.5rem',
				borderRadius: '9999px',
				background: v === 'active' ? '#dcfce7' : '#f3f4f6',
				color: v === 'active' ? '#166534' : '#6b7280'
			}}>${v || 'draft'}</span>
		` },
		{ key: 'created_at', label: 'Created', sortable: true, render: (v: string) => v ? new Date(v).toLocaleDateString() : '-' },
	];

	if (loading) return html`<${LoadingSpinner} message="Loading products..." />`;

	return html`
		<div>
			<${PageHeader} title="Products" description="Manage your product catalog" />
			<${SearchInput} value=${search} onChange=${setSearch} placeholder="Search products..." />
			<${DataTable} columns=${columns} data=${filtered} emptyMessage="No products yet" />
		</div>
	`;
}

function PricingTab() {
	const [templates, setTemplates] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		api.get('/admin/ext/products/pricing').then((data: any) => {
			const records = Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : [];
			setTemplates(records);
			setLoading(false);
		}).catch(() => setLoading(false));
	}, []);

	const columns = [
		{ key: 'name', label: 'Template Name', sortable: true },
		{ key: 'formula', label: 'Formula' },
		{ key: 'created_at', label: 'Created', sortable: true, render: (v: string) => v ? new Date(v).toLocaleDateString() : '-' },
	];

	if (loading) return html`<${LoadingSpinner} message="Loading pricing templates..." />`;

	return html`
		<div>
			<${PageHeader} title="Pricing Templates" description="Define pricing formulas and variables" />
			<${DataTable} columns=${columns} data=${templates} emptyMessage="No pricing templates defined" />
		</div>
	`;
}

function PurchasesTab() {
	const [purchases, setPurchases] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		api.get('/admin/ext/products/purchases').then((data: any) => {
			const records = Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : [];
			setPurchases(records);
			setLoading(false);
		}).catch(() => setLoading(false));
	}, []);

	const columns = [
		{ key: 'id', label: 'Order ID', width: '120px' },
		{ key: 'user_id', label: 'User' },
		{ key: 'status', label: 'Status', render: (v: string) => html`
			<span style=${{
				fontSize: '0.75rem',
				padding: '0.125rem 0.5rem',
				borderRadius: '9999px',
				background: v === 'completed' ? '#dcfce7' : v === 'pending' ? '#fefce8' : '#f3f4f6',
				color: v === 'completed' ? '#166534' : v === 'pending' ? '#854d0e' : '#6b7280'
			}}>${v || 'unknown'}</span>
		` },
		{ key: 'total', label: 'Total', render: (v: any) => v != null ? `$${Number(v).toFixed(2)}` : '-' },
		{ key: 'created_at', label: 'Date', sortable: true, render: (v: string) => v ? new Date(v).toLocaleDateString() : '-' },
	];

	if (loading) return html`<${LoadingSpinner} message="Loading purchases..." />`;

	return html`
		<div>
			<${PageHeader} title="Purchases" description="View and manage orders" />
			<${DataTable} columns=${columns} data=${purchases} emptyMessage="No purchases yet" />
		</div>
	`;
}

function SubscriptionsTab() {
	const [subscriptions, setSubscriptions] = useState<any[]>([]);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		api.get('/admin/ext/products/purchases?status=completed').then((data: any) => {
			const records = Array.isArray(data?.records) ? data.records : Array.isArray(data) ? data : [];
			setSubscriptions(records);
			setLoading(false);
		}).catch(() => setLoading(false));
	}, []);

	const columns = [
		{ key: 'id', label: 'Purchase ID', width: '120px' },
		{ key: 'user_id', label: 'User ID', sortable: true },
		{ key: 'status', label: 'Status', render: (v: string) => html`
			<span style=${{
				fontSize: '0.75rem',
				padding: '0.125rem 0.5rem',
				borderRadius: '9999px',
				background: v === 'completed' ? '#dcfce7' : v === 'active' ? '#dbeafe' : '#f3f4f6',
				color: v === 'completed' ? '#166534' : v === 'active' ? '#1e40af' : '#6b7280'
			}}>${v || 'unknown'}</span>
		` },
		{ key: 'total', label: 'Amount', render: (v: any) => v != null ? `$${Number(v).toFixed(2)}` : '-' },
		{ key: 'created_at', label: 'Created', sortable: true, render: (v: string) => v ? new Date(v).toLocaleDateString() : '-' },
	];

	if (loading) return html`<${LoadingSpinner} message="Loading subscriptions..." />`;

	return html`
		<div>
			<${PageHeader} title="Subscriptions" description="Completed purchases across all users" />
			<${DataTable} columns=${columns} data=${subscriptions} emptyMessage="No subscriptions yet" />
		</div>
	`;
}

export function App() {
	const [tab, setTab] = useState('products');

	const tabs = [
		{ id: 'products', label: 'Products', icon: ShoppingBag },
		{ id: 'pricing', label: 'Pricing', icon: DollarSign },
		{ id: 'purchases', label: 'Purchases', icon: Package },
		{ id: 'subscriptions', label: 'Subscriptions', icon: CreditCard },
	];

	return html`
		<${FeatureShell} title="Products">
			<${TabNavigation} tabs=${tabs} activeTab=${tab} onTabChange=${setTab} />
			${tab === 'products' ? html`<${ProductsTab} />` : null}
			${tab === 'pricing' ? html`<${PricingTab} />` : null}
			${tab === 'purchases' ? html`<${PurchasesTab} />` : null}
			${tab === 'subscriptions' ? html`<${SubscriptionsTab} />` : null}
		<//>
	`;
}
