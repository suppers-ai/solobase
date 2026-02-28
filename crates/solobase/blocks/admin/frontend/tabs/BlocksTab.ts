import { html } from '@solobase/ui';
import { useState } from 'preact/hooks';
import { Package, ExternalLink, ChevronDown, ChevronRight, GitBranch } from 'lucide-preact';
import { SearchInput, EmptyState } from '@solobase/ui';

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

interface BlocksTabProps {
	blocks: BlockInfo[];
	chains: ChainDef[];
}

const INTERFACE_COLORS: Record<string, { bg: string; color: string }> = {
	'MessageHandler': { bg: '#dbeafe', color: '#1e40af' },
	'DataStore': { bg: '#dcfce7', color: '#166534' },
	'AuthProvider': { bg: '#fef3c7', color: '#92400e' },
	'Notifier': { bg: '#fce7f3', color: '#9d174d' },
};

function getInterfaceStyle(iface: string) {
	return INTERFACE_COLORS[iface] || { bg: '#f1f5f9', color: '#475569' };
}

function getModeColor(mode: string) {
	switch (mode) {
		case 'singleton': return { bg: '#dbeafe', color: '#1e40af' };
		case 'per-request': return { bg: '#dcfce7', color: '#166534' };
		case 'pool': return { bg: '#fef3c7', color: '#92400e' };
		default: return { bg: '#f1f5f9', color: '#475569' };
	}
}

/** Walk a chain node tree and collect all block names referenced. */
function collectBlockNames(node: any, names: Set<string>) {
	if (!node) return;
	if (node.block) names.add(node.block);
	if (node.next) collectBlockNames(node.next, names);
	if (node.branches) {
		for (const branch of Object.values(node.branches) as any[]) {
			collectBlockNames(branch, names);
		}
	}
}

/** Return all chains that reference a given block name. */
function getChainsForBlock(blockName: string, chains: ChainDef[]): ChainDef[] {
	return chains.filter(c => {
		const names = new Set<string>();
		collectBlockNames(c.root, names);
		return names.has(blockName);
	});
}

