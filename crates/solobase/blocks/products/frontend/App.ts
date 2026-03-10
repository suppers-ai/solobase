import { html, FeatureShell, TabNavigation, StatCard, PageHeader, LoadingSpinner, api } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { ShoppingBag, Layers, DollarSign, Variable, Receipt, BarChart3, Package, CreditCard } from 'lucide-preact';
import { ProductsTab } from './tabs/ProductsTab';
import { GroupsTab } from './tabs/GroupsTab';
import { PricingTab } from './tabs/PricingTab';
import { VariablesTab } from './tabs/VariablesTab';
import { PurchasesTab } from './tabs/PurchasesTab';

function OverviewTab() {
	const [stats, setStats] = useState<any>(null);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		api.get('/admin/b/products/stats').then((data: any) => {
			setStats(data);
			setLoading(false);
		}).catch(() => setLoading(false));
	}, []);

	if (loading) return html`<${LoadingSpinner} message="Loading stats..." />`;

	const revenue = typeof stats?.total_revenue === 'number' ? `$${stats.total_revenue.toFixed(2)}` : '$0.00';

	return html`
		<div>
			<${PageHeader} title="Overview" description="Product and sales metrics" />
			<div style=${{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))', gap: '1rem' }}>
				<${StatCard} title="Total Products" value=${stats?.total_products ?? 0} icon=${Package} />
				<${StatCard} title="Active Products" value=${stats?.active_products ?? 0} icon=${ShoppingBag} />
				<${StatCard} title="Groups" value=${stats?.total_groups ?? 0} icon=${Layers} />
				<${StatCard} title="Total Purchases" value=${stats?.total_purchases ?? 0} icon=${Receipt} />
				<${StatCard} title="Revenue" value=${revenue} icon=${DollarSign} />
			</div>
		</div>
	`;
}

export function App() {
	const [tab, setTab] = useState(() => window.location.hash.slice(1) || 'overview');

	useEffect(() => { window.location.hash = tab; }, [tab]);
	useEffect(() => {
		function onHash() { setTab(window.location.hash.slice(1) || 'overview'); }
		window.addEventListener('hashchange', onHash);
		return () => window.removeEventListener('hashchange', onHash);
	}, []);

	const tabs = [
		{ id: 'overview', label: 'Overview', icon: BarChart3 },
		{ id: 'products', label: 'Products', icon: ShoppingBag },
		{ id: 'groups', label: 'Groups', icon: Layers },
		{ id: 'pricing', label: 'Pricing', icon: DollarSign },
		{ id: 'variables', label: 'Variables', icon: Variable },
		{ id: 'purchases', label: 'Purchases', icon: CreditCard },
	];

	return html`
		<${FeatureShell} title="Products">
			<${TabNavigation} tabs=${tabs} activeTab=${tab} onTabChange=${setTab} />
			${tab === 'overview' ? html`<${OverviewTab} />` : null}
			${tab === 'products' ? html`<${ProductsTab} />` : null}
			${tab === 'groups' ? html`<${GroupsTab} />` : null}
			${tab === 'pricing' ? html`<${PricingTab} />` : null}
			${tab === 'variables' ? html`<${VariablesTab} />` : null}
			${tab === 'purchases' ? html`<${PurchasesTab} />` : null}
		<//>
	`;
}
