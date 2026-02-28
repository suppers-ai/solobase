import { html } from '../../htm';
import { useRef, useEffect } from 'preact/hooks';
import { Search, X } from 'lucide-preact';

interface SearchInputProps {
	value: string;
	onChange: (value: string) => void;
	placeholder?: string;
	debounce?: number;
}

export function SearchInput({ value, onChange, placeholder = 'Search...', debounce = 300 }: SearchInputProps) {
	const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

	function handleInput(e: Event) {
		const val = (e.target as HTMLInputElement).value;
		if (debounce > 0) {
			if (timerRef.current) clearTimeout(timerRef.current);
			timerRef.current = setTimeout(() => onChange(val), debounce);
		} else {
			onChange(val);
		}
	}

	useEffect(() => {
		return () => {
			if (timerRef.current) clearTimeout(timerRef.current);
		};
	}, []);

	return html`
		<div style=${{ position: 'relative', display: 'inline-flex', alignItems: 'center' }}>
			<${Search} size=${16} style=${{
				position: 'absolute',
				left: '0.75rem',
				color: 'var(--text-muted, #94a3b8)',
				pointerEvents: 'none'
			}} />
			<input
				type="text"
				class="form-input"
				value=${value}
				onInput=${handleInput}
				placeholder=${placeholder}
				style=${{ paddingLeft: '2.25rem', paddingRight: value ? '2.25rem' : '0.875rem' }}
			/>
			${value ? html`
				<button
					type="button"
					onClick=${() => onChange('')}
					style=${{
						position: 'absolute',
						right: '0.5rem',
						background: 'none',
						border: 'none',
						cursor: 'pointer',
						padding: '2px',
						color: 'var(--text-muted, #94a3b8)',
						display: 'flex'
					}}
					aria-label="Clear search"
				>
					<${X} size=${14} />
				</button>
			` : null}
		</div>
	`;
}
