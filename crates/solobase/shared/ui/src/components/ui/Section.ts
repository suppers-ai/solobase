import { html } from '../../htm';
import type { ComponentChildren } from 'preact';

interface SectionProps {
	title: string;
	description?: string;
	children?: ComponentChildren;
}

export function Section({ title, description, children }: SectionProps) {
	return html`
		<div class="form-section">
			<h3 class="form-section-title">${title}</h3>
			${description ? html`<p class="form-section-description">${description}</p>` : null}
			${children}
		</div>
	`;
}
