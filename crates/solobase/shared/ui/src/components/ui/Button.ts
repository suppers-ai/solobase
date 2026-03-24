import { html } from '../../htm';
import type { ComponentChildren } from 'preact';

interface ButtonProps {
	variant?: 'primary' | 'secondary' | 'danger' | 'ghost' | 'link';
	size?: 'sm' | 'md' | 'lg';
	icon?: any;
	iconOnly?: boolean;
	disabled?: boolean;
	loading?: boolean;
	type?: 'button' | 'submit' | 'reset';
	href?: string;
	onClick?: (e: MouseEvent) => void;
	children?: ComponentChildren;
	class?: string;
}

export function Button({
	variant = 'primary',
	size = 'md',
	icon: Icon,
	iconOnly = false,
	disabled = false,
	loading = false,
	type = 'button',
	href,
	onClick,
	children,
	class: className = ''
}: ButtonProps) {
	const iconSize = size === 'sm' ? 14 : size === 'lg' ? 18 : 16;
	const classes = `btn btn-${variant} btn-${size}${iconOnly ? ' icon-only' : ''}${disabled ? ' disabled' : ''} ${className}`.trim();

	function handleClick(e: MouseEvent) {
		if (!disabled && !loading && onClick) {
			onClick(e);
		}
	}

	const content = html`
		${loading
			? html`<span class="spinner"></span>`
			: Icon
				? html`<${Icon} size=${iconSize} />`
				: null
		}
		${!iconOnly ? children : null}
	`;

	if (href) {
		return html`<a href=${href} class=${classes} onClick=${handleClick}>${content}</a>`;
	}

	return html`<button type=${type} class=${classes} disabled=${disabled || loading} onClick=${handleClick}>${content}</button>`;
}
