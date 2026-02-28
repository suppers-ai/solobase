import { html, LoadingSpinner } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';

function isValidRedirectUrl(url: string): boolean {
	if (!url) return false;
	try {
		if (url.startsWith('/') && !url.startsWith('//')) return true;
		if (url.startsWith('http')) {
			const urlObj = new URL(url);
			return urlObj.origin === window.location.origin;
		}
		return false;
	} catch {
		return false;
	}
}

export function OAuthCallbackPage() {
	const [status, setStatus] = useState<'processing' | 'success' | 'error'>('processing');
	const [message, setMessage] = useState('Processing OAuth authentication...');

	useEffect(() => {
		try {
			const urlParams = new URLSearchParams(window.location.search);
			const success = urlParams.get('success');
			const error = urlParams.get('error');
			const redirectTo = urlParams.get('redirect');

			if (success === 'true') {
				setStatus('success');
				setMessage('Authentication successful! Redirecting...');

				if (window.opener) {
					window.opener.postMessage({ type: 'oauth-success', redirect: redirectTo }, '*');
					window.opener.postMessage({ type: 'auth-success', redirect: redirectTo }, '*');
					window.close();
					return;
				}

				const destination = redirectTo && isValidRedirectUrl(redirectTo) ? redirectTo : '/';
				setTimeout(() => { window.location.href = destination; }, 1000);
			} else {
				setStatus('error');
				const errorMsg = error || 'Authentication failed. Please try again.';
				setMessage(errorMsg);

				if (window.opener) {
					window.opener.postMessage({ type: 'oauth-error', error: errorMsg }, window.location.origin);
					window.close();
					return;
				}

				setTimeout(() => { window.location.href = '/login'; }, 3000);
			}
		} catch (err) {
			console.error('OAuth callback error:', err);
			setStatus('error');
			setMessage('An unexpected error occurred.');

			if (window.opener) {
				window.opener.postMessage({ type: 'oauth-error', error: 'An unexpected error occurred.' }, window.location.origin);
				window.close();
			} else {
				setTimeout(() => { window.location.href = '/login'; }, 3000);
			}
		}
	}, []);

	return html`
		<div class="callback-container">
			<div class="callback-content">
				${status === 'processing' ? html`
					<${LoadingSpinner} size="lg" color="primary" centered=${true} />
					<h2>Processing Authentication</h2>
					<p>${message}</p>
				` : status === 'success' ? html`
					<div class="success-icon">✓</div>
					<h2>Authentication Successful</h2>
					<p>${message}</p>
				` : html`
					<div class="error-icon">✗</div>
					<h2>Authentication Failed</h2>
					<p>${message}</p>
				`}
			</div>
		</div>
		<style>
			.callback-container {
				min-height: 100vh;
				display: flex;
				align-items: center;
				justify-content: center;
				background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
				font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
			}
			.callback-content {
				text-align: center;
				background: white;
				padding: 2rem;
				border-radius: 12px;
				box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1);
				max-width: 400px;
				width: 100%;
				margin: 0 1rem;
			}
			h2 { margin: 1rem 0 0.5rem 0; color: #333; font-weight: 600; }
			p { color: #666; line-height: 1.5; margin-bottom: 0; }
			.success-icon {
				width: 60px; height: 60px; border-radius: 50%; background: #4caf50; color: white;
				display: flex; align-items: center; justify-content: center;
				font-size: 28px; font-weight: bold; margin: 0 auto 1rem;
			}
			.error-icon {
				width: 60px; height: 60px; border-radius: 50%; background: #f44336; color: white;
				display: flex; align-items: center; justify-content: center;
				font-size: 28px; font-weight: bold; margin: 0 auto 1rem;
			}
		</style>
	`;
}
