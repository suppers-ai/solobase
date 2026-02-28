import { html } from '@solobase/ui';
import { FlowNode as FlowNodeType } from './types';

const TYPE_COLORS: Record<string, string> = {
	'block': '#3b82f6',
	'chain-ref': '#8b5cf6',
};

interface FlowNodeProps {
	node: FlowNodeType;
	selected: boolean;
	onSelect: (id: string) => void;
	onDragStart: (id: string, e: MouseEvent) => void;
}

export function FlowNodeComponent({ node, selected, onSelect, onDragStart }: FlowNodeProps) {
	const color = TYPE_COLORS[node.type] || '#6b7280';
	const strokeColor = selected ? '#f59e0b' : color;
	const strokeWidth = selected ? 2.5 : 1.5;

	return html`
		<g
			transform="translate(${node.x}, ${node.y})"
			style="cursor: grab"
			onMouseDown=${(e: MouseEvent) => {
				e.stopPropagation();
				onSelect(node.id);
				onDragStart(node.id, e);
			}}
		>
			<!-- Node body -->
			<rect
				width=${node.width}
				height=${node.height}
				rx="8"
				ry="8"
				fill="var(--card-bg, #1e293b)"
				stroke=${strokeColor}
				stroke-width=${strokeWidth}
			/>

			<!-- Type indicator bar -->
			<rect
				x="0" y="0"
				width="4"
				height=${node.height}
				rx="8" ry="0"
				fill=${color}
			/>

			<!-- Label -->
			<text
				x=${node.width / 2}
				y="22"
				text-anchor="middle"
				fill="var(--text-color, #e2e8f0)"
				font-size="13"
				font-weight="600"
				font-family="monospace"
			>
				${node.label.length > 20 ? node.label.slice(0, 18) + '...' : node.label}
			</text>

			<!-- Type badge -->
			<text
				x=${node.width / 2}
				y="40"
				text-anchor="middle"
				fill="var(--text-muted, #94a3b8)"
				font-size="10"
			>
				${node.type === 'chain-ref' ? 'chain' : 'block'}${node.match ? ` · ${node.match}` : ''}
			</text>

			<!-- Output port -->
			<circle
				cx=${node.width / 2}
				cy=${node.height}
				r="5"
				fill=${color}
				stroke="var(--card-bg, #1e293b)"
				stroke-width="2"
			/>

			<!-- Input port -->
			<circle
				cx=${node.width / 2}
				cy="0"
				r="5"
				fill=${color}
				stroke="var(--card-bg, #1e293b)"
				stroke-width="2"
			/>
		</g>
	`;
}
