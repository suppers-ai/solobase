import { html } from '@solobase/ui';
import { useState } from 'preact/hooks';
import { GitBranch, Box } from 'lucide-preact';
import { SearchInput, EmptyState } from '@solobase/ui';
import { flowJSONToFlow } from '../editor/serializer';

interface FlowDef {
	id: string;
	summary?: string;
	config?: { on_error: string; timeout?: string };
	http?: { routes: any[] };
	root?: any;
}

interface FlowsTabProps {
	flows: FlowDef[];
	onReload: () => void;
}

export function FlowsTab({ flows, onReload }: FlowsTabProps) {
	const [searchQuery, setSearchQuery] = useState('');
	const [selectedFlow, setSelectedFlow] = useState<FlowDef | null>(null);

	// Filter flows by search
	const filtered = flows.filter(c =>
		!searchQuery ||
		c.id.toLowerCase().includes(searchQuery.toLowerCase()) ||
		(c.summary || '').toLowerCase().includes(searchQuery.toLowerCase())
	);

	// Build preview data for selected flow
	let previewNodes: any[] = [];
	let previewEdges: any[] = [];
	if (selectedFlow?.root) {
		try {
			const result = flowJSONToFlow(selectedFlow);
			previewNodes = result.nodes;
			previewEdges = result.edges;
		} catch {
			// ignore parse errors
		}
	}

	return html`
		<div style=${{ display: 'flex', gap: '1rem', minHeight: '500px' }}>
			<!-- Left panel: flow list -->
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
					placeholder="Search flows..."
				/>

				<div style=${{ display: 'flex', flexDirection: 'column', gap: '0.5rem', overflowY: 'auto', flex: 1 }}>
					${filtered.length === 0 ? html`
						<div style=${{
							padding: '2rem 1rem',
							textAlign: 'center',
							color: 'var(--text-secondary, #64748b)',
							fontSize: '0.813rem'
						}}>
							${searchQuery ? 'No flows match your search.' : 'No flows registered.'}
						</div>
					` : filtered.map((c: FlowDef) => html`
						<div key=${c.id}
							onClick=${() => setSelectedFlow(c)}
							style=${{
								background: selectedFlow?.id === c.id ? 'var(--bg-muted, #f1f5f9)' : 'var(--card-bg, white)',
								border: selectedFlow?.id === c.id
									? '1px solid var(--primary-color, #189AB4)'
									: '1px solid var(--border-color, #e2e8f0)',
								borderRadius: '10px',
								padding: '0.875rem',
								transition: 'border-color 0.2s, box-shadow 0.2s',
								cursor: 'pointer'
							}} onMouseEnter=${(e: any) => {
								if (selectedFlow?.id !== c.id) {
									e.currentTarget.style.borderColor = 'var(--primary-color, #189AB4)';
									e.currentTarget.style.boxShadow = '0 2px 8px rgba(0,0,0,0.06)';
								}
							}} onMouseLeave=${(e: any) => {
								if (selectedFlow?.id !== c.id) {
									e.currentTarget.style.borderColor = 'var(--border-color, #e2e8f0)';
									e.currentTarget.style.boxShadow = 'none';
								}
							}}>
							<div style=${{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '0.375rem' }}>
								<code style=${{ fontSize: '0.85rem', fontWeight: 600, color: 'var(--text-primary, #1e293b)' }}>${c.id}</code>
							</div>

							${c.summary ? html`
								<p style=${{
									fontSize: '0.75rem',
									color: 'var(--text-secondary, #64748b)',
									margin: 0,
									lineHeight: 1.4
								}}>${c.summary}</p>
							` : null}
						</div>
					`)}
				</div>
			</div>

			<!-- Right panel: flow preview -->
			<div style=${{
				flex: 1,
				display: 'flex',
				flexDirection: 'column',
				background: 'var(--bg-muted, #f8fafc)',
				border: selectedFlow ? '1px solid var(--border-color, #e2e8f0)' : '2px dashed var(--border-color, #e2e8f0)',
				borderRadius: '12px',
				minHeight: '400px',
				overflow: 'hidden'
			}}>
				${selectedFlow ? html`
					<!-- Preview header -->
					<div style=${{
						display: 'flex',
						justifyContent: 'space-between',
						alignItems: 'center',
						padding: '0.75rem 1rem',
						borderBottom: '1px solid var(--border-color, #e2e8f0)',
						background: 'var(--card-bg, white)'
					}}>
						<div>
							<code style=${{ fontSize: '0.875rem', fontWeight: 600 }}>${selectedFlow.id}</code>
							${selectedFlow.summary ? html`
								<span style=${{ fontSize: '0.75rem', color: 'var(--text-secondary, #64748b)', marginLeft: '0.5rem' }}>
									— ${selectedFlow.summary}
								</span>
							` : null}
						</div>
					</div>

					<!-- Flow preview -->
					<div style=${{ flex: 1, padding: '1.5rem', overflowY: 'auto' }}>
						${previewNodes.length > 0 ? html`
							<div style=${{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
								${previewNodes.map((node: any, i: number) => html`
									<div key=${node.id}>
										<div style=${{
											display: 'flex',
											alignItems: 'center',
											gap: '0.5rem',
											padding: '0.625rem 0.875rem',
											background: 'var(--card-bg, white)',
											border: '1px solid var(--border-color, #e2e8f0)',
											borderRadius: '8px',
											fontSize: '0.813rem'
										}}>
											<${Box} size=${14} style=${{ color: 'var(--primary-color, #189AB4)', flexShrink: 0 }} />
											<code style=${{ fontWeight: 600, color: 'var(--text-primary, #1e293b)' }}>
												${node.label || '?'}
											</code>
											${node.match ? html`
												<span style=${{
													fontSize: '0.688rem',
													padding: '1px 6px',
													borderRadius: '4px',
													background: '#fef3c7',
													color: '#92400e'
												}}>match: ${node.match}</span>
											` : null}
										</div>
										${i < previewNodes.length - 1 ? html`
											<div style=${{
												display: 'flex',
												justifyContent: 'center',
												padding: '0.125rem 0',
												color: 'var(--text-secondary, #94a3b8)'
											}}>\u2193</div>
										` : null}
									</div>
								`)}
							</div>
						` : html`
							<${EmptyState}
								icon=${GitBranch}
								title="No blocks in flow"
								description="This flow has no block nodes defined."
							/>
						`}
					</div>
				` : html`
					<div style=${{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
						<${EmptyState}
							icon=${GitBranch}
							title="Select a flow"
							description="Choose a flow from the list to preview its flow."
						/>
					</div>
				`}
			</div>
		</div>
	`;
}
