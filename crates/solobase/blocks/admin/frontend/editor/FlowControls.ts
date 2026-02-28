import { html } from '@solobase/ui';
import { ZoomIn, ZoomOut, Maximize2 } from 'lucide-preact';

interface FlowControlsProps {
	zoom: number;
	onZoomIn: () => void;
	onZoomOut: () => void;
	onFitView: () => void;
	onAutoLayout: () => void;
}

export function FlowControls({ zoom, onZoomIn, onZoomOut, onFitView, onAutoLayout }: FlowControlsProps) {
	const btnStyle = `
		display: flex; align-items: center; justify-content: center;
		width: 32px; height: 32px; border: 1px solid var(--border-color);
		background: var(--card-bg); color: var(--text-color);
		border-radius: 6px; cursor: pointer; font-size: 12px;
	`;

	return html`
		<div style="position: absolute; bottom: 12px; left: 12px; display: flex; gap: 4px; z-index: 10">
			<button style=${btnStyle} onClick=${onZoomOut} title="Zoom out">
				<${ZoomOut} size=${16} />
			</button>
			<button style="${btnStyle} width: 48px; font-family: monospace" disabled>
				${Math.round(zoom * 100)}%
			</button>
			<button style=${btnStyle} onClick=${onZoomIn} title="Zoom in">
				<${ZoomIn} size=${16} />
			</button>
			<button style=${btnStyle} onClick=${onFitView} title="Fit view">
				<${Maximize2} size=${16} />
			</button>
			<button style="${btnStyle} width: auto; padding: 0 8px; font-size: 11px" onClick=${onAutoLayout} title="Auto layout">
				Layout
			</button>
		</div>
	`;
}
