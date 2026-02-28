import { FlowNode, FlowEdge, NODE_WIDTH, NODE_HEIGHT, NODE_GAP_X, NODE_GAP_Y } from './types';

// Simple top-down auto-layout (Sugiyama-style, simplified).
// Assigns layers based on topological order, then centers nodes within each layer.
export function autoLayout(nodes: FlowNode[], edges: FlowEdge[]): FlowNode[] {
	if (nodes.length === 0) return nodes;

	// Build adjacency
	const children = new Map<string, string[]>();
	const parents = new Map<string, string[]>();
	for (const n of nodes) {
		children.set(n.id, []);
		parents.set(n.id, []);
	}
	for (const e of edges) {
		children.get(e.source)?.push(e.target);
		parents.get(e.target)?.push(e.source);
	}

	// Find roots (no parents)
	const roots = nodes.filter(n => (parents.get(n.id)?.length ?? 0) === 0);
	if (roots.length === 0) {
		// Fallback: treat first node as root
		roots.push(nodes[0]);
	}

	// Assign layers via BFS
	const layers = new Map<string, number>();
	const queue: string[] = [];

	for (const r of roots) {
		layers.set(r.id, 0);
		queue.push(r.id);
	}

	while (queue.length > 0) {
		const id = queue.shift()!;
		const layer = layers.get(id)!;
		for (const childId of children.get(id) ?? []) {
			const existing = layers.get(childId);
			if (existing === undefined || existing < layer + 1) {
				layers.set(childId, layer + 1);
				queue.push(childId);
			}
		}
	}

	// Assign positions not yet in layers
	for (const n of nodes) {
		if (!layers.has(n.id)) {
			layers.set(n.id, 0);
		}
	}

	// Group by layer
	const layerGroups = new Map<number, FlowNode[]>();
	for (const n of nodes) {
		const layer = layers.get(n.id)!;
		if (!layerGroups.has(layer)) layerGroups.set(layer, []);
		layerGroups.get(layer)!.push(n);
	}

	// Position nodes
	const maxLayer = Math.max(...layers.values());
	const startX = 60;
	const startY = 60;

	return nodes.map(n => {
		const layer = layers.get(n.id)!;
		const group = layerGroups.get(layer)!;
		const idx = group.indexOf(n);
		const groupWidth = group.length * (NODE_WIDTH + NODE_GAP_X) - NODE_GAP_X;
		const totalWidth = Math.max(groupWidth, NODE_WIDTH);

		return {
			...n,
			x: startX + (totalWidth - groupWidth) / 2 + idx * (NODE_WIDTH + NODE_GAP_X),
			y: startY + layer * (NODE_HEIGHT + NODE_GAP_Y),
		};
	});
}
