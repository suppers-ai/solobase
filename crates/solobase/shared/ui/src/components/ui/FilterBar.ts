import { html } from '../../htm';
import { SearchInput } from './SearchInput';
import type { ComponentChildren } from 'preact';

interface FilterBarProps {
	search: string;
	onSearchChange: (value: string) => void;
	searchPlaceholder?: string;
	children?: ComponentChildren;
}

export function FilterBar({ search, onSearchChange, searchPlaceholder, children }: FilterBarProps) {
	return html`
		<div style=${{
			display: 'flex',
			justifyContent: 'space-between',
			alignItems: 'center',
			gap: '1rem',
			marginBottom: '1rem',
			flexWrap: 'wrap'
		}}>
			<${SearchInput} value=${search} onChange=${onSearchChange} placeholder=${searchPlaceholder} />
			${children ? html`<div style=${{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>${children}</div>` : null}
		</div>
	`;
}
