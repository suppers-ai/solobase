import { html } from '../../htm';
import { useEffect, useState } from 'preact/hooks';
import { checkAuth, isAuthenticated, authLoading } from '../../stores/auth';
import { Sidebar } from './Sidebar';
import { ToastContainer } from '../ui/Toast';
import { LoadingSpinner } from '../ui/LoadingSpinner';
import type { ComponentChildren } from 'preact';
import { Menu, X } from 'lucide-preact';

interface BlockShellProps {
	title: string;
	children?: ComponentChildren;
}

export function BlockShell({ title, children }: BlockShellProps) {
	const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

	useEffect(() => {
		checkAuth().then(authenticated => {
			if (!authenticated) {
				window.location.href = '/admin/login';
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
		<div class="app-layout">
			<div class="mobile-header">
				<button class="menu-toggle" onClick=${() => setMobileMenuOpen(v => !v)} type="button">
					${mobileMenuOpen ? html`<${X} size=${24} />` : html`<${Menu} size=${24} />`}
				</button>
				<span class="mobile-title">${title}</span>
			</div>

			<div class=${`sidebar-container${mobileMenuOpen ? ' active' : ''}`}>
				<div class="sidebar-overlay" onClick=${() => setMobileMenuOpen(false)} />
				<div class="sidebar-wrapper">
					<${Sidebar} />
				</div>
			</div>

			<main class="main-content">
				<div class="content-wrapper">
					${children}
				</div>
			</main>

			<${ToastContainer} />
		</div>
	`;
}
