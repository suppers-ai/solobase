import { html, api, LoadingSpinner, EmptyState, checkAuth, formatPrice } from '@solobase/ui';
import { useState, useEffect } from 'preact/hooks';
import { ArrowLeft, Package, Calendar, DollarSign, CheckCircle, Clock, XCircle } from 'lucide-preact';

interface Product {
	id: string;
	name: string;
	description?: string;
	price?: number;
	currency?: string;
	status: string;
	purchasedAt?: string;
	expiresAt?: string;
}

function formatDate(dateString?: string): string {
	if (!dateString) return 'N/A';
	return new Date(dateString).toLocaleDateString('en-US', {
		year: 'numeric',
		month: 'short',
		day: 'numeric'
	});
}

function getStatusIcon(status: string) {
	switch (status.toLowerCase()) {
		case 'active': return CheckCircle;
		case 'pending': return Clock;
		case 'expired':
		case 'cancelled': return XCircle;
		default: return Package;
	}
}

function getStatusColor(status: string): string {
	switch (status.toLowerCase()) {
		case 'active': return 'color: #16a34a';
		case 'pending': return 'color: #ca8a04';
		case 'expired':
		case 'cancelled': return 'color: #dc2626';
		default: return 'color: #6b7280';
	}
}

export function ProfileProductsPage() {
	const [products, setProducts] = useState<Product[]>([]);
	const [loading, setLoading] = useState(true);
	const [error, setError] = useState('');

	useEffect(() => {
		(async () => {
			const authed = await checkAuth();
			if (!authed) {
				window.location.href = '/login';
				return;
			}

			try {
				const response = await api.get<Product[]>('/ext/products/list');
				setProducts(Array.isArray(response) ? response : []);
			} catch (err: any) {
				if (err.status !== 404) setError(err.message || 'Failed to load products');
			}
			setLoading(false);
		})();
	}, []);

	return html`
		<div class="page-container">
			<div class="content-wrapper">
				<div class="products-card">
					<div class="header">
						<button class="back-button" onClick=${() => { window.location.href = '/profile'; }}>
							<${ArrowLeft} size=${20} />
							<span>Back to Profile</span>
						</button>
						<h1>My Products</h1>
					</div>

					${loading ? html`
						<div class="loading">
							<${LoadingSpinner} size="lg" />
							<p>Loading your products...</p>
						</div>
					` : error ? html`
						<div class="alert alert-error">${error}</div>
					` : products.length === 0 ? html`
						<${EmptyState}
							icon=${Package}
							title="No products yet"
							description="You haven't purchased any products yet. Browse our catalog to find something you'll love."
						>
							<button class="btn btn-primary" onClick=${() => { window.location.href = '/products'; }}>
								Browse Products
							</button>
						<//>
					` : html`
						<div class="products-list">
							${products.map(product => html`
								<div class="product-card" key=${product.id}>
									<div class="product-icon">
										<${Package} size=${24} />
									</div>
									<div class="product-info">
										<h3>${product.name}</h3>
										${product.description ? html`<p class="description">${product.description}</p>` : null}
										<div class="meta">
											${product.purchasedAt ? html`
												<span class="meta-item">
													<${Calendar} size=${14} />
													Purchased ${formatDate(product.purchasedAt)}
												</span>
											` : null}
											${product.price !== undefined ? html`
												<span class="meta-item">
													<${DollarSign} size=${14} />
													${formatPrice(product.price, product.currency)}
												</span>
											` : null}
										</div>
									</div>
									<div class="product-status" style=${getStatusColor(product.status)}>
										<${getStatusIcon(product.status)} size=${16} />
										<span>${product.status}</span>
									</div>
								</div>
							`)}
						</div>
					`}
				</div>
			</div>
		</div>
		<style>
			.page-container { min-height: 100vh; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); display: flex; align-items: flex-start; justify-content: center; padding: 2rem 1rem; }
			.content-wrapper { width: 100%; max-width: 800px; }
			.products-card { background: white; border-radius: 16px; box-shadow: 0 10px 40px rgba(0, 0, 0, 0.15); padding: 2rem; }
			.header { margin-bottom: 2rem; }
			.back-button { display: inline-flex; align-items: center; gap: 0.5rem; color: #667eea; background: none; border: none; cursor: pointer; font-size: 0.875rem; padding: 0; margin-bottom: 1rem; }
			.back-button:hover { text-decoration: underline; }
			h1 { font-size: 1.5rem; font-weight: 600; color: #1f2937; margin: 0; }
			.loading { display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 3rem; color: #6b7280; }
			.loading p { margin-top: 1rem; }
			.alert { padding: 1rem; border-radius: 8px; margin-bottom: 1rem; }
			.alert-error { background: #fef2f2; color: #991b1b; border: 1px solid #fecaca; }
			.products-list { display: flex; flex-direction: column; gap: 1rem; }
			.product-card { display: flex; align-items: flex-start; gap: 1rem; padding: 1.25rem; background: #f9fafb; border-radius: 12px; border: 1px solid #e5e7eb; transition: border-color 0.2s, box-shadow 0.2s; }
			.product-card:hover { border-color: #667eea; box-shadow: 0 4px 12px rgba(102, 126, 234, 0.1); }
			.product-icon { flex-shrink: 0; width: 48px; height: 48px; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); border-radius: 12px; display: flex; align-items: center; justify-content: center; color: white; }
			.product-info { flex: 1; min-width: 0; }
			.product-info h3 { font-size: 1rem; font-weight: 600; color: #1f2937; margin: 0 0 0.25rem 0; }
			.product-info .description { font-size: 0.875rem; color: #6b7280; margin: 0 0 0.5rem 0; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
			.meta { display: flex; flex-wrap: wrap; gap: 1rem; }
			.meta-item { display: flex; align-items: center; gap: 0.25rem; font-size: 0.75rem; color: #6b7280; }
			.product-status { display: flex; align-items: center; gap: 0.25rem; font-size: 0.75rem; font-weight: 500; text-transform: capitalize; white-space: nowrap; }
			.btn { display: inline-flex; align-items: center; gap: 0.5rem; padding: 0.75rem 1.5rem; border-radius: 8px; font-weight: 500; cursor: pointer; transition: all 0.2s; border: none; }
			.btn-primary { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; }
			.btn-primary:hover { opacity: 0.9; transform: translateY(-1px); }
			@media (max-width: 640px) {
				.page-container { padding: 1rem; }
				.products-card { padding: 1.5rem; }
				.product-card { flex-direction: column; }
				.product-status { align-self: flex-start; }
			}
		</style>
	`;
}
