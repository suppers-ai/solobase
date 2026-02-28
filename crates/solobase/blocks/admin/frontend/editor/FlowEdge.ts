import { html } from '@solobase/ui';
import { FlowNode, FlowEdge as FlowEdgeType } from './types';

interface FlowEdgeProps {
	edge: FlowEdgeType;
	nodes: FlowNode[];
}

export function FlowEdgeComponent({ edge, nodes }: FlowEdgeProps) {
	const source = nodes.find(n => n.id === edge.source);
	const target = nodes.find(n => n.id === edge.target);

	if (!source || !target) return null;

	// Start from output port (bottom center of source)
	const x1 = source.x + source.width / 2;
	const y1 = source.y + source.height;

	// End at input port (top center of target)
	const x2 = target.x + target.width / 2;
	const y2 = target.y;

	// Bezier control points for smooth curve
	const midY = (y1 + y2) / 2;
	const path = `M ${x1} ${y1} C ${x1} ${midY}, ${x2} ${midY}, ${x2} ${y2}`;

	return html`
		<g>
			<path
				d=${path}
				fill="none"
				stroke="var(--border-color, #334155)"
				stroke-width="2"
				marker-end="url(#arrowhead)"
			/>
		</g>
	`;
}
