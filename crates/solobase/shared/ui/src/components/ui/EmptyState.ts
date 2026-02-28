import { html } from '../../htm';
import type { ComponentChildren } from 'preact';

interface EmptyStateProps {
	icon?: any;
	title: string;
	description?: string;
	children?: ComponentChildren;
}

export function EmptyState({ icon: Icon, title, description, children }: EmptyStateProps) {
	return html`
		<div style=${{
			display: 'flex',
			flexDirection: 'column',
			alignItems: 'center',
			justifyContent: 'center',
			padding: '3rem 2rem',
			textAlign: 'center'
		}}>
			${Icon ? html`<${Icon} size=${48} style=${{ color: 'var(--text-muted, #94a3b8)', marginBottom: '1rem' }} />` : null}
			<h3 style=${{ margin: '0 0 0.5rem', fontSize: '1.125rem', fontWeight: 600, color: 'var(--text-primary, #1e293b)' }}>${title}</h3>
			${description ? html`<p style=${{ margin: '0 0 1rem', fontSize: '0.875rem', color: 'var(--text-secondary, #64748b)', maxWidth: '400px' }}>${description}</p>` : null}
			${children}
		</div>
	`;
}
