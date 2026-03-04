import { html } from '@solobase/ui';
import { Package, GitBranch } from 'lucide-preact';

interface BlockInfo {
	name: string;
	version: string;
	interface: string;
	summary: string;
}

interface NodePaletteProps {
	blocks: BlockInfo[];
	onAddBlock: (blockType: string) => void;
	onAddFlowRef: (flowId: string) => void;
	flowIds: string[];
}

export function NodePalette({ blocks, onAddBlock, onAddFlowRef, flowIds }: NodePaletteProps) {
	return html`
		<div style="
			width: 220px; border-right: 1px solid var(--border-color);
			overflow-y: auto; padding: 12px; background: var(--bg-secondary, #0f172a);
			flex-shrink: 0;
		">
			<div style="font-weight: 600; font-size: 12px; text-transform: uppercase; color: var(--text-muted); margin-bottom: 8px; letter-spacing: 0.05em">
				Blocks
			</div>
			${blocks.map(b => html`
				<button
					key=${b.name}
					onClick=${() => onAddBlock(b.name)}
					style="
						display: flex; align-items: center; gap: 8px;
						width: 100%; padding: 8px; margin-bottom: 4px;
						background: var(--card-bg); border: 1px solid var(--border-color);
						border-radius: 6px; cursor: pointer; color: var(--text-color);
						font-size: 12px; text-align: left;
					"
					title=${b.summary}
				>
					<${Package} size=${14} color="#3b82f6" />
					<span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap">${b.name}</span>
				</button>
			`)}

			${flowIds.length > 0 && html`
				<div style="font-weight: 600; font-size: 12px; text-transform: uppercase; color: var(--text-muted); margin: 12px 0 8px; letter-spacing: 0.05em">
					Flow Refs
				</div>
				${flowIds.map(id => html`
					<button
						key=${id}
						onClick=${() => onAddFlowRef(id)}
						style="
							display: flex; align-items: center; gap: 8px;
							width: 100%; padding: 8px; margin-bottom: 4px;
							background: var(--card-bg); border: 1px solid var(--border-color);
							border-radius: 6px; cursor: pointer; color: var(--text-color);
							font-size: 12px; text-align: left;
						"
					>
						<${GitBranch} size=${14} color="#8b5cf6" />
						<span>${id}</span>
					</button>
				`)}
			`}
		</div>
	`;
}
