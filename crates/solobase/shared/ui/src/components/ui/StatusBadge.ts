import { html } from '../../htm';

interface StatusBadgeProps {
	status: string;
	variant?: 'success' | 'warning' | 'danger' | 'info' | 'neutral';
}

const VARIANT_STYLES: Record<string, { bg: string; color: string }> = {
	success: { bg: '#dcfce7', color: '#166534' },
	warning: { bg: '#fef3c7', color: '#92400e' },
	danger: { bg: '#fee2e2', color: '#991b1b' },
	info: { bg: '#dbeafe', color: '#1e40af' },
	neutral: { bg: '#f1f5f9', color: '#475569' }
};

export function StatusBadge({ status, variant = 'neutral' }: StatusBadgeProps) {
	const styles = VARIANT_STYLES[variant] || VARIANT_STYLES.neutral;

	return html`
		<span style=${{
			display: 'inline-flex',
			alignItems: 'center',
			padding: '0.125rem 0.625rem',
			fontSize: '0.75rem',
			fontWeight: 500,
			borderRadius: '9999px',
			backgroundColor: styles.bg,
			color: styles.color
		}}>${status}</span>
	`;
}
