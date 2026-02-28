import { html } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { login, authState, logout } from '@solobase/ui';
import { Eye, EyeOff, Mail, Lock, LogIn } from 'lucide-preact';


export function LoginPage() {
	const [email, setEmail] = useState('');
	const [password, setPassword] = useState('');
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState('');
	const [showPassword, setShowPassword] = useState(false);
	const [rememberMe, setRememberMe] = useState(false);
	const [availableProviders, setAvailableProviders] = useState<string[]>([]);

	useEffect(() => {
		// If already authenticated as admin, redirect
		const state = authState.value;
		if (state.user && state.roles?.includes('admin')) {
			window.location.href = '/admin';
		}

		// Fetch available OAuth providers
		fetch('/api/auth/oauth/providers')
			.then(r => r.ok ? r.json() : null)
			.then(data => {
				if (data?.providers) setAvailableProviders(data.providers);
			})
			.catch(() => {});
	}, []);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		setLoading(true);
		setError('');

		const success = await login(email, password);

		if (success) {
			const state = authState.value;
			if (state.roles?.includes('admin')) {
				window.location.href = '/admin';
			} else {
				setError('Access denied. Admin privileges required.');
				logout();
				setLoading(false);
			}
		} else {
			const state = authState.value;
			setError(state.error || 'Invalid email or password');
			setLoading(false);
		}
	}

	function handleOAuthLogin(provider: string) {
		window.location.href = `/api/auth/oauth/${provider}/login`;
	}

	return html`
		<div class="login-page">
			<div class="login-container">
				<div class="login-logo">
					<img src="/logo_long.png" alt="Solobase Admin" class="logo-image" />
					<p class="login-subtitle">Administrator Login</p>
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
							<button
								type="button"
								class="oauth-button google"
								onClick=${() => handleOAuthLogin('google')}
								disabled=${loading}
							>
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
							<button
								type="button"
								class="oauth-button microsoft"
								onClick=${() => handleOAuthLogin('microsoft')}
								disabled=${loading}
							>
								<svg viewBox="0 0 24 24" width="20" height="20">
									<path fill="#f25022" d="M1 1h10v10H1z"/>
									<path fill="#00a4ef" d="M13 1h10v10H13z"/>
									<path fill="#7fba00" d="M1 13h10v10H1z"/>
									<path fill="#ffb900" d="M13 13h10v10H13z"/>
								</svg>
								<span>Continue with Microsoft</span>
							</button>
						` : null}

						${availableProviders.includes('facebook') ? html`
							<button
								type="button"
								class="oauth-button facebook"
								onClick=${() => handleOAuthLogin('facebook')}
								disabled=${loading}
							>
								<svg viewBox="0 0 24 24" width="20" height="20">
									<path fill="#1877F2" d="M24 12.073c0-6.627-5.373-12-12-12s-12 5.373-12 12c0 5.99 4.388 10.954 10.125 11.854v-8.385H7.078v-3.47h3.047V9.43c0-3.007 1.792-4.669 4.533-4.669 1.312 0 2.686.235 2.686.235v2.953H15.83c-1.491 0-1.956.925-1.956 1.874v2.25h3.328l-.532 3.47h-2.796v8.385C19.612 23.027 24 18.062 24 12.073z"/>
								</svg>
								<span>Continue with Facebook</span>
							</button>
						` : null}
					</div>

					<div class="oauth-divider">
						<span>Or sign in with email</span>
					</div>
				` : null}

				<form onSubmit=${handleSubmit} class="login-form">
					<div class="form-group">
						<label for="email" class="form-label">Email Address</label>
						<div class="form-input-wrapper">
							<span class="input-icon"><${Mail} size=${16} /></span>
							<input
								id="email"
								type="email"
								class="form-input"
								value=${email}
								onInput=${(e: Event) => setEmail((e.target as HTMLInputElement).value)}
								placeholder="admin@example.com"
								required
								disabled=${loading}
								autocomplete="email"
							/>
						</div>
					</div>

					<div class="form-group">
						<label for="password" class="form-label">Password</label>
						<div class="password-input-container">
							<div class="form-input-wrapper" style=${{ flex: 1 }}>
								<span class="input-icon"><${Lock} size=${16} /></span>
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
							</div>
							<button
								type="button"
								class="password-toggle"
								onClick=${() => setShowPassword(v => !v)}
								tabindex=${-1}
								aria-label=${showPassword ? 'Hide password' : 'Show password'}
							>
								${showPassword ? html`<${EyeOff} size=${18} />` : html`<${Eye} size=${18} />`}
							</button>
						</div>
					</div>

					<div class="form-actions" style=${{ justifyContent: 'space-between', borderTop: 'none', paddingTop: 0, marginTop: '0.5rem', marginBottom: '1.25rem' }}>
						<label class="remember-me" style=${{ display: 'flex', alignItems: 'center', gap: '0.5rem', cursor: 'pointer', fontSize: '0.875rem' }}>
							<input
								type="checkbox"
								checked=${rememberMe}
								onChange=${(e: Event) => setRememberMe((e.target as HTMLInputElement).checked)}
							/>
							<span>Remember me</span>
						</label>
					</div>

					<button
						type="submit"
						class="login-button"
						disabled=${loading}
					>
						${loading ? html`
							<div style=${{
								width: '20px',
								height: '20px',
								border: '2px solid rgba(255,255,255,0.3)',
								borderTopColor: 'white',
								borderRadius: '50%',
								animation: 'spin 0.6s linear infinite'
							}} />
							<span>Logging in...</span>
						` : html`
							<${LogIn} size=${20} />
							<span>Login</span>
						`}
					</button>
				</form>
			</div>
		</div>
	`;
}
