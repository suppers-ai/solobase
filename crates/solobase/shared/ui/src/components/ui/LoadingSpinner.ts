import { html } from '../../htm';

interface LoadingSpinnerProps {
	size?: 'sm' | 'md' | 'lg';
	message?: string;
}

export function LoadingSpinner({ size = 'md', message }: LoadingSpinnerProps) {
	const sizeMap = { sm: '24px', md: '40px', lg: '56px' };
	const dim = sizeMap[size];

	return html`
		<div style=${{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: '0.75rem', padding: '2rem' }}>
			<div style=${{
				width: dim,
				height: dim,
				border: '3px solid #e5e7eb',
				borderTopColor: 'var(--primary-color, #fe6627)',
				borderRadius: '50%',
				animation: 'spin 0.6s linear infinite'
			}} />
			${message ? html`<span style=${{ fontSize: '0.875rem', color: 'var(--text-secondary, #6b7280)' }}>${message}</span>` : null}
		</div>
	`;
}
