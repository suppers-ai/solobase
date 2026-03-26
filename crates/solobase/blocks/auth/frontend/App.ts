import { html, api, login, authState, checkAuth, isAuthenticated, authLoading, toasts, ToastContainer } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';

// ─── Change Password Form ───────────────────────────────────────────
function ChangePasswordForm() {
	const [currentPassword, setCurrentPassword] = useState('');
	const [newPassword, setNewPassword] = useState('');
	const [confirmPassword, setConfirmPassword] = useState('');
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const [success, setSuccess] = useState(false);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		setError(null);

		if (newPassword.length < 8) {
			setError('New password must be at least 8 characters');
			return;
		}
		if (newPassword !== confirmPassword) {
			setError('New passwords do not match');
			return;
		}

		setLoading(true);
		try {
			const res = await api.post('/auth/change-password', {
				current_password: currentPassword,
				new_password: newPassword,
			});
			if (res.error) {
				const msg = typeof res.error === 'string' ? res.error : res.error.message;
				setError(msg || 'Failed to change password');
			} else {
				setSuccess(true);
				toasts.success('Password changed successfully');
			}
		} catch (err: any) {
			setError(err.message || 'Failed to change password');
		}
		setLoading(false);
	}

	const labelStyle = { display: 'block', fontSize: '0.813rem', fontWeight: 500, color: 'var(--text-primary, #1e293b)', marginBottom: '0.375rem' };
	const inputStyle = { width: '100%', padding: '0.625rem 0.75rem', border: '1px solid var(--border-color, #e2e8f0)', borderRadius: '8px', fontSize: '0.875rem', outline: 'none', boxSizing: 'border-box' };

	return html`
		<div style=${{ minHeight: '100vh', display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'var(--bg-secondary, #f8fafc)' }}>
			<div style=${{ width: '100%', maxWidth: '400px', padding: '2rem' }}>
				<div style=${{ background: 'white', border: '1px solid var(--border-color, #e2e8f0)', borderRadius: '12px', padding: '2rem' }}>
					<div style=${{ display: 'flex', flexDirection: 'column', alignItems: 'center', marginBottom: '1.5rem' }}>
						<img src="/images/logo_long.png" alt="Solobase" style=${{ height: '36px', width: 'auto', marginBottom: '0.75rem' }} />
						<p style=${{ fontSize: '0.875rem', color: 'var(--text-secondary, #64748b)', margin: 0 }}>Change your password</p>
					</div>

					${success ? html`
						<div style=${{ background: '#f0fdf4', border: '1px solid #bbf7d0', borderRadius: '8px', padding: '1rem', textAlign: 'center' }}>
							<p style=${{ fontSize: '0.875rem', color: '#16a34a', margin: '0 0 1rem 0', fontWeight: 500 }}>Password changed successfully!</p>
							<a href="/blocks/dashboard/frontend/#settings"
								style=${{ fontSize: '0.813rem', color: '#fe6627', fontWeight: 600, textDecoration: 'none' }}>
								Back to Settings
							</a>
						</div>
					` : html`
						${error ? html`
							<div style=${{ background: '#fef2f2', border: '1px solid #fecaca', borderRadius: '8px', padding: '0.75rem 1rem', marginBottom: '1rem', fontSize: '0.813rem', color: '#dc2626' }}>
								${error}
							</div>
						` : null}

						<form onSubmit=${handleSubmit}>
							<div style=${{ marginBottom: '1rem' }}>
								<label style=${labelStyle}>Current Password</label>
								<input type="password" value=${currentPassword}
									onInput=${(e: any) => setCurrentPassword(e.target.value)}
									placeholder="Enter your current password" required
									style=${inputStyle} />
							</div>
							<div style=${{ marginBottom: '1rem' }}>
								<label style=${labelStyle}>New Password</label>
								<input type="password" value=${newPassword}
									onInput=${(e: any) => setNewPassword(e.target.value)}
									placeholder="Min 8 characters" required minlength="8"
									style=${inputStyle} />
							</div>
							<div style=${{ marginBottom: '1.5rem' }}>
								<label style=${labelStyle}>Confirm New Password</label>
								<input type="password" value=${confirmPassword}
									onInput=${(e: any) => setConfirmPassword(e.target.value)}
									placeholder="Repeat new password" required minlength="8"
									style=${inputStyle} />
							</div>
							<button type="submit" disabled=${loading}
								style=${{
									width: '100%', padding: '0.75rem', background: 'var(--primary-color, #fe6627)',
									color: 'white', border: 'none', borderRadius: '8px', fontSize: '0.875rem',
									fontWeight: 600, cursor: loading ? 'not-allowed' : 'pointer', opacity: loading ? 0.7 : 1
								}}>
								${loading ? 'Changing...' : 'Change Password'}
							</button>
						</form>
						<div style=${{ textAlign: 'center', marginTop: '1rem' }}>
							<a href="/blocks/dashboard/frontend/#settings"
								style=${{ fontSize: '0.813rem', color: 'var(--text-secondary, #64748b)', textDecoration: 'none' }}>
								Cancel
							</a>
						</div>
					`}
				</div>
			</div>
			<${ToastContainer} />
		</div>
	`;
}

// ─── Login Form ─────────────────────────────────────────────────────
function LoginForm() {
	const [email, setEmail] = useState('');
	const [password, setPassword] = useState('');
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		setError(null);
		setLoading(true);
		const result = await login(email, password);
		setLoading(false);
		if (result.ok) {
			const params = new URLSearchParams(window.location.search);
			window.location.href = params.get('redirect') || '/blocks/dashboard/frontend/';
		} else {
			setError(result.error || 'Invalid email or password');
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
	const [page, setPage] = useState('login');

	useEffect(() => {
		const params = new URLSearchParams(window.location.search);
		const requestedPage = params.get('page') || 'login';
		setPage(requestedPage);

		checkAuth().then(authenticated => {
			if (requestedPage === 'change-password') {
				// Change password requires auth — redirect to login if not authenticated
				if (!authenticated) {
					window.location.href = '/blocks/auth/frontend/?redirect=' + encodeURIComponent(window.location.href);
				} else {
					setChecked(true);
				}
			} else {
				// Login page — redirect to dashboard if already authenticated
				if (authenticated) {
					const redirect = params.get('redirect') || '/blocks/dashboard/frontend/';
					window.location.href = redirect;
				} else {
					setChecked(true);
				}
			}
		});
	}, []);

	if (!checked) {
		return html`<div style=${{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh' }}>Loading...</div>`;
	}

	if (page === 'change-password') {
		return html`<${ChangePasswordForm} />`;
	}

	return html`<${LoginForm} />`;
}
