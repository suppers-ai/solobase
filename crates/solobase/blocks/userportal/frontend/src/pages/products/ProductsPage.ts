import { html, api, LoadingSpinner, EmptyState } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { ShoppingCart, Package, ArrowLeft, Plus, Minus, Trash2 } from 'lucide-preact';

interface Product {
	id: string;
	name: string;
	description?: string;
	price?: number;
	currency?: string;
	status: string;
	imageUrl?: string;
}

interface CartItem {
	product: Product;
	quantity: number;
}

function formatPrice(price?: number, currency?: string): string {
	if (price === undefined || price === null) return 'Free';
	return new Intl.NumberFormat('en-US', { style: 'currency', currency: currency || 'USD' }).format(price / 100);
}

export function ProductsPage() {
	const [products, setProducts] = useState<Product[]>([]);
	const [loading, setLoading] = useState(true);
	const [error, setError] = useState('');
	const [cart, setCart] = useState<CartItem[]>([]);

	function loadCart() {
		try {
			const saved = localStorage.getItem('solobase_cart');
			if (saved) return JSON.parse(saved) as CartItem[];
		} catch { /* ignore */ }
		return [];
	}

	function saveCart(c: CartItem[]) {
		localStorage.setItem('solobase_cart', JSON.stringify(c));
	}

	useEffect(() => {
		setCart(loadCart());

		(async () => {
			try {
				const response = await api.get<Product[]>('/ext/products/products');
				setProducts(Array.isArray(response) ? response.filter(p => p.status === 'active') : []);
			} catch (err: any) {
				if (err.status !== 404) setError(err.message || 'Failed to load products');
			}
			setLoading(false);
		})();
	}, []);

	function addToCart(product: Product) {
		setCart(prev => {
			const existing = prev.find(item => item.product.id === product.id);
			const next = existing
				? prev.map(item => item.product.id === product.id ? { ...item, quantity: item.quantity + 1 } : item)
				: [...prev, { product, quantity: 1 }];
			saveCart(next);
			return next;
		});
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

	function getCartQuantity(productId: string): number {
		return cart.find(i => i.product.id === productId)?.quantity || 0;
	}

	function isInCart(productId: string): boolean {
		return cart.some(item => item.product.id === productId);
	}

	function getCartTotal(): number {
		return cart.reduce((sum, item) => sum + (item.product.price || 0) * item.quantity, 0);
	}

	function getCartItemCount(): number {
		return cart.reduce((sum, item) => sum + item.quantity, 0);
	}

	return html`
		<div class="page-container">
			<div class="content-wrapper">
				<div class="products-card">
					<div class="header">
						<button class="back-button" onClick=${() => { window.location.href = '/profile'; }}>
							<${ArrowLeft} size=${20} />
							<span>Back to Profile</span>
						</button>
						<div class="title-row">
							<h1>Products</h1>
							${cart.length > 0 ? html`
								<button class="cart-button" onClick=${() => { window.location.href = '/products/checkout'; }}>
									<${ShoppingCart} size=${20} />
									<span class="cart-count">${getCartItemCount()}</span>
									<span class="cart-total">${formatPrice(getCartTotal())}</span>
								</button>
							` : null}
						</div>
					</div>

					${loading ? html`
						<div class="loading">
							<${LoadingSpinner} size="lg" />
							<p>Loading products...</p>
						</div>
					` : error ? html`
						<div class="alert alert-error">${error}</div>
					` : products.length === 0 ? html`
						<${EmptyState}
							icon=${Package}
							title="No products available"
							description="There are no products available at the moment. Check back later!"
						/>
					` : html`
						<div class="products-grid">
							${products.map(product => html`
								<div class="product-card" key=${product.id}>
									<div class="product-image">
										${product.imageUrl ? html`
											<img src=${product.imageUrl} alt=${product.name} />
										` : html`
											<div class="placeholder-image">
												<${Package} size=${32} />
											</div>
										`}
									</div>
									<div class="product-info">
										<h3>${product.name}</h3>
										${product.description ? html`<p class="description">${product.description}</p>` : null}
										<div class="price">${formatPrice(product.price, product.currency)}</div>
									</div>
									<div class="product-actions">
										${isInCart(product.id) ? html`
											<div class="quantity-controls">
												<button class="qty-btn" onClick=${() => updateQuantity(product.id, -1)}>
													<${Minus} size=${16} />
												</button>
												<span class="qty-value">${getCartQuantity(product.id)}</span>
												<button class="qty-btn" onClick=${() => updateQuantity(product.id, 1)}>
													<${Plus} size=${16} />
												</button>
												<button class="remove-btn" onClick=${() => removeFromCart(product.id)}>
													<${Trash2} size=${16} />
												</button>
											</div>
										` : html`
											<button class="btn btn-primary" onClick=${() => addToCart(product)}>
												<${ShoppingCart} size=${16} />
												Add to Cart
											</button>
										`}
									</div>
								</div>
							`)}
						</div>

						${cart.length > 0 ? html`
							<div class="checkout-bar">
								<div class="checkout-summary">
									<span>${getCartItemCount()} item${getCartItemCount() !== 1 ? 's' : ''}</span>
									<span class="checkout-total">${formatPrice(getCartTotal())}</span>
								</div>
								<button class="btn btn-checkout" onClick=${() => { window.location.href = '/products/checkout'; }}>
									Proceed to Checkout
								</button>
							</div>
						` : null}
					`}
				</div>
			</div>
		</div>
		<style>
			.page-container { min-height: 100vh; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); display: flex; align-items: flex-start; justify-content: center; padding: 2rem 1rem; }
			.content-wrapper { width: 100%; max-width: 900px; }
			.products-card { background: white; border-radius: 16px; box-shadow: 0 10px 40px rgba(0, 0, 0, 0.15); padding: 2rem; }
			.header { margin-bottom: 2rem; }
			.back-button { display: inline-flex; align-items: center; gap: 0.5rem; color: #667eea; background: none; border: none; cursor: pointer; font-size: 0.875rem; padding: 0; margin-bottom: 1rem; }
			.back-button:hover { text-decoration: underline; }
			.title-row { display: flex; align-items: center; justify-content: space-between; gap: 1rem; }
			h1 { font-size: 1.5rem; font-weight: 600; color: #1f2937; margin: 0; }
			.cart-button { display: flex; align-items: center; gap: 0.5rem; padding: 0.5rem 1rem; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; border: none; border-radius: 8px; cursor: pointer; font-weight: 500; transition: opacity 0.2s; }
			.cart-button:hover { opacity: 0.9; }
			.cart-count { background: white; color: #667eea; width: 20px; height: 20px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 0.75rem; font-weight: 600; }
			.cart-total { font-size: 0.875rem; }
			.loading { display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 3rem; color: #6b7280; }
			.loading p { margin-top: 1rem; }
			.alert { padding: 1rem; border-radius: 8px; margin-bottom: 1rem; }
			.alert-error { background: #fef2f2; color: #991b1b; border: 1px solid #fecaca; }
			.products-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(250px, 1fr)); gap: 1.5rem; }
			.product-card { display: flex; flex-direction: column; background: #f9fafb; border-radius: 12px; border: 1px solid #e5e7eb; overflow: hidden; transition: border-color 0.2s, box-shadow 0.2s; }
			.product-card:hover { border-color: #667eea; box-shadow: 0 4px 12px rgba(102, 126, 234, 0.15); }
			.product-image { width: 100%; height: 160px; overflow: hidden; background: #e5e7eb; }
			.product-image img { width: 100%; height: 100%; object-fit: cover; }
			.placeholder-image { width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; }
			.product-info { flex: 1; padding: 1rem; }
			.product-info h3 { font-size: 1rem; font-weight: 600; color: #1f2937; margin: 0 0 0.5rem 0; }
			.product-info .description { font-size: 0.875rem; color: #6b7280; margin: 0 0 0.75rem 0; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
			.price { font-size: 1.25rem; font-weight: 700; color: #667eea; }
			.product-actions { padding: 0 1rem 1rem; }
			.quantity-controls { display: flex; align-items: center; gap: 0.5rem; }
			.qty-btn { width: 32px; height: 32px; display: flex; align-items: center; justify-content: center; background: #e5e7eb; border: none; border-radius: 6px; cursor: pointer; transition: background 0.2s; }
			.qty-btn:hover { background: #d1d5db; }
			.qty-value { min-width: 32px; text-align: center; font-weight: 600; }
			.remove-btn { margin-left: auto; width: 32px; height: 32px; display: flex; align-items: center; justify-content: center; background: none; border: none; color: #9ca3af; cursor: pointer; border-radius: 6px; transition: all 0.2s; }
			.remove-btn:hover { background: #fee2e2; color: #dc2626; }
			.btn { display: inline-flex; align-items: center; gap: 0.5rem; padding: 0.75rem 1.5rem; border-radius: 8px; font-weight: 500; cursor: pointer; transition: all 0.2s; border: none; width: 100%; justify-content: center; }
			.btn-primary { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; }
			.btn-primary:hover { opacity: 0.9; transform: translateY(-1px); }
			.checkout-bar { display: flex; align-items: center; justify-content: space-between; margin-top: 2rem; padding: 1.5rem; background: #f9fafb; border-radius: 12px; border: 1px solid #e5e7eb; }
			.checkout-summary { display: flex; flex-direction: column; gap: 0.25rem; }
			.checkout-summary span:first-child { font-size: 0.875rem; color: #6b7280; }
			.checkout-total { font-size: 1.5rem; font-weight: 700; color: #1f2937; }
			.btn-checkout { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 1rem 2rem; font-size: 1rem; width: auto; }
			.btn-checkout:hover { opacity: 0.9; transform: translateY(-1px); }
			@media (max-width: 640px) {
				.page-container { padding: 1rem; }
				.products-card { padding: 1.5rem; }
				.products-grid { grid-template-columns: 1fr; }
				.checkout-bar { flex-direction: column; gap: 1rem; }
				.btn-checkout { width: 100%; }
			}
		</style>
	`;
}
