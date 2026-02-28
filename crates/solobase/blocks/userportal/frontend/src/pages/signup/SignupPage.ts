import { html, api, LoadingSpinner } from '@solobase/ui';
import { useState } from 'preact/hooks';
import { Eye, EyeOff, Mail, Lock, UserPlus, User } from 'lucide-preact';

export function SignupPage() {
	const [email, setEmail] = useState('');
	const [password, setPassword] = useState('');
	const [confirmPassword, setConfirmPassword] = useState('');
	const [fullName, setFullName] = useState('');
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState('');
	const [showPassword, setShowPassword] = useState(false);
	const [showConfirmPassword, setShowConfirmPassword] = useState(false);

	async function handleSignup(e: Event) {
		e.preventDefault();
		setLoading(true);
		setError('');

		if (password !== confirmPassword) {
			setError('Passwords do not match');
			setLoading(false);
			return;
		}

		if (password.length < 8) {
			setError('Password must be at least 8 characters');
			setLoading(false);
			return;
		}

		const nameParts = fullName.trim().split(/\s+/);
		const firstName = nameParts[0] || '';
		const lastName = nameParts.length > 1 ? nameParts.slice(1).join(' ') : '';

		try {
			await api.post('/api/auth/signup', { email, password, firstName, lastName });
			window.location.href = '/login?registered=true';
		} catch (err: any) {
			setError(err.message || 'An error occurred during signup');
			setLoading(false);
		}
	}

	return html`
		<div class="signup-page">
			<div class="signup-container">
				<div class="signup-logo">
					<img src="/logo_long.png" alt="Solobase" class="logo-image" />
					<p class="signup-subtitle">Create your account to get started</p>
				</div>

				${error ? html`
					<div class="signup-error">
						<svg width="20" height="20" viewBox="0 0 20 20" fill="currentColor">
							<path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd"/>
						</svg>
						${error}
					</div>
				` : null}

				<form onSubmit=${handleSignup} class="signup-form">
					<div class="form-group">
						<label for="fullName" class="form-label">
							<${User} size=${16} />
							Full Name
						</label>
						<input
							id="fullName"
							type="text"
							class="form-input"
							value=${fullName}
							onInput=${(e: Event) => setFullName((e.target as HTMLInputElement).value)}
							placeholder="John Doe"
							disabled=${loading}
							autocomplete="name"
						/>
					</div>

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
							placeholder="john@example.com"
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
								placeholder="Min. 8 characters"
								required
								disabled=${loading}
								autocomplete="new-password"
							/>
							<button
								type="button"
								class="password-toggle"
								onClick=${() => setShowPassword((v: boolean) => !v)}
								tabindex=${-1}
							>
								${showPassword ? html`<${EyeOff} size=${20} />` : html`<${Eye} size=${20} />`}
							</button>
						</div>
					</div>

					<div class="form-group">
						<label for="confirmPassword" class="form-label">
							<${Lock} size=${16} />
							Confirm Password
						</label>
						<div class="password-input-container">
							<input
								id="confirmPassword"
								type=${showConfirmPassword ? 'text' : 'password'}
								class="form-input with-icon"
								value=${confirmPassword}
								onInput=${(e: Event) => setConfirmPassword((e.target as HTMLInputElement).value)}
								placeholder="Re-enter your password"
								required
								disabled=${loading}
								autocomplete="new-password"
							/>
							<button
								type="button"
								class="password-toggle"
								onClick=${() => setShowConfirmPassword((v: boolean) => !v)}
								tabindex=${-1}
							>
								${showConfirmPassword ? html`<${EyeOff} size=${20} />` : html`<${Eye} size=${20} />`}
							</button>
						</div>
					</div>

					<div class="terms-section">
						<label class="terms-checkbox">
							<input type="checkbox" required />
							<span>I agree to the <a href="/terms" class="terms-link">Terms of Service</a> and <a href="/privacy" class="terms-link">Privacy Policy</a></span>
						</label>
					</div>

					<button type="submit" class="signup-button" disabled=${loading}>
						${loading ? html`
							<${LoadingSpinner} size="sm" color="white" />
							<span>Creating account...</span>
						` : html`
							<${UserPlus} size=${20} />
							<span>Sign Up</span>
						`}
					</button>
				</form>

				<div class="login-link">
					Already have an account?
					<a href="/login">Login here</a>
				</div>
			</div>
		</div>
		<style>
			.signup-page {
				min-height: 100vh; display: flex; align-items: center; justify-content: center;
				background: #f0f0f0; padding: 1rem;
			}
			.signup-container {
				width: 100%; max-width: 420px; background: white; border: 1px solid #e2e8f0;
				border-radius: 12px; padding: 2.5rem; animation: slideUp 0.4s ease-out;
			}
			@keyframes slideUp {
				from { opacity: 0; transform: translateY(20px); }
				to { opacity: 1; transform: translateY(0); }
			}
			.signup-logo { text-align: center; margin-bottom: 2rem; }
			.logo-image { height: 60px; width: auto; margin: 0 auto 1rem auto; display: block; }
			.signup-subtitle { color: #6b7280; font-size: 0.875rem; margin: 0; }
			.signup-error {
				background: #fee2e2; color: #dc2626; padding: 0.75rem 1rem; border-radius: 8px;
				margin-bottom: 1.5rem; font-size: 0.875rem; display: flex; align-items: center;
				gap: 0.5rem; animation: shake 0.3s ease-in-out;
			}
			@keyframes shake {
				0%, 100% { transform: translateX(0); }
				25% { transform: translateX(-5px); }
				75% { transform: translateX(5px); }
			}
			.signup-form { margin-bottom: 1.5rem; }
			.terms-section { margin-bottom: 1.5rem; }
			.terms-checkbox {
				display: flex; align-items: flex-start; gap: 0.5rem;
				font-size: 0.875rem; color: #6b7280; cursor: pointer;
			}
			.terms-checkbox input[type="checkbox"] { width: 1rem; height: 1rem; margin-top: 0.125rem; cursor: pointer; flex-shrink: 0; }
			.terms-link { color: #3b82f6; text-decoration: none; transition: color 0.2s; }
			.terms-link:hover { color: #2563eb; text-decoration: underline; }
			.signup-button {
				width: 100%; padding: 0.875rem 1.5rem; background: #3b82f6; color: white;
				border: none; border-radius: 8px; font-size: 0.9375rem; font-weight: 600;
				cursor: pointer; transition: all 0.2s; display: flex; align-items: center;
				justify-content: center; gap: 0.5rem;
			}
			.signup-button:hover:not(:disabled) { background: #2563eb; transform: translateY(-1px); }
			.signup-button:disabled { cursor: not-allowed; opacity: 0.7; }
			.login-link { text-align: center; font-size: 0.875rem; color: #6b7280; }
			.login-link a { color: #3b82f6; text-decoration: none; font-weight: 600; transition: color 0.2s; }
			.login-link a:hover { color: #2563eb; text-decoration: underline; }
			@media (max-width: 480px) { .signup-container { padding: 2rem; } }
		</style>
	`;
}
