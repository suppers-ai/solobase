import { html, login, checkAuth, authState, LoadingSpinner } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { Eye, EyeOff, Mail, Lock, LogIn } from 'lucide-preact';

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

export function LoginPage() {
	const [email, setEmail] = useState('');
	const [password, setPassword] = useState('');
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState('');
	const [showPassword, setShowPassword] = useState(false);
	const [rememberMe, setRememberMe] = useState(false);
	const [availableProviders, setAvailableProviders] = useState<string[]>([]);
	const [isPopupMode, setIsPopupMode] = useState(false);

	const params = new URLSearchParams(window.location.search);
	const redirectTo = params.get('redirect');

	useEffect(() => {
		const urlParams = new URLSearchParams(window.location.search);
		setIsPopupMode(urlParams.get('popup') === 'true');

		fetch('/api/auth/oauth/providers')
			.then(r => r.ok ? r.json() : null)
			.then(data => {
				if (data?.providers) setAvailableProviders(data.providers);
			})
			.catch(() => {});
	}, []);

	function closePopupWithSuccess(): boolean {
		if (!isPopupMode) return false;
		if (window.opener) {
			try {
				window.opener.postMessage({ type: 'auth-success' }, '*');
			} catch (e) { /* ignore */ }
		}
		window.close();
		return true;
	}

	async function handleSubmit(e: Event) {
		e.preventDefault();
		setLoading(true);
		setError('');

		const success = await login(email, password);

		if (success) {
			if (closePopupWithSuccess()) return;

			if (redirectTo && isValidRedirectUrl(redirectTo)) {
				window.location.href = redirectTo;
			} else {
				window.location.href = '/profile';
			}
		} else {
			const state = authState.value;
			setError(state.error || 'Invalid email or password');
			setLoading(false);
		}
	}

	function handleOAuthLogin(provider: string) {
		setLoading(true);
		setError('');

		const callbackUrl = `${window.location.origin}/oauth/callback`;
		let oauthUrl = `/api/auth/oauth/login?provider=${provider}&callback=${encodeURIComponent(callbackUrl)}`;

		if (!isPopupMode && redirectTo && isValidRedirectUrl(redirectTo)) {
			oauthUrl += `&redirect=${encodeURIComponent(redirectTo)}`;
		}

		if (isPopupMode) {
			window.location.href = oauthUrl;
			return;
		}

		const popup = window.open(oauthUrl, 'oauth-login', 'width=600,height=700,scrollbars=yes,resizable=yes');
		if (!popup) {
			setLoading(false);
			setError('Popup blocked. Please allow popups for this site and try again.');
			return;
		}

		const handleMessage = (event: MessageEvent) => {
			if (event.origin !== window.location.origin) return;

			if (event.data?.type === 'oauth-success') {
				checkAuth().then(success => {
					if (success) {
						const redirect = event.data.redirect;
						if (redirect && isValidRedirectUrl(redirect)) {
							window.location.href = redirect;
						} else {
							window.location.href = '/profile';
						}
					} else {
						setError('Authentication verification failed. Please try again.');
						setLoading(false);
					}
				});
				window.removeEventListener('message', handleMessage);
			} else if (event.data?.type === 'oauth-error') {
				setError(event.data.error || 'OAuth authentication failed');
				setLoading(false);
				window.removeEventListener('message', handleMessage);
			}
		};

		window.addEventListener('message', handleMessage);

		const checkClosed = setInterval(() => {
			if (popup.closed) {
				clearInterval(checkClosed);
				window.removeEventListener('message', handleMessage);
				if (loading) {
					setError('Authentication was cancelled');
					setLoading(false);
				}
			}
		}, 1000);
	}

	return html`
		<div class="login-page">
			<div class="login-container">
				<div class="login-logo">
					<img src="/logo_long.png" alt="Solobase" class="logo-image" />
					<p class="login-subtitle">
						${isPopupMode ? 'Sign in to continue' : 'Welcome back! Please login to your account.'}
					</p>
				</div>

				${error ? html`
					<div class="login-error">
						<svg width="20" height="20" viewBox="0 0 20 20" fill="currentColor">
							<path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd"/>
						</svg>
						${error}
					</div>
				` : null}

				${availableProviders.length > 0 ? html`
					<div class="oauth-buttons">
						${availableProviders.includes('google') ? html`
							<button type="button" class="oauth-button google" onClick=${() => handleOAuthLogin('google')} disabled=${loading}>
								<svg viewBox="0 0 24 24" width="20" height="20">
									<path fill="#4285F4" d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"/>
									<path fill="#34A853" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"/>
									<path fill="#FBBC05" d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"/>
									<path fill="#EA4335" d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"/>
								</svg>
								<span>Continue with Google</span>
							</button>
						` : null}
						${availableProviders.includes('microsoft') ? html`
							<button type="button" class="oauth-button microsoft" onClick=${() => handleOAuthLogin('microsoft')} disabled=${loading}>
								<svg viewBox="0 0 24 24" width="20" height="20">
									<path fill="#f25022" d="M1 1h10v10H1z"/>
									<path fill="#00a4ef" d="M13 1h10v10H13z"/>
									<path fill="#7fba00" d="M1 13h10v10H1z"/>
									<path fill="#ffb900" d="M13 13h10v10H13z"/>
								</svg>
								<span>Continue with Microsoft</span>
							</button>
						` : null}
					</div>
					<div class="oauth-divider"><span>Or sign in with email</span></div>
				` : null}

				<form onSubmit=${handleSubmit} class="login-form">
					<div class="form-group">
						<label for="email" class="form-label">
							<${Mail} size=${16} />
							Email Address
						</label>
						<input
							id="email"
							type="email"
							class="form-input"
							value=${email}
							onInput=${(e: Event) => setEmail((e.target as HTMLInputElement).value)}
							placeholder="you@example.com"
							required
							disabled=${loading}
							autocomplete="email"
						/>
					</div>

					<div class="form-group">
						<label for="password" class="form-label">
							<${Lock} size=${16} />
							Password
						</label>
						<div class="password-input-container">
							<input
								id="password"
								type=${showPassword ? 'text' : 'password'}
								class="form-input with-icon"
								value=${password}
								onInput=${(e: Event) => setPassword((e.target as HTMLInputElement).value)}
								placeholder="Enter your password"
								required
								disabled=${loading}
								autocomplete="current-password"
							/>
							<button
								type="button"
								class="password-toggle"
								onClick=${() => setShowPassword((v: boolean) => !v)}
								tabindex=${-1}
								aria-label=${showPassword ? 'Hide password' : 'Show password'}
							>
								${showPassword ? html`<${EyeOff} size=${20} />` : html`<${Eye} size=${20} />`}
							</button>
						</div>
					</div>

					${!isPopupMode ? html`
						<div class="form-actions" style=${{ justifyContent: 'space-between', borderTop: 'none', paddingTop: 0, marginTop: '0.5rem' }}>
							<label class="remember-me" style=${{ display: 'flex', alignItems: 'center', gap: '0.5rem', cursor: 'pointer', fontSize: '0.875rem' }}>
								<input
									type="checkbox"
									checked=${rememberMe}
									onChange=${(e: Event) => setRememberMe((e.target as HTMLInputElement).checked)}
								/>
								<span>Remember me</span>
							</label>
						</div>
					` : null}

					<button type="submit" class="login-button" disabled=${loading}>
						${loading ? html`
							<${LoadingSpinner} size="sm" color="white" />
							<span>Logging in...</span>
						` : html`
							<${LogIn} size=${20} />
							<span>Login</span>
						`}
					</button>
				</form>

				${!isPopupMode ? html`
					<div class="signup-link" style=${{ textAlign: 'center', marginTop: '1.5rem', fontSize: '0.875rem', color: '#6b7280' }}>
						Don't have an account?
						<a href="/signup" style=${{ color: '#189AB4', textDecoration: 'none', fontWeight: '600', marginLeft: '0.25rem' }}>Sign up</a>
					</div>
				` : null}
			</div>
		</div>
	`;
}
