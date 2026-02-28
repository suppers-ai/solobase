import { FlowNode, FlowEdge, generateId, NODE_WIDTH, NODE_HEIGHT } from './types';

// WAFFLE chain JSON types (matching the Go types)
interface WaffleNodeDef {
	block?: string;
	chain?: string;
	match?: string;
	config?: Record<string, unknown>;
	instance?: string;
	next?: WaffleNodeDef[];
}

interface WaffleChainDef {
	id: string;
	summary: string;
	config: { on_error: string; timeout?: string };
	root: WaffleNodeDef;
}

// Converts a flow graph (nodes + edges) back to WAFFLE chain JSON.
export function flowToChainJSON(
	nodes: FlowNode[],
	edges: FlowEdge[],
	chainConfig: { id: string; summary: string; on_error: string; timeout?: string }
): WaffleChainDef {
	// Build children map
	const children = new Map<string, string[]>();
	for (const n of nodes) children.set(n.id, []);
	for (const e of edges) {
		children.get(e.source)?.push(e.target);
	}

	// Find root (no incoming edges)
	const targets = new Set(edges.map(e => e.target));
	const roots = nodes.filter(n => !targets.has(n.id));
	const root = roots[0] || nodes[0];

	if (!root) {
		return {
			...chainConfig,
			config: { on_error: chainConfig.on_error, timeout: chainConfig.timeout },
			root: { block: 'placeholder' },
		};
	}

	function nodeToWaffle(flowNode: FlowNode): WaffleNodeDef {
		const def: WaffleNodeDef = {};

		if (flowNode.type === 'chain-ref') {
			def.chain = flowNode.label;
		} else {
			def.block = flowNode.label;
		}

		if (flowNode.match) def.match = flowNode.match;
		if (flowNode.config && Object.keys(flowNode.config).length > 0) def.config = flowNode.config;
		if (flowNode.instance) def.instance = flowNode.instance;

		const childIds = children.get(flowNode.id) || [];
		if (childIds.length > 0) {
			def.next = childIds
				.map(id => nodes.find(n => n.id === id))
				.filter((n): n is FlowNode => n !== undefined)
				.map(nodeToWaffle);
		}

		return def;
	}

	return {
		id: chainConfig.id,
		summary: chainConfig.summary,
		config: { on_error: chainConfig.on_error, timeout: chainConfig.timeout },
		root: nodeToWaffle(root),
	};
}

// Converts WAFFLE chain JSON to a flow graph (nodes + edges).
export function chainJSONToFlow(chainDef: WaffleChainDef): { nodes: FlowNode[]; edges: FlowEdge[] } {
	const nodes: FlowNode[] = [];
	const edges: FlowEdge[] = [];

	function parseNode(def: WaffleNodeDef, parentId: string | null, x: number, y: number): string {
		const id = generateId();
		const node: FlowNode = {
			id,
			type: def.chain ? 'chain-ref' : 'block',
			label: def.chain || def.block || 'unknown',
			match: def.match,
			config: def.config,
			instance: def.instance,
			x,
			y,
			width: NODE_WIDTH,
			height: NODE_HEIGHT,
		};
		nodes.push(node);

		if (parentId) {
			edges.push({ id: generateId(), source: parentId, target: id });
		}

		if (def.next) {
			def.next.forEach((child, i) => {
				parseNode(child, id, x + i * (NODE_WIDTH + 60), y + NODE_HEIGHT + 40);
			});
		}

		return id;
	}

	parseNode(chainDef.root, null, 60, 60);

	return { nodes, edges };
}
