import { html } from '@solobase/ui';
import { Settings, Trash2, X } from 'lucide-preact';
import { FlowNode } from './types';

interface NodeConfigProps {
	node: FlowNode;
	onUpdate: (id: string, updates: Partial<FlowNode>) => void;
	onDelete: (id: string) => void;
	onClose: () => void;
}

export function NodeConfig({ node, onUpdate, onDelete, onClose }: NodeConfigProps) {
	const inputStyle = `
		width: 100%; padding: 6px 8px; font-size: 12px;
		background: var(--input-bg, #1e293b); border: 1px solid var(--border-color);
		border-radius: 4px; color: var(--text-color);
		font-family: monospace;
	`;

	const labelStyle = `
		display: block; font-size: 11px; font-weight: 600;
		color: var(--text-muted); margin-bottom: 4px; text-transform: uppercase;
		letter-spacing: 0.05em;
	`;

	return html`
		<div style="
			width: 260px; border-left: 1px solid var(--border-color);
			overflow-y: auto; padding: 12px; background: var(--bg-secondary, #0f172a);
			flex-shrink: 0;
		">
			<div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px">
				<div style="display: flex; align-items: center; gap: 6px; font-weight: 600; font-size: 13px">
					<${Settings} size=${14} />
					Node Config
				</div>
				<button onClick=${onClose} style="background: none; border: none; cursor: pointer; color: var(--text-muted); padding: 4px">
					<${X} size=${14} />
				</button>
			</div>

			<div style="margin-bottom: 12px">
				<label style=${labelStyle}>Type</label>
				<select
					value=${node.type}
					onChange=${(e: any) => onUpdate(node.id, { type: e.target.value })}
					style=${inputStyle}
				>
					<option value="block">Block</option>
					<option value="chain-ref">Chain Reference</option>
				</select>
			</div>

			<div style="margin-bottom: 12px">
				<label style=${labelStyle}>${node.type === 'chain-ref' ? 'Chain ID' : 'Block Type'}</label>
				<input
					type="text"
					value=${node.label}
					onInput=${(e: any) => onUpdate(node.id, { label: e.target.value })}
					style=${inputStyle}
				/>
			</div>

			<div style="margin-bottom: 12px">
				<label style=${labelStyle}>Match Pattern</label>
				<input
					type="text"
					value=${node.match || ''}
					onInput=${(e: any) => onUpdate(node.id, { match: e.target.value || undefined })}
					style=${inputStyle}
					placeholder="e.g., user.* or user.create"
				/>
			</div>

			${node.type === 'block' && html`
				<div style="margin-bottom: 12px">
					<label style=${labelStyle}>Instance Mode</label>
					<select
						value=${node.instance || ''}
						onChange=${(e: any) => onUpdate(node.id, { instance: e.target.value || undefined })}
						style=${inputStyle}
					>
						<option value="">Default (per-node)</option>
						<option value="singleton">Singleton</option>
						<option value="per-chain">Per Chain</option>
						<option value="per-execution">Per Execution</option>
						<option value="pooled">Pooled</option>
					</select>
				</div>
			`}

			<div style="margin-bottom: 12px">
				<label style=${labelStyle}>Config (JSON)</label>
				<textarea
					value=${node.config ? JSON.stringify(node.config, null, 2) : ''}
					onInput=${(e: any) => {
						try {
							const parsed = e.target.value ? JSON.parse(e.target.value) : undefined;
							onUpdate(node.id, { config: parsed });
						} catch { /* ignore parse errors while typing */ }
					}}
					style="${inputStyle} min-height: 80px; resize: vertical"
					placeholder='{ "key": "value" }'
				/>
			</div>

			<button
				onClick=${() => onDelete(node.id)}
				style="
					display: flex; align-items: center; gap: 6px; width: 100%;
					padding: 8px; background: none; border: 1px solid var(--danger-color, #ef4444);
					border-radius: 6px; cursor: pointer; color: var(--danger-color, #ef4444);
					font-size: 12px; justify-content: center;
				"
			>
				<${Trash2} size=${14} />
				Delete Node
			</button>
		</div>
	`;
}
