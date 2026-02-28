import { html } from '../../htm';
import { Button } from './Button';
import { ChevronLeft, ChevronRight } from 'lucide-preact';

interface PaginationProps {
	currentPage: number;
	totalPages: number;
	onPageChange: (page: number) => void;
}

export function Pagination({ currentPage, totalPages, onPageChange }: PaginationProps) {
	if (totalPages <= 1) return null;

	return html`
		<div style=${{
			display: 'flex',
			justifyContent: 'center',
			alignItems: 'center',
			gap: '0.5rem',
			marginTop: '1rem'
		}}>
			<${Button}
				variant="secondary"
				size="sm"
				icon=${ChevronLeft}
				iconOnly
				disabled=${currentPage <= 1}
				onClick=${() => onPageChange(currentPage - 1)}
			/>
			<span style=${{ fontSize: '0.875rem', color: 'var(--text-secondary, #64748b)' }}>
				Page ${currentPage} of ${totalPages}
			</span>
			<${Button}
				variant="secondary"
				size="sm"
				icon=${ChevronRight}
				iconOnly
				disabled=${currentPage >= totalPages}
				onClick=${() => onPageChange(currentPage + 1)}
			/>
		</div>
	`;
}
