import { html } from '../../htm';

interface StatCardProps {
	title: string;
	value: string | number;
	icon?: any;
	trend?: { value: number; label: string };
	color?: string;
}

export function StatCard({ title, value, icon: Icon, trend, color = 'var(--primary-color, #189AB4)' }: StatCardProps) {
	return html`
		<div style=${{
			background: 'white',
			border: '1px solid var(--border-color, #e2e8f0)',
			borderRadius: '12px',
			padding: '1.25rem',
			display: 'flex',
			flexDirection: 'column',
			gap: '0.5rem'
		}}>
			<div style=${{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
				<span style=${{ fontSize: '0.813rem', fontWeight: 500, color: 'var(--text-secondary, #64748b)', textTransform: 'uppercase', letterSpacing: '0.025em' }}>${title}</span>
				${Icon ? html`<${Icon} size=${20} style=${{ color }} />` : null}
			</div>
			<div style=${{ fontSize: '1.75rem', fontWeight: 700, color: 'var(--text-primary, #1e293b)' }}>${value}</div>
			${trend ? html`
				<div style=${{ fontSize: '0.75rem', color: trend.value >= 0 ? 'var(--success-color, #10b981)' : 'var(--danger-color, #ef4444)' }}>
					${trend.value >= 0 ? '+' : ''}${trend.value}% ${trend.label}
				</div>
			` : null}
		</div>
	`;
}
