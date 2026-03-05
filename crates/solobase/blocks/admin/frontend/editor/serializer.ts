import { FlowNode, FlowEdge, generateId, NODE_WIDTH, NODE_HEIGHT } from './types';

// Wafer flow JSON types (matching the Go types)
interface FlowNodeDef {
	block?: string;
	flow?: string;
	match?: string;
	config?: Record<string, unknown>;
	instance?: string;
	next?: FlowNodeDef[];
}

interface WaferFlowDef {
	id: string;
	summary: string;
	config: { on_error: string; timeout?: string };
	root: FlowNodeDef;
}

// Converts a flow graph (nodes + edges) back to Wafer flow JSON.
export function flowToFlowJSON(
	nodes: FlowNode[],
	edges: FlowEdge[],
	flowConfig: { id: string; summary: string; on_error: string; timeout?: string }
): WaferFlowDef {
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
			...flowConfig,
			config: { on_error: flowConfig.on_error, timeout: flowConfig.timeout },
			root: { block: 'placeholder' },
		};
	}

	function nodeToFlowDef(flowNode: FlowNode): FlowNodeDef {
		const def: FlowNodeDef = {};

		if (flowNode.type === 'flow-ref') {
			def.flow = flowNode.label;
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
				.map(nodeToFlowDef);
		}

		return def;
	}

	return {
		id: flowConfig.id,
		summary: flowConfig.summary,
		config: { on_error: flowConfig.on_error, timeout: flowConfig.timeout },
		root: nodeToFlowDef(root),
	};
}

// Converts Wafer flow JSON to a flow graph (nodes + edges).
export function flowJSONToFlow(flowDef: WaferFlowDef): { nodes: FlowNode[]; edges: FlowEdge[] } {
	const nodes: FlowNode[] = [];
	const edges: FlowEdge[] = [];

	function parseNode(def: FlowNodeDef, parentId: string | null, x: number, y: number): string {
		const id = generateId();
		const node: FlowNode = {
			id,
			type: def.flow ? 'flow-ref' : 'block',
			label: def.flow || def.block || 'unknown',
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

	parseNode(flowDef.root, null, 60, 60);

	return { nodes, edges };
}
