import { html } from '../../htm';
import { toasts } from '../../stores/toast';
import { X, CheckCircle, AlertCircle, AlertTriangle, Info } from 'lucide-preact';

const ICONS = {
	success: CheckCircle,
	error: AlertCircle,
	warning: AlertTriangle,
	info: Info
};

const COLORS: Record<string, { bg: string; border: string; text: string }> = {
	success: { bg: '#f0fdf4', border: '#86efac', text: '#166534' },
	error: { bg: '#fef2f2', border: '#fca5a5', text: '#991b1b' },
	warning: { bg: '#fffbeb', border: '#fcd34d', text: '#92400e' },
	info: { bg: '#eff6ff', border: '#93c5fd', text: '#1e40af' }
};

export function ToastContainer() {
	const list = toasts.list.value;
	if (list.length === 0) return null;

	return html`
		<div class="toast-container" style=${{
			position: 'fixed',
			top: '1rem',
			right: '1rem',
			zIndex: 'var(--z-notification, 500)',
			display: 'flex',
			flexDirection: 'column',
			gap: '0.5rem',
			maxWidth: '400px'
		}}>
			${list.map(toast => {
				const Icon = ICONS[toast.type] || Info;
				const colors = COLORS[toast.type] || COLORS.info;
				return html`
					<div key=${toast.id} class="toast" style=${{
						background: colors.bg,
						border: `1px solid ${colors.border}`,
						borderRadius: '8px',
						padding: '0.75rem 1rem',
						display: 'flex',
						alignItems: 'flex-start',
						gap: '0.75rem',
						boxShadow: '0 4px 12px rgba(0,0,0,0.1)',
						animation: 'toastSlideIn 0.3s ease-out'
					}}>
						<${Icon} size=${18} style=${{ color: colors.text, flexShrink: 0, marginTop: '1px' }} />
						<div style=${{ flex: 1 }}>
							${toast.title ? html`<div style=${{ fontWeight: 600, fontSize: '0.875rem', color: colors.text, marginBottom: '0.25rem' }}>${toast.title}</div>` : null}
							<div style=${{ fontSize: '0.813rem', color: colors.text }}>${toast.message}</div>
						</div>
						${toast.dismissible !== false ? html`
							<button onClick=${() => toasts.dismiss(toast.id)} style=${{
								background: 'none',
								border: 'none',
								cursor: 'pointer',
								padding: '2px',
								color: colors.text,
								opacity: 0.6
							}} type="button" aria-label="Dismiss">
								<${X} size=${14} />
							</button>
						` : null}
					</div>
				`;
			})}
		</div>
	`;
}
