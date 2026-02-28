import { html } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { CheckCircle, ShoppingBag, ArrowRight, Package } from 'lucide-preact';

export function SuccessPage() {
	const [sessionId, setSessionId] = useState('');

	useEffect(() => {
		const params = new URLSearchParams(window.location.search);
		setSessionId(params.get('session_id') || '');
		localStorage.removeItem('solobase_cart');
	}, []);

	return html`
		<div class="page-container">
			<div class="content-wrapper">
				<div class="success-card">
					<div class="success-icon">
						<${CheckCircle} size=${64} />
					</div>
					<h1>Payment Successful!</h1>
					<p class="subtitle">
						Thank you for your purchase. Your order has been confirmed and you'll receive a confirmation email shortly.
					</p>
					${sessionId ? html`
						<div class="session-info">
							<span class="session-label">Order Reference:</span>
							<code class="session-id">${sessionId.substring(0, 20)}...</code>
						</div>
					` : null}
					<div class="actions">
						<button class="btn btn-primary" onClick=${() => { window.location.href = '/profile/products'; }}>
							<${Package} size=${20} />
							View My Products
						</button>
						<button class="btn btn-secondary" onClick=${() => { window.location.href = '/products'; }}>
							<${ShoppingBag} size=${20} />
							Continue Shopping
							<${ArrowRight} size=${16} />
						</button>
					</div>
					<div class="info-section">
						<h3>What's next?</h3>
						<ul>
							<li>A confirmation email has been sent to your email address</li>
							<li>You can view your purchases anytime from your profile</li>
							<li>If you have any questions, please contact our support team</li>
						</ul>
					</div>
				</div>
			</div>
		</div>
		<style>
			.page-container {
				min-height: 100vh;
				background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
				display: flex;
				align-items: center;
				justify-content: center;
				padding: 2rem 1rem;
			}
			.content-wrapper { width: 100%; max-width: 500px; }
			.success-card {
				background: white;
				border-radius: 16px;
				box-shadow: 0 10px 40px rgba(0, 0, 0, 0.15);
				padding: 3rem 2rem;
				text-align: center;
			}
			.success-icon {
				width: 100px;
				height: 100px;
				margin: 0 auto 1.5rem;
				background: linear-gradient(135deg, #10b981 0%, #059669 100%);
				border-radius: 50%;
				display: flex;
				align-items: center;
				justify-content: center;
				color: white;
				animation: pop 0.5s ease-out;
			}
			@keyframes pop {
				0% { transform: scale(0); opacity: 0; }
				50% { transform: scale(1.1); }
				100% { transform: scale(1); opacity: 1; }
			}
			h1 { font-size: 1.75rem; font-weight: 700; color: #1f2937; margin: 0 0 0.75rem 0; }
			.subtitle { font-size: 1rem; color: #6b7280; margin: 0 0 2rem 0; line-height: 1.6; }
			.session-info {
				display: inline-flex; align-items: center; gap: 0.5rem;
				padding: 0.75rem 1rem; background: #f3f4f6; border-radius: 8px; margin-bottom: 2rem;
			}
			.session-label { font-size: 0.875rem; color: #6b7280; }
			.session-id { font-family: monospace; font-size: 0.813rem; color: #374151; background: #e5e7eb; padding: 0.25rem 0.5rem; border-radius: 4px; }
			.actions { display: flex; flex-direction: column; gap: 0.75rem; margin-bottom: 2rem; }
			.btn {
				display: inline-flex; align-items: center; justify-content: center; gap: 0.5rem;
				padding: 1rem 1.5rem; border-radius: 8px; font-weight: 600; cursor: pointer;
				transition: all 0.2s; border: none; font-size: 0.938rem;
			}
			.btn-primary { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; }
			.btn-primary:hover { opacity: 0.9; transform: translateY(-1px); }
			.btn-secondary { background: #f3f4f6; color: #374151; border: 1px solid #e5e7eb; }
			.btn-secondary:hover { background: #e5e7eb; transform: translateY(-1px); }
			.info-section {
				text-align: left; padding: 1.5rem; background: #f0f9ff;
				border: 1px solid #bae6fd; border-radius: 12px;
			}
			.info-section h3 { font-size: 0.938rem; font-weight: 600; color: #0369a1; margin: 0 0 0.75rem 0; }
			.info-section ul { margin: 0; padding-left: 1.25rem; }
			.info-section li { font-size: 0.875rem; color: #0c4a6e; margin: 0.5rem 0; line-height: 1.5; }
			@media (max-width: 640px) {
				.page-container { padding: 1rem; }
				.success-card { padding: 2rem 1.5rem; }
				.success-icon { width: 80px; height: 80px; }
				h1 { font-size: 1.5rem; }
			}
		</style>
	`;
}
