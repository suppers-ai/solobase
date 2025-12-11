<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { goto } from '$app/navigation';

	interface Product {
		id: number;
		name: string;
		description: string;
		basePrice: number;
		currency: string;
	}

	interface CartItem {
		product: Product;
		quantity: number;
		variables: Record<string, any>;
	}

	let cart: CartItem[] = [];
	let loading = false;
	let error: string | null = null;
	let products: Product[] = [];
	let selectedProduct: Product | null = null;
	let quantity = 1;
	let email = '';

	async function loadProducts() {
		try {
			const data = await api.get<Product[]>('/ext/products/products');
			products = data || [];
		} catch (err) {
			console.error('Error loading products:', err);
		}
	}

	function addToCart(product: Product) {
		const existing = cart.find(item => item.product.id === product.id);
		if (existing) {
			existing.quantity += quantity;
		} else {
			cart.push({
				product,
				quantity,
				variables: {}
			});
		}
		cart = cart;
		quantity = 1;
		selectedProduct = null;
	}

	function removeFromCart(index: number) {
		cart.splice(index, 1);
		cart = cart;
	}

	function updateQuantity(index: number, newQuantity: number) {
		if (newQuantity > 0) {
			cart[index].quantity = newQuantity;
			cart = cart;
		}
	}

	function calculateTotal(): number {
		return cart.reduce((total, item) => {
			return total + (item.product.basePrice * item.quantity);
		}, 0);
	}

	async function proceedToCheckout() {
		if (cart.length === 0) {
			error = 'Your cart is empty';
			return;
		}

		loading = true;
		error = null;

		try {
			// Prepare purchase request
			const purchaseRequest = {
				items: cart.map(item => ({
					productId: item.product.id,
					quantity: item.quantity,
					variables: item.variables
				})),
				successUrl: `${window.location.origin}/products/success`,
				cancelUrl: `${window.location.origin}/products/checkout`,
				customerEmail: email || undefined,
				paymentMethodTypes: ['card'],
				metadata: {
					source: 'web_checkout'
				}
			};

			interface CheckoutResponse {
				checkoutUrl?: string;
				purchase?: {
					providerSessionId?: string;
				};
			}

			// Create checkout session
			const data = await api.post<CheckoutResponse>('/ext/products/purchase', purchaseRequest);

			// Redirect to Stripe Checkout
			if (data.checkoutUrl) {
				// For Stripe's hosted checkout, we can redirect directly
				window.location.href = data.checkoutUrl;
			} else if (data.purchase?.providerSessionId) {
				// Construct Stripe Checkout URL if not provided
				const checkoutUrl = `https://checkout.stripe.com/c/pay/${data.purchase.providerSessionId}#`;
				window.location.href = checkoutUrl;
			} else {
				throw new Error('No checkout URL received');
			}
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to create checkout session';
			console.error('Checkout error:', err);
			loading = false;
		}
	}

	function formatPrice(price: number, currency: string): string {
		return new Intl.NumberFormat('en-US', {
			style: 'currency',
			currency: currency || 'USD'
		}).format(price);
	}

	onMount(() => {
		loadProducts();
		// Load cart from localStorage if exists
		const savedCart = localStorage.getItem('product_cart');
		if (savedCart) {
			try {
				cart = JSON.parse(savedCart);
			} catch (e) {
				console.error('Failed to load cart:', e);
			}
		}
	});

	// Save cart to localStorage whenever it changes
	$: if (cart) {
		localStorage.setItem('product_cart', JSON.stringify(cart));
	}
</script>

<div class="max-w-6xl mx-auto p-6">
	<h1 class="text-3xl font-bold mb-8">Checkout</h1>

	<div class="grid grid-cols-1 lg:grid-cols-3 gap-8">
		<!-- Product Selection -->
		<div class="lg:col-span-2">
			<div class="bg-white rounded-lg shadow p-6 mb-6">
				<h2 class="text-xl font-semibold mb-4">Available Products</h2>

				{#if products.length === 0}
					<p class="text-gray-500">No products available</p>
				{:else}
					<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
						{#each products as product}
							<div class="border rounded-lg p-4 hover:shadow-md transition-shadow">
								<h3 class="font-semibold">{product.name}</h3>
								<p class="text-sm text-gray-600 mb-2">{product.description}</p>
								<div class="flex items-center justify-between">
									<span class="text-lg font-bold">
										{formatPrice(product.basePrice, product.currency)}
									</span>
									<button
										on:click={() => addToCart(product)}
										class="px-3 py-1 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors text-sm"
									>
										Add to Cart
									</button>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>

			<!-- Shopping Cart -->
			<div class="bg-white rounded-lg shadow p-6">
				<h2 class="text-xl font-semibold mb-4">Shopping Cart</h2>

				{#if cart.length === 0}
					<p class="text-gray-500">Your cart is empty</p>
				{:else}
					<div class="space-y-4">
						{#each cart as item, index}
							<div class="flex items-center justify-between border-b pb-4">
								<div class="flex-1">
									<h3 class="font-semibold">{item.product.name}</h3>
									<p class="text-sm text-gray-600">
										{formatPrice(item.product.basePrice, item.product.currency)} each
									</p>
								</div>
								<div class="flex items-center gap-2">
									<input
										type="number"
										min="1"
										value={item.quantity}
										on:change={(e) => updateQuantity(index, parseInt(e.currentTarget.value))}
										class="w-16 px-2 py-1 border rounded"
									/>
									<button
										on:click={() => removeFromCart(index)}
										class="text-red-500 hover:text-red-700"
									>
										Remove
									</button>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		</div>

		<!-- Checkout Summary -->
		<div class="lg:col-span-1">
			<div class="bg-white rounded-lg shadow p-6 sticky top-6">
				<h2 class="text-xl font-semibold mb-4">Order Summary</h2>

				<div class="space-y-2 mb-4">
					{#each cart as item}
						<div class="flex justify-between text-sm">
							<span>{item.product.name} x{item.quantity}</span>
							<span>{formatPrice(item.product.basePrice * item.quantity, item.product.currency)}</span>
						</div>
					{/each}
				</div>

				<div class="border-t pt-4 mb-4">
					<div class="flex justify-between font-semibold text-lg">
						<span>Total</span>
						<span>{formatPrice(calculateTotal(), 'USD')}</span>
					</div>
				</div>

				<div class="mb-4">
					<label for="email" class="block text-sm font-medium text-gray-700 mb-1">
						Email (for receipt)
					</label>
					<input
						id="email"
						type="email"
						bind:value={email}
						placeholder="your@email.com"
						class="w-full px-3 py-2 border rounded-md"
					/>
				</div>

				{#if error}
					<div class="mb-4 p-3 bg-red-50 text-red-700 rounded-md text-sm">
						{error}
					</div>
				{/if}

				<button
					on:click={proceedToCheckout}
					disabled={loading || cart.length === 0}
					class="w-full py-3 px-4 bg-blue-600 text-white font-semibold rounded-md hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors"
				>
					{#if loading}
						Processing...
					{:else}
						Proceed to Stripe Checkout
					{/if}
				</button>

				<p class="mt-4 text-xs text-gray-500 text-center">
					You will be redirected to Stripe's secure checkout page
				</p>
			</div>
		</div>
	</div>
</div>

<style>
	input[type="number"]::-webkit-inner-spin-button,
	input[type="number"]::-webkit-outer-spin-button {
		-webkit-appearance: none;
		margin: 0;
	}
</style>