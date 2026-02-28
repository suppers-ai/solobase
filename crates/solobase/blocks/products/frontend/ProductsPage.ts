import { html } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { PageHeader, TabNavigation, LoadingSpinner, authFetch } from '@solobase/ui';
import { ShoppingBag, Layers, DollarSign, Variable, Receipt } from 'lucide-preact';
import { ProductsTab } from './tabs/ProductsTab';
import { GroupsTab } from './tabs/GroupsTab';
import { PricingTab } from './tabs/PricingTab';
import { VariablesTab } from './tabs/VariablesTab';
import { PurchasesTab } from './tabs/PurchasesTab';

const tabs = [
	{ id: 'products', label: 'Products', icon: ShoppingBag },
	{ id: 'groups', label: 'Groups', icon: Layers },
	{ id: 'pricing', label: 'Pricing', icon: DollarSign },
	{ id: 'variables', label: 'Variables', icon: Variable },
	{ id: 'purchases', label: 'Purchases', icon: Receipt },
];

export function ProductsPage() {
	const [activeTab, setActiveTab] = useState('products');
	const [loading, setLoading] = useState(true);
	const [stats, setStats] = useState<any>(null);

	useEffect(() => { loadStats(); }, []);

	async function loadStats() {
		setLoading(false);
		try {
			const response = await authFetch('/api/admin/ext/products/stats');
			if (response.ok) setStats(await response.json());
		} catch { /* ignore */ }
	}

	const description = stats
		? `${stats.totalProducts ?? 0} products \u2022 ${stats.totalGroups ?? 0} groups \u2022 ${stats.totalPurchases ?? 0} purchases`
		: 'Manage products, pricing, and purchases';

	return html`
		<div class="products-page">
			<${PageHeader} title="Products" description=${description} />
			<div class="content-area">
				<${TabNavigation} tabs=${tabs} activeTab=${activeTab} onTabChange=${setActiveTab} />
				<div style=${{ marginTop: '1rem' }}>
					${activeTab === 'products' ? html`<${ProductsTab} />` : null}
					${activeTab === 'groups' ? html`<${GroupsTab} />` : null}
					${activeTab === 'pricing' ? html`<${PricingTab} />` : null}
					${activeTab === 'variables' ? html`<${VariablesTab} />` : null}
					${activeTab === 'purchases' ? html`<${PurchasesTab} />` : null}
				</div>
			</div>
		</div>
	`;
}
