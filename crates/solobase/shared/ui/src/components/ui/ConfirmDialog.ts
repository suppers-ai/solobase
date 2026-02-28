import { html } from '../../htm';
import { Modal } from './Modal';
import { Button } from './Button';

interface ConfirmDialogProps {
	show: boolean;
	title?: string;
	message: string;
	confirmText?: string;
	cancelText?: string;
	variant?: 'danger' | 'primary';
	loading?: boolean;
	onConfirm: () => void;
	onCancel: () => void;
}

export function ConfirmDialog({
	show,
	title = 'Confirm',
	message,
	confirmText = 'Confirm',
	cancelText = 'Cancel',
	variant = 'danger',
	loading = false,
	onConfirm,
	onCancel
}: ConfirmDialogProps) {
	const footer = html`
		<${Button} variant="secondary" onClick=${onCancel}>${cancelText}<//>
		<${Button} variant=${variant} onClick=${onConfirm} loading=${loading}>${confirmText}<//>
	`;

	return html`
		<${Modal} show=${show} title=${title} onClose=${onCancel} maxWidth="400px" footer=${footer}>
			<p style=${{ margin: 0, color: '#4b5563', fontSize: '0.875rem', lineHeight: '1.5' }}>${message}</p>
		<//>
	`;
}
