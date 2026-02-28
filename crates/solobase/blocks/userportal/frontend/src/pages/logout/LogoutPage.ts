import { html, logout, LoadingSpinner } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { LogOut } from 'lucide-preact';

export function LogoutPage() {
	const [message, setMessage] = useState('Logging out...');
	const [isLoading, setIsLoading] = useState(true);

	useEffect(() => {
		(async () => {
			try {
				await logout();
				setMessage('You have been logged out successfully.');
				setIsLoading(false);
				setTimeout(() => {
					window.location.href = '/login';
				}, 1500);
			} catch (err) {
				console.error('Logout error:', err);
				setMessage('Logout completed.');
				setIsLoading(false);
				setTimeout(() => {
					window.location.href = '/login';
				}, 1500);
			}
		})();
	}, []);

	return html`
		<div class="logout-page">
			<div class="logout-container">
				<div class="logout-logo">
					<img src="/logo_long.png" alt="Solobase" class="logo-image" />
				</div>
				<div class="logout-content">
					<div class="logout-icon">
						<${LogOut} size=${32} />
					</div>
					<h1 class="logout-title">
						${isLoading ? 'Signing Out' : 'Signed Out'}
					</h1>
					<p class="logout-message">${message}</p>
					${isLoading ? html`
						<div class="spinner-container">
							<${LoadingSpinner} size="md" color="primary" />
						</div>
					` : html`
						<p class="redirect-message">Redirecting to login...</p>
					`}
				</div>
				<div class="logout-footer">
					<a href="/login" class="login-link">
						Click here if you're not redirected
					</a>
				</div>
			</div>
		</div>
		<style>
			.logout-page {
				min-height: 100vh;
				display: flex;
				align-items: center;
				justify-content: center;
				background: #f0f0f0;
				padding: 1rem;
			}
			.logout-container {
				width: 100%;
				max-width: 420px;
				background: white;
				border: 1px solid #e2e8f0;
				border-radius: 12px;
				padding: 2.5rem;
				text-align: center;
				animation: slideUp 0.4s ease-out;
			}
			@keyframes slideUp {
				from { opacity: 0; transform: translateY(20px); }
				to { opacity: 1; transform: translateY(0); }
			}
			.logout-logo { margin-bottom: 2rem; }
			.logo-image { height: 60px; width: auto; margin: 0 auto; }
			.logout-content { margin-bottom: 1.5rem; }
			.logout-icon {
				display: inline-flex;
				align-items: center;
				justify-content: center;
				width: 64px;
				height: 64px;
				background: #f3f4f6;
				border-radius: 50%;
				margin-bottom: 1rem;
				color: #6b7280;
			}
			.logout-title {
				font-size: 1.5rem;
				font-weight: 700;
				color: #1e293b;
				margin: 0 0 0.5rem 0;
			}
			.logout-message {
				color: #64748b;
				font-size: 0.875rem;
				margin: 0 0 1.5rem 0;
			}
			.spinner-container { display: flex; justify-content: center; }
			.redirect-message { font-size: 0.875rem; color: #94a3b8; margin: 0; }
			.logout-footer { margin-top: 1rem; }
			.login-link {
				font-size: 0.875rem;
				color: #189AB4;
				text-decoration: none;
				transition: color 0.2s;
			}
			.login-link:hover { color: #0284c7; text-decoration: underline; }
		</style>
	`;
}
