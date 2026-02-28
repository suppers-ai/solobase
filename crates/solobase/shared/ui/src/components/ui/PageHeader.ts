import { html } from '../../htm';
import type { ComponentChildren } from 'preact';

interface PageHeaderProps {
	title: string;
	description?: string;
	children?: ComponentChildren;
}

export function PageHeader({ title, description, children }: PageHeaderProps) {
	return html`
		<div style=${{
			display: 'flex',
			justifyContent: 'space-between',
			alignItems: 'flex-start',
			marginBottom: '1.5rem',
			flexWrap: 'wrap',
			gap: '1rem'
		}}>
			<div>
				<h1 style=${{ margin: 0, fontSize: '1.5rem', fontWeight: 700, color: 'var(--text-primary, #1e293b)' }}>${title}</h1>
				${description ? html`<p style=${{ margin: '0.25rem 0 0', fontSize: '0.875rem', color: 'var(--text-secondary, #64748b)' }}>${description}</p>` : null}
			</div>
			${children ? html`<div style=${{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>${children}</div>` : null}
		</div>
	`;
}
