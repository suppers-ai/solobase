import { html, api, LoadingSpinner, EmptyState, formatPrice } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { ArrowLeft, ShoppingCart, Trash2, Plus, Minus, CreditCard, AlertCircle } from 'lucide-preact';

interface Product {
	id: string;
	name: string;
	description?: string;
	price?: number;
	currency?: string;
	status: string;
}

interface CartItem {
	product: Product;
	quantity: number;
}

interface PurchaseResponse {
	id: string;
	checkoutUrl?: string;
	status: string;
}

export function CheckoutPage() {
	const [cart, setCart] = useState<CartItem[]>([]);
	const [email, setEmail] = useState('');
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState('');
	const [isLoggedIn, setIsLoggedIn] = useState(false);

	useEffect(() => {
		// Load cart from localStorage
		try {
			const saved = localStorage.getItem('solobase_cart');
			if (saved) setCart(JSON.parse(saved));
		} catch { /* ignore */ }

		// Check if user is logged in and get their email
		(async () => {
			try {
				const response = await api.get<{ user: any }>('/auth/me');
				if (response?.user) {
					setIsLoggedIn(true);
					setEmail(response.user.email || '');
				}
			} catch { /* not logged in */ }
		})();
	}, []);

	function saveCart(c: CartItem[]) {
		localStorage.setItem('solobase_cart', JSON.stringify(c));
	}

	function removeFromCart(productId: string) {
		setCart(prev => {
			const next = prev.filter(item => item.product.id !== productId);
			saveCart(next);
			return next;
		});
	}

	function updateQuantity(productId: string, delta: number) {
		setCart(prev => {
			const next = prev.map(item =>
				item.product.id === productId ? { ...item, quantity: Math.max(1, item.quantity + delta) } : item
			);
			saveCart(next);
			return next;
		});
	}

	function getCartTotal(): number {
		return cart.reduce((sum, item) => sum + (item.product.price || 0) * item.quantity, 0);
	}

	function getCartItemCount(): number {
		return cart.reduce((sum, item) => sum + item.quantity, 0);
	}

	function isValidEmail(e: string): boolean {
		return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(e);
	}

	async function handleCheckout() {
		setError('');

		if (!email.trim()) {
			setError('Please enter your email address');
			return;
		}
		if (!isValidEmail(email)) {
			setError('Please enter a valid email address');
			return;
		}
		if (cart.length === 0) {
			setError('Your cart is empty');
			return;
		}

		setLoading(true);

		try {
			const items = cart.map(item => ({
				productId: item.product.id,
				quantity: item.quantity
			}));

			const purchaseRequest = {
				items,
				customerEmail: email,
				successUrl: `${window.location.origin}/products/success?session_id={CHECKOUT_SESSION_ID}`,
				cancelUrl: `${window.location.origin}/products/checkout`,
				paymentMethods: ['card']
			};

			const response = await api.post<PurchaseResponse>('/ext/products/purchase', purchaseRequest);

			if (response.checkoutUrl) {
				localStorage.removeItem('solobase_cart');
				window.location.href = response.checkoutUrl;
			} else {
				setError('Failed to create checkout session. Please try again.');
				setLoading(false);
			}
		} catch (err: any) {
			setError(err.message || 'Failed to create checkout session. Please try again.');
			setLoading(false);
		}
	}

	return html`
		<div class="page-container">
			<div class="content-wrapper">
				<div class="checkout-card">
					<div class="header">
						<button class="back-button" onClick=${() => { window.location.href = '/products'; }}>
							<${ArrowLeft} size=${20} />
							<span>Continue Shopping</span>
						</button>
						<h1>Checkout</h1>
					</div>

					${cart.length === 0 ? html`
						<${EmptyState}
							icon=${ShoppingCart}
							title="Your cart is empty"
							description="Add some products to your cart before checking out."
						>
							<button class="btn btn-primary" onClick=${() => { window.location.href = '/products'; }}>
								Browse Products
							</button>
						<//>
					` : html`
						<div class="checkout-content">
							<div class="cart-section">
								<h2>Cart Items (${getCartItemCount()})</h2>
								<div class="cart-items">
									${cart.map(item => html`
										<div class="cart-item" key=${item.product.id}>
											<div class="item-info">
												<h3>${item.product.name}</h3>
												${item.product.description ? html`<p class="item-description">${item.product.description}</p>` : null}
												<div class="item-price">${formatPrice(item.product.price, item.product.currency)}</div>
											</div>
											<div class="item-controls">
												<div class="quantity-controls">
													<button class="qty-btn" onClick=${() => updateQuantity(item.product.id, -1)}>
														<${Minus} size=${16} />
													</button>
													<span class="qty-value">${item.quantity}</span>
													<button class="qty-btn" onClick=${() => updateQuantity(item.product.id, 1)}>
														<${Plus} size=${16} />
													</button>
												</div>
												<div class="item-total">${formatPrice((item.product.price || 0) * item.quantity, item.product.currency)}</div>
												<button class="remove-btn" onClick=${() => removeFromCart(item.product.id)}>
													<${Trash2} size=${18} />
												</button>
											</div>
										</div>
									`)}
								</div>
							</div>

							<div class="summary-section">
								<h2>Order Summary</h2>
								<div class="summary-details">
									<div class="summary-row">
										<span>Subtotal</span>
										<span>${formatPrice(getCartTotal())}</span>
									</div>
									<div class="summary-row total">
										<span>Total</span>
										<span>${formatPrice(getCartTotal())}</span>
									</div>
								</div>

								<div class="email-section">
									<label for="email">Email Address</label>
									<input
										type="email"
										id="email"
										value=${email}
										onInput=${(e: Event) => setEmail((e.target as HTMLInputElement).value)}
										placeholder="Enter your email"
										disabled=${isLoggedIn}
										class=${isLoggedIn ? 'disabled' : ''}
									/>
									<p class="email-hint">${isLoggedIn ? 'Using your account email' : "We'll send your receipt to this email"}</p>
								</div>

								${error ? html`
									<div class="error-message">
										<${AlertCircle} size=${16} />
										<span>${error}</span>
									</div>
								` : null}

								<button
									class="btn btn-checkout"
									onClick=${handleCheckout}
									disabled=${loading || cart.length === 0}
								>
									${loading ? html`
										<${LoadingSpinner} size="sm" color="white" />
										Processing...
									` : html`
										<${CreditCard} size=${20} />
										Pay ${formatPrice(getCartTotal())}
									`}
								</button>

								<p class="secure-notice">
									<${CreditCard} size=${14} />
									Secured by Stripe
								</p>
							</div>
						</div>
					`}
				</div>
			</div>
		</div>
		<style>
			.page-container { min-height: 100vh; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); display: flex; align-items: flex-start; justify-content: center; padding: 2rem 1rem; }
			.content-wrapper { width: 100%; max-width: 900px; }
			.checkout-card { background: white; border-radius: 16px; box-shadow: 0 10px 40px rgba(0, 0, 0, 0.15); padding: 2rem; }
			.header { margin-bottom: 2rem; }
			.back-button { display: inline-flex; align-items: center; gap: 0.5rem; color: #667eea; background: none; border: none; cursor: pointer; font-size: 0.875rem; padding: 0; margin-bottom: 1rem; }
			.back-button:hover { text-decoration: underline; }
			h1 { font-size: 1.5rem; font-weight: 600; color: #1f2937; margin: 0; }
			h2 { font-size: 1rem; font-weight: 600; color: #374151; margin: 0 0 1rem 0; }
			.checkout-content { display: grid; grid-template-columns: 1fr 350px; gap: 2rem; }
			.cart-section { padding-right: 2rem; border-right: 1px solid #e5e7eb; }
			.cart-items { display: flex; flex-direction: column; gap: 1rem; }
			.cart-item { display: flex; justify-content: space-between; align-items: flex-start; padding: 1.25rem; background: #f9fafb; border-radius: 12px; border: 1px solid #e5e7eb; }
			.item-info { flex: 1; }
			.item-info h3 { font-size: 1rem; font-weight: 600; color: #1f2937; margin: 0 0 0.25rem 0; }
			.item-description { font-size: 0.813rem; color: #6b7280; margin: 0 0 0.5rem 0; }
			.item-price { font-size: 0.875rem; color: #667eea; font-weight: 500; }
			.item-controls { display: flex; align-items: center; gap: 1rem; }
			.quantity-controls { display: flex; align-items: center; gap: 0.5rem; }
			.qty-btn { width: 28px; height: 28px; display: flex; align-items: center; justify-content: center; background: #e5e7eb; border: none; border-radius: 6px; cursor: pointer; transition: background 0.2s; }
			.qty-btn:hover { background: #d1d5db; }
			.qty-value { min-width: 24px; text-align: center; font-weight: 600; font-size: 0.875rem; }
			.item-total { font-weight: 600; color: #1f2937; min-width: 80px; text-align: right; }
			.remove-btn { width: 36px; height: 36px; display: flex; align-items: center; justify-content: center; background: none; border: none; color: #9ca3af; cursor: pointer; border-radius: 8px; transition: all 0.2s; }
			.remove-btn:hover { background: #fee2e2; color: #dc2626; }
			.summary-details { background: #f9fafb; border-radius: 12px; padding: 1.25rem; margin-bottom: 1.5rem; }
			.summary-row { display: flex; justify-content: space-between; padding: 0.5rem 0; font-size: 0.875rem; color: #6b7280; }
			.summary-row.total { border-top: 1px solid #e5e7eb; margin-top: 0.5rem; padding-top: 1rem; font-size: 1.25rem; font-weight: 700; color: #1f2937; }
			.email-section { margin-bottom: 1.5rem; }
			.email-section label { display: block; font-size: 0.875rem; font-weight: 500; color: #374151; margin-bottom: 0.5rem; }
			.email-section input { width: 100%; padding: 0.75rem 1rem; border: 1px solid #d1d5db; border-radius: 8px; font-size: 0.875rem; transition: all 0.2s; box-sizing: border-box; }
			.email-section input:focus { outline: none; border-color: #667eea; box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1); }
			.email-section input.disabled { background: #f9fafb; color: #6b7280; cursor: not-allowed; }
			.email-hint { font-size: 0.75rem; color: #9ca3af; margin: 0.5rem 0 0 0; }
			.error-message { display: flex; align-items: center; gap: 0.5rem; padding: 0.75rem 1rem; background: #fef2f2; border: 1px solid #fecaca; border-radius: 8px; color: #991b1b; font-size: 0.875rem; margin-bottom: 1rem; }
			.btn { display: inline-flex; align-items: center; justify-content: center; gap: 0.5rem; padding: 1rem 1.5rem; border-radius: 8px; font-weight: 600; cursor: pointer; transition: all 0.2s; border: none; width: 100%; font-size: 1rem; }
			.btn-primary { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; }
			.btn-primary:hover { opacity: 0.9; transform: translateY(-1px); }
			.btn-checkout { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; }
			.btn-checkout:hover:not(:disabled) { opacity: 0.9; transform: translateY(-1px); }
			.btn-checkout:disabled { opacity: 0.7; cursor: not-allowed; transform: none; }
			.secure-notice { display: flex; align-items: center; justify-content: center; gap: 0.5rem; margin-top: 1rem; font-size: 0.75rem; color: #9ca3af; }
			@media (max-width: 768px) {
				.page-container { padding: 1rem; }
				.checkout-card { padding: 1.5rem; }
				.checkout-content { grid-template-columns: 1fr; }
				.cart-section { padding-right: 0; border-right: none; border-bottom: 1px solid #e5e7eb; padding-bottom: 2rem; }
				.cart-item { flex-direction: column; gap: 1rem; }
				.item-controls { width: 100%; justify-content: space-between; }
			}
		</style>
	`;
}
