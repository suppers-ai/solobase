import { html } from '@solobase/ui';
import { Package, GitBranch, Layers, Cpu } from 'lucide-preact';
import { StatCard } from '@solobase/ui';

interface BlockInfo {
	name: string;
	version: string;
	interface: string;
	instance_mode: string;
	allowed_modes: string[];
}

interface ChainDef {
	id: string;
	summary?: string;
}

interface SettingsTabProps {
	blocks: BlockInfo[];
	chains: ChainDef[];
}

export function SettingsTab({ blocks, chains }: SettingsTabProps) {
	const interfaces = [...new Set(blocks.map(b => b.interface))];
	const instanceModes = [...new Set(blocks.map(b => b.instance_mode))];

	return html`
		<div>
			<!-- Summary stats -->
			<div style=${{
				display: 'grid',
				gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
				gap: '1rem',
				marginBottom: '2rem'
			}}>
				<${StatCard} title="Registered Blocks" value=${blocks.length} icon=${Package} />
				<${StatCard} title="Active Chains" value=${chains.length} icon=${GitBranch} color="#8b5cf6" />
			</div>

			<!-- Registered Interfaces -->
			<div style=${{
				background: 'var(--card-bg, white)',
				border: '1px solid var(--border-color, #e2e8f0)',
				borderRadius: '12px',
				padding: '1.25rem',
				marginBottom: '1.5rem'
			}}>
				<div style=${{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '1rem' }}>
					<${Layers} size=${18} style=${{ color: 'var(--primary-color, #189AB4)' }} />
					<h3 style=${{ margin: 0, fontSize: '1rem', fontWeight: 600, color: 'var(--text-primary, #1e293b)' }}>Registered Interfaces</h3>
				</div>
				${interfaces.length === 0 ? html`
					<p style=${{ fontSize: '0.875rem', color: 'var(--text-secondary, #64748b)', margin: 0 }}>No interfaces registered.</p>
				` : html`
					<div style=${{ display: 'flex', flexWrap: 'wrap', gap: '0.5rem' }}>
						${interfaces.map((iface: string) => {
							const count = blocks.filter(b => b.interface === iface).length;
							return html`
								<div key=${iface} style=${{
									display: 'flex',
									alignItems: 'center',
									gap: '0.5rem',
									padding: '0.5rem 0.75rem',
									background: 'var(--bg-muted, #f8fafc)',
									border: '1px solid var(--border-color, #e2e8f0)',
									borderRadius: '8px',
									fontSize: '0.813rem'
								}}>
									<span style=${{ fontWeight: 500, color: 'var(--text-primary, #1e293b)' }}>${iface}</span>
									<span style=${{
										fontSize: '0.688rem',
										fontWeight: 600,
										padding: '0.0625rem 0.375rem',
										borderRadius: '9999px',
										background: 'var(--primary-color, #189AB4)',
										color: 'white'
									}}>${count}</span>
								</div>
							`;
						})}
					</div>
				`}
			</div>

			<!-- Instance Modes -->
			<div style=${{
				background: 'var(--card-bg, white)',
				border: '1px solid var(--border-color, #e2e8f0)',
				borderRadius: '12px',
				padding: '1.25rem'
			}}>
				<div style=${{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '1rem' }}>
					<${Cpu} size=${18} style=${{ color: '#f59e0b' }} />
					<h3 style=${{ margin: 0, fontSize: '1rem', fontWeight: 600, color: 'var(--text-primary, #1e293b)' }}>Instance Modes in Use</h3>
				</div>
				${instanceModes.length === 0 ? html`
					<p style=${{ fontSize: '0.875rem', color: 'var(--text-secondary, #64748b)', margin: 0 }}>No instance modes in use.</p>
				` : html`
					<div style=${{ display: 'flex', flexWrap: 'wrap', gap: '0.5rem' }}>
						${instanceModes.map((mode: string) => {
							const count = blocks.filter(b => b.instance_mode === mode).length;
							return html`
								<div key=${mode} style=${{
									display: 'flex',
									alignItems: 'center',
									gap: '0.5rem',
									padding: '0.5rem 0.75rem',
									background: 'var(--bg-muted, #f8fafc)',
									border: '1px solid var(--border-color, #e2e8f0)',
									borderRadius: '8px',
									fontSize: '0.813rem'
								}}>
									<code style=${{ fontWeight: 500, color: 'var(--text-primary, #1e293b)' }}>${mode}</code>
									<span style=${{
										fontSize: '0.688rem',
										fontWeight: 600,
										padding: '0.0625rem 0.375rem',
										borderRadius: '9999px',
										background: '#f59e0b',
										color: 'white'
									}}>${count}</span>
								</div>
							`;
						})}
					</div>
				`}
			</div>
		</div>
	`;
}
