import { html } from '@solobase/ui';
import { useState, useEffect, useCallback } from 'preact/hooks';
import { api, PageHeader, LoadingSpinner } from '@solobase/ui';
import { BlocksTab } from './tabs/BlocksTab';
import { ChainsTab } from './tabs/ChainsTab';
import { SettingsTab } from './tabs/SettingsTab';

interface AdminUIInfo {
	path: string;
	icon: string;
	title: string;
}

interface BlockInfo {
	name: string;
	version: string;
	interface: string;
	summary: string;
	instance_mode: string;
	allowed_modes: string[];
	admin_ui?: AdminUIInfo;
}

interface ChainDef {
	id: string;
	summary?: string;
	config?: { on_error: string; timeout?: string };
	http?: { routes: any[] };
	root?: any;
}

function getInitialTab(): string {
	const hash = window.location.hash.replace('#', '');
	if (['blocks', 'chains', 'settings'].includes(hash)) return hash;
	return 'blocks';
}

export function WafflePage() {
	const [activeTab, setActiveTab] = useState(getInitialTab);
	const [blocks, setBlocks] = useState<BlockInfo[]>([]);
	const [chains, setChains] = useState<ChainDef[]>([]);
	const [loading, setLoading] = useState(true);

	const loadData = useCallback(async () => {
		setLoading(true);
		try {
			const [blocksRes, chainsRes] = await Promise.all([
				api.get<BlockInfo[]>('/admin/waffle/blocks'),
				api.get('/admin/waffle/chains'),
			]);
			setBlocks(blocksRes || []);
			setChains(chainsRes || []);
		} catch (err) {
			console.error('Failed to load data:', err);
		} finally {
			setLoading(false);
		}
	}, []);

	useEffect(() => { loadData(); }, [loadData]);

	// Sync tab with hash changes (sidebar navigation, back/forward)
	useEffect(() => {
		function onHashChange() {
			const hash = window.location.hash.replace('#', '');
			if (['blocks', 'chains', 'settings'].includes(hash)) {
				setActiveTab(hash);
			}
		}
		window.addEventListener('hashchange', onHashChange);
		return () => window.removeEventListener('hashchange', onHashChange);
	}, []);

	const pageInfo = activeTab === 'chains'
		? { title: 'Chains', description: 'Manage block chains and workflows' }
		: activeTab === 'settings'
		? { title: 'Settings', description: 'System settings and configuration' }
		: { title: 'Blocks', description: 'Manage installed blocks' };

	return html`
		<>
			<${PageHeader}
				title=${pageInfo.title}
				description=${pageInfo.description}
			/>

			${loading ? html`<${LoadingSpinner} />` :
				activeTab === 'blocks' ? html`<${BlocksTab} blocks=${blocks} chains=${chains} />` :
				activeTab === 'chains' ? html`<${ChainsTab} chains=${chains} onReload=${loadData} />` :
				html`<${SettingsTab} blocks=${blocks} chains=${chains} />`
			}
		<//>
	`;
}
