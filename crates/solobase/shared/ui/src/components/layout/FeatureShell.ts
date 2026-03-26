import { html } from '../../htm';
import { useEffect } from 'preact/hooks';
import { checkAuth, isAuthenticated, authLoading } from '../../stores/auth';
import { safeRedirect } from '../../utils/helpers';
import { ToastContainer } from '../ui/Toast';
import { LoadingSpinner } from '../ui/LoadingSpinner';
import type { ComponentChildren } from 'preact';
import { ArrowLeft } from 'lucide-preact';

interface FeatureShellProps {
	title: string;
	children?: ComponentChildren;
}

export function FeatureShell({ title, children }: FeatureShellProps) {
	useEffect(() => {
		checkAuth().then(authenticated => {
			if (!authenticated) {
				safeRedirect('/auth/login');
			}
		});
	}, []);

	useEffect(() => {
		document.title = `${title} - Solobase Admin`;
	}, [title]);

	if (authLoading.value) {
		return html`
			<div style=${{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh', background: 'var(--bg-secondary, #f0f0f0)' }}>
				<${LoadingSpinner} size="lg" message="Loading..." />
			</div>
		`;
	}

	if (!isAuthenticated.value) {
		return null;
	}

	return html`
		<div style=${{ minHeight: '100vh', background: 'var(--bg-secondary, #f8fafc)' }}>
			<header style=${{
				display: 'flex',
				alignItems: 'center',
				gap: '0.75rem',
				padding: '0.75rem 1.5rem',
				background: 'var(--card-bg, white)',
				borderBottom: '1px solid var(--border-color, #e2e8f0)'
			}}>
				<a
					href="/admin/wafer#blocks"
					style=${{
						display: 'inline-flex',
						alignItems: 'center',
						gap: '0.375rem',
						fontSize: '0.813rem',
						color: 'var(--text-secondary, #64748b)',
						textDecoration: 'none',
						padding: '0.25rem 0.5rem',
						borderRadius: '6px',
						transition: 'color 0.15s, background 0.15s'
					}}
					onMouseEnter=${(e: any) => {
						e.currentTarget.style.color = 'var(--text-primary, #1e293b)';
						e.currentTarget.style.background = 'var(--bg-muted, #f1f5f9)';
					}}
					onMouseLeave=${(e: any) => {
						e.currentTarget.style.color = 'var(--text-secondary, #64748b)';
						e.currentTarget.style.background = 'transparent';
					}}
				>
					<${ArrowLeft} size=${14} />
					Blocks
				</a>
				<span style=${{ color: 'var(--border-color, #e2e8f0)' }}>/</span>
				<span style=${{
					fontSize: '0.875rem',
					fontWeight: 600,
					color: 'var(--text-primary, #1e293b)'
				}}>${title}</span>
			</header>

			<main style=${{ padding: '1.5rem', maxWidth: '1400px', margin: '0 auto' }}>
				${children}
			</main>

			<${ToastContainer} />
		</div>
	`;
}
