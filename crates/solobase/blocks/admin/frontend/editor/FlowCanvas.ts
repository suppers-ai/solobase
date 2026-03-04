import { html } from '@solobase/ui';
import { useState, useRef, useCallback } from 'preact/hooks';
import { FlowNode, FlowEdge, FlowState, NODE_WIDTH, NODE_HEIGHT, generateId } from './types';
import { FlowNodeComponent } from './FlowNode';
import { FlowEdgeComponent } from './FlowEdge';
import { FlowControls } from './FlowControls';
import { NodePalette } from './NodePalette';
import { NodeConfig } from './NodeConfig';
import { autoLayout } from './layout';
import { flowToFlowJSON, flowJSONToFlow } from './serializer';

interface BlockInfo {
	name: string;
	version: string;
	interface: string;
	summary: string;
}

interface FlowCanvasProps {
	initialNodes?: FlowNode[];
	initialEdges?: FlowEdge[];
	blocks: BlockInfo[];
	flowIds: string[];
	flowConfig: { id: string; summary: string; on_error: string; timeout?: string };
	onSave: (flowJSON: any) => void;
	onValidate: (flowJSON: any) => Promise<{ valid: boolean; errors: string[] }>;
}

export function FlowCanvas({
	initialNodes = [],
	initialEdges = [],
	blocks,
	flowIds,
	flowConfig,
	onSave,
	onValidate,
}: FlowCanvasProps) {
	const [nodes, setNodes] = useState<FlowNode[]>(initialNodes);
	const [edges, setEdges] = useState<FlowEdge[]>(initialEdges);
	const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
	const [pan, setPan] = useState({ x: 0, y: 0 });
	const [zoom, setZoom] = useState(1);
	const [connecting, setConnecting] = useState<string | null>(null);
	const [validationErrors, setValidationErrors] = useState<string[]>([]);

	const svgRef = useRef<SVGSVGElement>(null);
	const dragState = useRef<{ nodeId: string; startX: number; startY: number; nodeStartX: number; nodeStartY: number } | null>(null);
	const panState = useRef<{ startX: number; startY: number; panStartX: number; panStartY: number } | null>(null);

	const selectedNode = nodes.find(n => n.id === selectedNodeId);

	// Add a block node
	const addBlock = useCallback((blockType: string) => {
		const node: FlowNode = {
			id: generateId(),
			type: 'block',
			label: blockType,
			x: 100 - pan.x / zoom,
			y: 100 - pan.y / zoom,
			width: NODE_WIDTH,
			height: NODE_HEIGHT,
		};
		setNodes(prev => [...prev, node]);
	}, [pan, zoom]);

	// Add a flow reference node
	const addFlowRef = useCallback((flowId: string) => {
		const node: FlowNode = {
			id: generateId(),
			type: 'flow-ref',
			label: flowId,
			x: 100 - pan.x / zoom,
			y: 100 - pan.y / zoom,
			width: NODE_WIDTH,
			height: NODE_HEIGHT,
		};
		setNodes(prev => [...prev, node]);
	}, [pan, zoom]);

	// Update a node
	const updateNode = useCallback((id: string, updates: Partial<FlowNode>) => {
		setNodes(prev => prev.map(n => n.id === id ? { ...n, ...updates } : n));
	}, []);

	// Delete a node
	const deleteNode = useCallback((id: string) => {
		setNodes(prev => prev.filter(n => n.id !== id));
		setEdges(prev => prev.filter(e => e.source !== id && e.target !== id));
		if (selectedNodeId === id) setSelectedNodeId(null);
	}, [selectedNodeId]);

	// Node dragging
	const handleNodeDragStart = useCallback((nodeId: string, e: MouseEvent) => {
		const node = nodes.find(n => n.id === nodeId);
		if (!node) return;
		dragState.current = {
			nodeId,
			startX: e.clientX,
			startY: e.clientY,
			nodeStartX: node.x,
			nodeStartY: node.y,
		};
	}, [nodes]);

	// Canvas panning
	const handleCanvasMouseDown = useCallback((e: MouseEvent) => {
		if (e.target === svgRef.current || (e.target as SVGElement).tagName === 'rect' && (e.target as SVGElement).classList.contains('canvas-bg')) {
			setSelectedNodeId(null);
			panState.current = {
				startX: e.clientX,
				startY: e.clientY,
				panStartX: pan.x,
				panStartY: pan.y,
			};
		}
	}, [pan]);

	const handleMouseMove = useCallback((e: MouseEvent) => {
		if (dragState.current) {
			const dx = (e.clientX - dragState.current.startX) / zoom;
			const dy = (e.clientY - dragState.current.startY) / zoom;
			updateNode(dragState.current.nodeId, {
				x: dragState.current.nodeStartX + dx,
				y: dragState.current.nodeStartY + dy,
			});
		}
		if (panState.current) {
			setPan({
				x: panState.current.panStartX + (e.clientX - panState.current.startX),
				y: panState.current.panStartY + (e.clientY - panState.current.startY),
			});
		}
	}, [zoom, updateNode]);

	const handleMouseUp = useCallback(() => {
		// If we were connecting, check if we dropped on a node
		dragState.current = null;
		panState.current = null;
		setConnecting(null);
	}, []);

	// Zoom
	const handleWheel = useCallback((e: WheelEvent) => {
		e.preventDefault();
		const delta = e.deltaY > 0 ? 0.9 : 1.1;
		setZoom(z => Math.max(0.25, Math.min(2, z * delta)));
	}, []);

	// Edge connection by double-click
	const handleNodeSelect = useCallback((id: string) => {
		if (connecting) {
			// Complete connection
			if (connecting !== id) {
				setEdges(prev => [...prev, { id: generateId(), source: connecting, target: id }]);
			}
			setConnecting(null);
		}
		setSelectedNodeId(id);
	}, [connecting]);

	// Start connection mode
	const startConnection = useCallback(() => {
		if (selectedNodeId) {
			setConnecting(selectedNodeId);
		}
	}, [selectedNodeId]);

	// Auto-layout
	const handleAutoLayout = useCallback(() => {
		setNodes(prev => autoLayout(prev, edges));
	}, [edges]);

	// Save
	const handleSave = useCallback(() => {
		const json = flowToFlowJSON(nodes, edges, flowConfig);
		onSave(json);
	}, [nodes, edges, flowConfig, onSave]);

	// Validate
	const handleValidate = useCallback(async () => {
		const json = flowToFlowJSON(nodes, edges, flowConfig);
		const result = await onValidate(json);
		setValidationErrors(result.errors);
	}, [nodes, edges, flowConfig, onValidate]);

	const viewBox = `${-pan.x / zoom} ${-pan.y / zoom} ${(svgRef.current?.clientWidth ?? 800) / zoom} ${(svgRef.current?.clientHeight ?? 600) / zoom}`;

	return html`
		<div style="display: flex; height: 100%; position: relative; overflow: hidden; border: 1px solid var(--border-color); border-radius: 8px; background: var(--bg-color, #0f172a)">
			<${NodePalette} blocks=${blocks} flowIds=${flowIds} onAddBlock=${addBlock} onAddFlowRef=${addFlowRef} />

			<div style="flex: 1; position: relative; overflow: hidden">
				${connecting && html`
					<div style="
						position: absolute; top: 8px; left: 50%; transform: translateX(-50%);
						background: #f59e0b; color: #000; padding: 4px 12px; border-radius: 4px;
						font-size: 12px; font-weight: 600; z-index: 10;
					">
						Click a target node to connect
					</div>
				`}

				<svg
					ref=${svgRef}
					width="100%"
					height="100%"
					viewBox=${viewBox}
					onMouseDown=${handleCanvasMouseDown}
					onMouseMove=${handleMouseMove}
					onMouseUp=${handleMouseUp}
					onWheel=${handleWheel}
					style="display: block"
				>
					<!-- Arrowhead marker -->
					<defs>
						<marker id="arrowhead" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto">
							<polygon points="0 0, 10 3.5, 0 7" fill="var(--border-color, #334155)" />
						</marker>
					</defs>

					<!-- Grid -->
					<defs>
						<pattern id="grid" width="20" height="20" patternUnits="userSpaceOnUse">
							<path d="M 20 0 L 0 0 0 20" fill="none" stroke="var(--border-color, #1e293b)" stroke-width="0.5" opacity="0.3" />
						</pattern>
					</defs>
					<rect class="canvas-bg" x="-10000" y="-10000" width="20000" height="20000" fill="url(#grid)" />

					<!-- Edges -->
					${edges.map(e => html`<${FlowEdgeComponent} key=${e.id} edge=${e} nodes=${nodes} />`)}

					<!-- Nodes -->
					${nodes.map(n => html`
						<${FlowNodeComponent}
							key=${n.id}
							node=${n}
							selected=${n.id === selectedNodeId}
							onSelect=${handleNodeSelect}
							onDragStart=${handleNodeDragStart}
						/>
					`)}
				</svg>

				<${FlowControls}
					zoom=${zoom}
					onZoomIn=${() => setZoom(z => Math.min(2, z * 1.2))}
					onZoomOut=${() => setZoom(z => Math.max(0.25, z / 1.2))}
					onFitView=${() => { setZoom(1); setPan({ x: 0, y: 0 }); }}
					onAutoLayout=${handleAutoLayout}
				/>

				<!-- Action buttons -->
				<div style="position: absolute; bottom: 12px; right: 12px; display: flex; gap: 8px; z-index: 10">
					${selectedNodeId && html`
						<button
							onClick=${startConnection}
							style="padding: 6px 12px; font-size: 12px; background: #3b82f6; color: white; border: none; border-radius: 6px; cursor: pointer"
						>
							Connect
						</button>
					`}
					<button
						onClick=${handleValidate}
						style="padding: 6px 12px; font-size: 12px; background: var(--card-bg); color: var(--text-color); border: 1px solid var(--border-color); border-radius: 6px; cursor: pointer"
					>
						Validate
					</button>
					<button
						onClick=${handleSave}
						style="padding: 6px 12px; font-size: 12px; background: #10b981; color: white; border: none; border-radius: 6px; cursor: pointer; font-weight: 600"
					>
						Save Flow
					</button>
				</div>

				${validationErrors.length > 0 && html`
					<div style="
						position: absolute; top: 12px; right: 12px; z-index: 10;
						background: var(--card-bg); border: 1px solid var(--danger-color, #ef4444);
						border-radius: 6px; padding: 8px 12px; max-width: 300px; font-size: 12px;
					">
						<div style="font-weight: 600; color: var(--danger-color); margin-bottom: 4px">Validation Errors</div>
						${validationErrors.map(e => html`<div style="color: var(--text-muted); margin-top: 2px">\u2022 ${e}</div>`)}
					</div>
				`}
			</div>

			${selectedNode && html`
				<${NodeConfig}
					node=${selectedNode}
					onUpdate=${updateNode}
					onDelete=${deleteNode}
					onClose=${() => setSelectedNodeId(null)}
				/>
			`}
		</div>
	`;
}
