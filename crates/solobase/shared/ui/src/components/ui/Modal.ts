import { html } from '../../htm';
import { useKeydown } from '../../hooks/useKeydown';
import { X } from 'lucide-preact';
import type { ComponentChildren } from 'preact';

interface ModalProps {
	show: boolean;
	title?: string;
	maxWidth?: string;
	closeOnOverlay?: boolean;
	onClose: () => void;
	children?: ComponentChildren;
	footer?: ComponentChildren;
}

export function Modal({
	show,
	title = '',
	maxWidth = '500px',
	closeOnOverlay = true,
	onClose,
	children,
	footer
}: ModalProps) {
	useKeydown('Escape', () => onClose(), show);

	if (!show) return null;

	function handleOverlayClick() {
		if (closeOnOverlay) onClose();
	}

	return html`
		<div class="modal-overlay" onClick=${handleOverlayClick}>
			<div class="modal" style=${{ maxWidth }} onClick=${(e: Event) => e.stopPropagation()}>
				<div class="modal-header">
					<h3>${title}</h3>
					<button class="icon-button" onClick=${onClose} type="button" aria-label="Close">
						<${X} size=${20} />
					</button>
				</div>
				<div class="modal-body">
					${children}
				</div>
				${footer ? html`<div class="modal-footer">${footer}</div>` : null}
			</div>
		</div>
	`;
}
