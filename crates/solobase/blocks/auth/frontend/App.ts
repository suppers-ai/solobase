import { html, login, authState, checkAuth, isAuthenticated, authLoading } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';

function LoginForm() {
	const [email, setEmail] = useState('');
	const [password, setPassword] = useState('');
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		setError(null);
		setLoading(true);
		const ok = await login(email, password);
		setLoading(false);
		if (ok) {
			const params = new URLSearchParams(window.location.search);
			window.location.href = params.get('redirect') || '/blocks/dashboard/frontend/';
		} else {
			setError(authState.value.error || 'Invalid email or password');
		}
	}

	return html`
		<div style=${{ minHeight: '100vh', display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'var(--bg-secondary, #f8fafc)' }}>
			<div style=${{ width: '100%', maxWidth: '400px', padding: '2rem' }}>
				<div style=${{ background: 'white', border: '1px solid var(--border-color, #e2e8f0)', borderRadius: '12px', padding: '2rem' }}>
					<div style=${{ display: 'flex', flexDirection: 'column', alignItems: 'center', marginBottom: '1.5rem' }}>
						<img src="/images/logo_long.png" alt="Solobase" style=${{ height: '36px', width: 'auto', marginBottom: '0.75rem' }} />
						<p style=${{ fontSize: '0.875rem', color: 'var(--text-secondary, #64748b)', margin: 0 }}>Sign in to your account</p>
					</div>

					${error ? html`
						<div style=${{ background: '#fef2f2', border: '1px solid #fecaca', borderRadius: '8px', padding: '0.75rem 1rem', marginBottom: '1rem', fontSize: '0.813rem', color: '#dc2626' }}>
							${error}
						</div>
					` : null}

					<form onSubmit=${handleSubmit}>
						<div style=${{ marginBottom: '1rem' }}>
							<label style=${{ display: 'block', fontSize: '0.813rem', fontWeight: 500, color: 'var(--text-primary, #1e293b)', marginBottom: '0.375rem' }}>Email</label>
							<input
								type="email"
								value=${email}
								onInput=${(e: any) => setEmail(e.target.value)}
								placeholder="admin@example.com"
								required
								style=${{ width: '100%', padding: '0.625rem 0.75rem', border: '1px solid var(--border-color, #e2e8f0)', borderRadius: '8px', fontSize: '0.875rem', outline: 'none', boxSizing: 'border-box' }}
							/>
						</div>
						<div style=${{ marginBottom: '1.5rem' }}>
							<label style=${{ display: 'block', fontSize: '0.813rem', fontWeight: 500, color: 'var(--text-primary, #1e293b)', marginBottom: '0.375rem' }}>Password</label>
							<input
								type="password"
								value=${password}
								onInput=${(e: any) => setPassword(e.target.value)}
								placeholder="Enter your password"
								required
								style=${{ width: '100%', padding: '0.625rem 0.75rem', border: '1px solid var(--border-color, #e2e8f0)', borderRadius: '8px', fontSize: '0.875rem', outline: 'none', boxSizing: 'border-box' }}
							/>
						</div>
						<button
							type="submit"
							disabled=${loading}
							style=${{
								width: '100%',
								padding: '0.75rem',
								background: 'var(--primary-color, #fe6627)',
								color: 'white',
								border: 'none',
								borderRadius: '8px',
								fontSize: '0.875rem',
								fontWeight: 600,
								cursor: loading ? 'not-allowed' : 'pointer',
								opacity: loading ? 0.7 : 1
							}}
						>
							${loading ? 'Signing in...' : 'Sign In'}
						</button>
					</form>
				</div>
			</div>
		</div>
	`;
}

export function App() {
	const [checked, setChecked] = useState(false);

	useEffect(() => {
		checkAuth().then(authenticated => {
			if (authenticated) {
				window.location.href = '/blocks/dashboard/frontend/';
			} else {
				setChecked(true);
			}
		});
	}, []);

	if (!checked) {
		return html`<div style=${{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh' }}>Loading...</div>`;
	}

	return html`<${LoginForm} />`;
}
