// Flow editor types

export interface FlowNode {
	id: string;
	type: 'block' | 'chain-ref';
	label: string;          // block type or chain ID
	match?: string;         // match pattern
	config?: Record<string, unknown>;
	instance?: string;      // instance mode override
	x: number;
	y: number;
	width: number;
	height: number;
}

export interface FlowEdge {
	id: string;
	source: string;   // source node ID
	target: string;   // target node ID
}

export interface FlowState {
	nodes: FlowNode[];
	edges: FlowEdge[];
	selectedNodeId: string | null;
	pan: { x: number; y: number };
	zoom: number;
}

export interface ChainConfig {
	id: string;
	summary: string;
	on_error: string;
	timeout?: string;
}

export const NODE_WIDTH = 180;
export const NODE_HEIGHT = 60;
export const NODE_GAP_X = 60;
export const NODE_GAP_Y = 40;

let idCounter = 0;
export function generateId(): string {
	return `node-${Date.now()}-${++idCounter}`;
}
