import { html } from '../../htm';

interface Tab {
	id: string;
	label: string;
	icon?: any;
	badge?: number;
}

interface TabNavigationProps {
	tabs: Tab[];
	activeTab: string;
	onTabChange: (tabId: string) => void;
}

export function TabNavigation({ tabs, activeTab, onTabChange }: TabNavigationProps) {
	return html`
		<div style=${{
			display: 'flex',
			gap: '0'
		}}>
			${tabs.map(tab => html`
				<button
					key=${tab.id}
					onClick=${() => onTabChange(tab.id)}
					style=${{
						padding: '0.75rem 1rem',
						background: 'none',
						border: 'none',
						borderBottom: `2px solid ${activeTab === tab.id ? 'var(--primary-color, #fe6627)' : 'transparent'}`,
						marginBottom: '-2px',
						color: activeTab === tab.id ? 'var(--primary-color, #fe6627)' : 'var(--text-secondary, #64748b)',
						fontWeight: 400,
						fontSize: '0.875rem',
						cursor: 'pointer',
						display: 'flex',
						alignItems: 'center',
						gap: '0.5rem',
						transition: 'all 0.2s'
					}}
					type="button"
				>
					${tab.icon ? html`<${tab.icon} size=${16} />` : null}
					${tab.label}
					${tab.badge != null ? html`
						<span style=${{
							background: 'var(--primary-color, #fe6627)',
							color: 'white',
							fontSize: '0.688rem',
							padding: '0 0.375rem',
							borderRadius: '9999px',
							minWidth: '18px',
							textAlign: 'center'
						}}>${tab.badge}</span>
					` : null}
				</button>
			`)}
		</div>
	`;
}