export function BlocksTab({ blocks, chains }: BlocksTabProps) {
	const [searchQuery, setSearchQuery] = useState('');
	const [selectedBlock, setSelectedBlock] = useState<BlockInfo | null>(null);
	const [jsonExpanded, setJsonExpanded] = useState(true);

	const q = searchQuery.toLowerCase();
	const filtered = blocks.filter(b =>
		!q ||
		b.name.toLowerCase().includes(q) ||
		b.interface.toLowerCase().includes(q) ||
		b.summary.toLowerCase().includes(q)
	);

	const usedInChains = selectedBlock ? getChainsForBlock(selectedBlock.name, chains) : [];

	return html`
		<div style=${{ display: 'flex', gap: '1rem', minHeight: '500px' }}>
			<!-- Left panel: block list -->
			<div style=${{
				width: '300px',
				minWidth: '300px',
				display: 'flex',
				flexDirection: 'column',
				gap: '0.75rem'
			}}>
				<${SearchInput}
					value=${searchQuery}
					onChange=${setSearchQuery}
					placeholder="Search blocks..."
				/>

				<div style=${{ display: 'flex', flexDirection: 'column', gap: '0.5rem', overflowY: 'auto', flex: 1 }}>
					${filtered.length === 0 ? html`
						<div style=${{
							padding: '2rem 1rem',
							textAlign: 'center',
							color: 'var(--text-secondary, #64748b)',
							fontSize: '0.813rem'
						}}>
							${searchQuery ? 'No blocks match your search.' : 'No blocks registered.'}
						</div>
					` : filtered.map((b: BlockInfo) => {
						const ifaceStyle = getInterfaceStyle(b.interface);
						const modeStyle = getModeColor(b.instance_mode);
						const isSelected = selectedBlock?.name === b.name;
						return html`
							<div key=${b.name}
								onClick=${() => setSelectedBlock(b)}
								style=${{
									background: isSelected ? 'var(--bg-muted, #f1f5f9)' : 'var(--card-bg, white)',
									border: isSelected
										? '1px solid var(--primary-color, #189AB4)'
										: '1px solid var(--border-color, #e2e8f0)',
									borderRadius: '10px',
									padding: '0.875rem',
									transition: 'border-color 0.2s, box-shadow 0.2s',
									cursor: 'pointer'
								}} onMouseEnter=${(e: any) => {
									if (!isSelected) {
										e.currentTarget.style.borderColor = 'var(--primary-color, #189AB4)';
										e.currentTarget.style.boxShadow = '0 2px 8px rgba(0,0,0,0.06)';
									}
								}} onMouseLeave=${(e: any) => {
									if (!isSelected) {
										e.currentTarget.style.borderColor = 'var(--border-color, #e2e8f0)';
										e.currentTarget.style.boxShadow = 'none';
									}
								}}>
								<div style=${{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '0.375rem' }}>
									<code style=${{ fontSize: '0.85rem', fontWeight: 600, color: 'var(--text-primary, #1e293b)' }}>${b.name}</code>
									${b.admin_ui ? html`
										<${ExternalLink} size=${12} style=${{ color: 'var(--primary-color, #189AB4)', flexShrink: 0 }} />
									` : null}
								</div>

								${b.summary ? html`
									<p style=${{
										fontSize: '0.75rem',
										color: 'var(--text-secondary, #64748b)',
										margin: '0 0 0.5rem 0',
										lineHeight: 1.4
									}}>${b.summary}</p>
								` : null}

								<div style=${{ display: 'flex', alignItems: 'center', gap: '0.375rem', flexWrap: 'wrap' }}>
									<span style=${{
										display: 'inline-flex',
										alignItems: 'center',
										padding: '0.0625rem 0.5rem',
										fontSize: '0.688rem',
										fontWeight: 500,
										borderRadius: '9999px',
										backgroundColor: ifaceStyle.bg,
										color: ifaceStyle.color
									}}>${b.interface}</span>
									<span style=${{
										fontSize: '0.625rem',
										fontWeight: 500,
										padding: '0.0625rem 0.375rem',
										borderRadius: '4px',
										backgroundColor: modeStyle.bg,
										color: modeStyle.color
									}}>${b.instance_mode}</span>
								</div>
							</div>
						`;
					})}
				</div>
			</div>

			<!-- Right panel: block detail -->
			<div style=${{
				flex: 1,
				display: 'flex',
				flexDirection: 'column',
				background: 'var(--bg-muted, #f8fafc)',
				border: selectedBlock ? '1px solid var(--border-color, #e2e8f0)' : '2px dashed var(--border-color, #e2e8f0)',
				borderRadius: '12px',
				minHeight: '400px',
				overflow: 'hidden'
			}}>
				${selectedBlock ? html`
					<!-- Detail header -->
					<div style=${{
						display: 'flex',
						justifyContent: 'space-between',
						alignItems: 'center',
						padding: '0.75rem 1rem',
						borderBottom: '1px solid var(--border-color, #e2e8f0)',
						background: 'var(--card-bg, white)'
					}}>
						<div>
							<code style=${{ fontSize: '0.875rem', fontWeight: 600 }}>${selectedBlock.name}</code>
							<span style=${{ fontSize: '0.75rem', color: 'var(--text-secondary, #64748b)', marginLeft: '0.5rem' }}>
								v${selectedBlock.version}
							</span>
							${selectedBlock.summary ? html`
								<span style=${{ fontSize: '0.75rem', color: 'var(--text-secondary, #64748b)', marginLeft: '0.5rem' }}>
									— ${selectedBlock.summary}
								</span>
							` : null}
						</div>
						${selectedBlock.admin_ui ? html`
							<a href=${selectedBlock.admin_ui.path}
								style=${{
									display: 'inline-flex',
									alignItems: 'center',
									gap: '0.375rem',
									padding: '0.375rem 0.75rem',
									fontSize: '0.75rem',
									fontWeight: 500,
									color: 'var(--primary-color, #189AB4)',
									border: '1px solid var(--primary-color, #189AB4)',
									borderRadius: '6px',
									textDecoration: 'none',
									transition: 'background 0.2s'
								}}
								onMouseEnter=${(e: any) => { e.currentTarget.style.background = 'rgba(24,154,180,0.08)'; }}
								onMouseLeave=${(e: any) => { e.currentTarget.style.background = 'transparent'; }}
							>
								<${ExternalLink} size=${12} />
								Admin UI
							</a>
						` : null}
					</div>

					<!-- Detail body -->
					<div style=${{ flex: 1, padding: '1.25rem', overflowY: 'auto' }}>
						<!-- Tags row -->
						<div style=${{ display: 'flex', alignItems: 'center', gap: '0.5rem', flexWrap: 'wrap', marginBottom: '1.25rem' }}>
							<span style=${{
								display: 'inline-flex',
								alignItems: 'center',
								padding: '0.125rem 0.625rem',
								fontSize: '0.75rem',
								fontWeight: 500,
								borderRadius: '9999px',
								backgroundColor: getInterfaceStyle(selectedBlock.interface).bg,
								color: getInterfaceStyle(selectedBlock.interface).color
							}}>${selectedBlock.interface}</span>
							<span style=${{
								fontSize: '0.688rem',
								fontWeight: 500,
								padding: '0.125rem 0.5rem',
								borderRadius: '6px',
								backgroundColor: getModeColor(selectedBlock.instance_mode).bg,
								color: getModeColor(selectedBlock.instance_mode).color
							}}>${selectedBlock.instance_mode}</span>
							${(selectedBlock.allowed_modes || []).filter((m: string) => m !== selectedBlock!.instance_mode).map((m: string) => html`
								<span key=${m} style=${{
									fontSize: '0.625rem',
									padding: '0.0625rem 0.375rem',
									borderRadius: '4px',
									border: '1px solid var(--border-color, #e2e8f0)',
									color: 'var(--text-secondary, #64748b)'
								}}>${m}</span>
							`)}
						</div>

						<!-- Used in Chains -->
						<div style=${{ marginBottom: '1.25rem' }}>
							<h4 style=${{
								fontSize: '0.75rem',
								fontWeight: 600,
								color: 'var(--text-secondary, #64748b)',
								textTransform: 'uppercase',
								letterSpacing: '0.05em',
								marginBottom: '0.5rem'
							}}>Used in Chains</h4>
							${usedInChains.length > 0 ? html`
								<div style=${{ display: 'flex', flexDirection: 'column', gap: '0.375rem' }}>
									${usedInChains.map((c: ChainDef) => html`
										<a key=${c.id}
											href="#chains"
											style=${{
												display: 'flex',
												alignItems: 'center',
												gap: '0.375rem',
												padding: '0.5rem 0.75rem',
												background: 'var(--card-bg, white)',
												border: '1px solid var(--border-color, #e2e8f0)',
												borderRadius: '8px',
												textDecoration: 'none',
												color: 'inherit',
												fontSize: '0.813rem',
												transition: 'border-color 0.2s'
											}}
											onMouseEnter=${(e: any) => { e.currentTarget.style.borderColor = 'var(--primary-color, #189AB4)'; }}
											onMouseLeave=${(e: any) => { e.currentTarget.style.borderColor = 'var(--border-color, #e2e8f0)'; }}
										>
											<${GitBranch} size=${14} style=${{ color: 'var(--primary-color, #189AB4)', flexShrink: 0 }} />
											<code style=${{ fontWeight: 600 }}>${c.id}</code>
											${c.summary ? html`
												<span style=${{ fontSize: '0.688rem', color: 'var(--text-secondary, #64748b)' }}>— ${c.summary}</span>
											` : null}
										</a>
									`)}
								</div>
							` : html`
								<p style=${{ fontSize: '0.813rem', color: 'var(--text-secondary, #64748b)', margin: 0 }}>
									Not used in any chains.
								</p>
							`}
						</div>

						<!-- Block Info (JSON) -->
						<div>
							<button
								onClick=${() => setJsonExpanded(!jsonExpanded)}
								style=${{
									display: 'flex',
									alignItems: 'center',
									gap: '0.375rem',
									fontSize: '0.75rem',
									fontWeight: 600,
									color: 'var(--text-secondary, #64748b)',
									textTransform: 'uppercase',
									letterSpacing: '0.05em',
									background: 'none',
									border: 'none',
									padding: 0,
									cursor: 'pointer',
									marginBottom: '0.5rem'
								}}
							>
								${jsonExpanded
									? html`<${ChevronDown} size=${14} />`
									: html`<${ChevronRight} size=${14} />`
								}
								Block Info (JSON)
							</button>
							${jsonExpanded ? html`
								<pre style=${{
									background: 'var(--card-bg, white)',
									border: '1px solid var(--border-color, #e2e8f0)',
									borderRadius: '8px',
									padding: '1rem',
									fontSize: '0.75rem',
									lineHeight: 1.5,
									overflowX: 'auto',
									margin: 0,
									color: 'var(--text-primary, #1e293b)'
								}}>${JSON.stringify(selectedBlock, null, 2)}</pre>
							` : null}
						</div>
					</div>
				` : html`
					<div style=${{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
						<${EmptyState}
							icon=${Package}
							title="Select a block"
							description="Choose a block from the list to view its details."
						/>
					</div>
				`}
			</div>
		</div>
	`;
}
