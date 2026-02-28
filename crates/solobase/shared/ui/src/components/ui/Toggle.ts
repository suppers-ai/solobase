import { html } from '../../htm';

interface ToggleProps {
	checked: boolean;
	onChange: (checked: boolean) => void;
	label?: string;
	disabled?: boolean;
}

export function Toggle({ checked, onChange, label, disabled = false }: ToggleProps) {
	return html`
		<label style=${{
			display: 'inline-flex',
			alignItems: 'center',
			gap: '0.5rem',
			cursor: disabled ? 'not-allowed' : 'pointer',
			opacity: disabled ? 0.5 : 1
		}}>
			<button
				type="button"
				role="switch"
				aria-checked=${checked}
				disabled=${disabled}
				onClick=${() => !disabled && onChange(!checked)}
				style=${{
					position: 'relative',
					width: '44px',
					height: '24px',
					borderRadius: '12px',
					border: 'none',
					background: checked ? 'var(--primary-color, #189AB4)' : '#d1d5db',
					cursor: disabled ? 'not-allowed' : 'pointer',
					transition: 'background 0.2s',
					flexShrink: 0
				}}
			>
				<span style=${{
					position: 'absolute',
					top: '2px',
					left: checked ? '22px' : '2px',
					width: '20px',
					height: '20px',
					borderRadius: '50%',
					background: 'white',
					boxShadow: '0 1px 3px rgba(0,0,0,0.2)',
					transition: 'left 0.2s'
				}} />
			</button>
			${label ? html`<span style=${{ fontSize: '0.875rem', color: 'var(--text-primary, #374151)' }}>${label}</span>` : null}
		</label>
	`;
}
