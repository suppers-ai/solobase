<script lang="ts">
	import { onMount } from 'svelte';
	import { api, ErrorHandler } from '$lib/api';
	import { formatDateLong } from '$lib/utils/formatters';

	interface Purchase {
		id: number;
		status: string;
		total_cents: number;
		currency: string;
		created_at: string;
		line_items: Array<{
			product_name: string;
			quantity: number;
			unit_price: number;
			total_price: number;
		}>;
	}

	let purchases: Purchase[] = [];
	let loading = true;
	let error: string | null = null;
	let currentPage = 0;
	const limit = 10;
	let total = 0;

	async function loadPurchases() {
		loading = true;
		error = null;
		try {
			const data = await api.get(
				`/ext/products/purchases?limit=${limit}&offset=${currentPage * limit}`
			);
			purchases = data.purchases || [];
			total = data.total || 0;
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load purchases';
			console.error('Error loading purchases:', err);
		} finally {
			loading = false;
		}
	}

	async function cancelPurchase(purchaseId: number) {
		if (!confirm('Are you sure you want to cancel this purchase?')) return;

		try {
			await api.post(`/ext/products/purchases/${purchaseId}/cancel`, {
				reason: 'Customer requested cancellation'
			});
			await loadPurchases();
		} catch (err) {
			ErrorHandler.handle(err);
		}
	}

	function formatCurrency(cents: number, currency: string): string {
		return new Intl.NumberFormat('en-US', {
			style: 'currency',
			currency: currency || 'USD'
		}).format(cents / 100);
	}

	function getStatusBadge(status: string): string {
		switch (status) {
			case 'paid': return 'bg-green-100 text-green-800';
			case 'pending': return 'bg-yellow-100 text-yellow-800';
			case 'refunded': return 'bg-red-100 text-red-800';
			case 'cancelled': return 'bg-gray-100 text-gray-800';
			default: return 'bg-gray-100 text-gray-800';
		}
	}

	function getStatusLabel(status: string): string {
		switch (status) {
			case 'paid': return 'Paid';
			case 'pending': return 'Pending';
			case 'refunded': return 'Refunded';
			case 'cancelled': return 'Cancelled';
			case 'requires_approval': return 'Awaiting Approval';
			default: return status;
		}
	}

	onMount(() => {
		loadPurchases();
	});
</script>

<div class="max-w-4xl mx-auto p-6">
	<div class="mb-8">
		<h1 class="text-3xl font-bold text-gray-900 mb-2">Your Purchases</h1>
		<p class="text-gray-600">View and manage your purchase history</p>
	</div>

	{#if loading}
		<div class="flex justify-center items-center h-64">
			<div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500"></div>
		</div>
	{:else if error}
		<div class="bg-red-50 border border-red-200 rounded-md p-4 text-red-700">
			{error}
		</div>
	{:else if purchases.length === 0}
		<div class="bg-white rounded-lg shadow p-12 text-center">
			<div class="mx-auto w-16 h-16 bg-gray-100 rounded-full flex items-center justify-center mb-4">
				<svg class="w-8 h-8 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 11V7a4 4 0 00-8 0v4M5 9h14l1 12H4L5 9z"></path>
				</svg>
			</div>
			<h2 class="text-xl font-semibold text-gray-900 mb-2">No purchases yet</h2>
			<p class="text-gray-600 mb-6">When you make a purchase, it will appear here.</p>
			<a
				href="/products/checkout"
				class="inline-block px-6 py-3 bg-blue-600 text-white font-semibold rounded-md hover:bg-blue-700 transition-colors"
			>
				Start Shopping
			</a>
		</div>
	{:else}
		<div class="space-y-4">
			{#each purchases as purchase}
				<div class="bg-white rounded-lg shadow hover:shadow-md transition-shadow">
					<div class="p-6">
						<div class="flex justify-between items-start mb-4">
							<div>
								<div class="flex items-center gap-3 mb-1">
									<h3 class="text-lg font-semibold">Order #{purchase.id}</h3>
									<span class="px-2 py-1 text-xs font-semibold rounded-full {getStatusBadge(purchase.status)}">
										{getStatusLabel(purchase.status)}
									</span>
								</div>
								<p class="text-sm text-gray-600">{formatDateLong(purchase.created_at)}</p>
							</div>
							<div class="text-right">
								<p class="text-xl font-bold">
									{formatCurrency(purchase.total_cents, purchase.currency)}
								</p>
								{#if purchase.status === 'pending'}
									<button
										on:click={() => cancelPurchase(purchase.id)}
										class="mt-2 text-sm text-red-600 hover:text-red-800"
									>
										Cancel Order
									</button>
								{/if}
							</div>
						</div>

						{#if purchase.line_items && purchase.line_items.length > 0}
							<div class="border-t pt-4">
								<h4 class="text-sm font-semibold text-gray-700 mb-2">Items</h4>
								<div class="space-y-2">
									{#each purchase.line_items as item}
										<div class="flex justify-between text-sm">
											<div>
												<span class="font-medium">{item.product_name}</span>
												<span class="text-gray-500 ml-2">×{item.quantity}</span>
											</div>
											<span class="text-gray-900">
												{formatCurrency(item.total_price, purchase.currency)}
											</span>
										</div>
									{/each}
								</div>
							</div>
						{/if}
					</div>
				</div>
			{/each}
		</div>

		{#if total > limit}
			<div class="mt-6 flex justify-center gap-2">
				<button
					on:click={() => { currentPage--; loadPurchases(); }}
					disabled={currentPage === 0}
					class="px-4 py-2 border rounded-md text-sm font-medium {currentPage === 0 ? 'bg-gray-100 text-gray-400 cursor-not-allowed' : 'bg-white hover:bg-gray-50 text-gray-700'}"
				>
					Previous
				</button>
				<span class="px-4 py-2 text-sm text-gray-700">
					Page {currentPage + 1} of {Math.ceil(total / limit)}
				</span>
				<button
					on:click={() => { currentPage++; loadPurchases(); }}
					disabled={(currentPage + 1) * limit >= total}
					class="px-4 py-2 border rounded-md text-sm font-medium {(currentPage + 1) * limit >= total ? 'bg-gray-100 text-gray-400 cursor-not-allowed' : 'bg-white hover:bg-gray-50 text-gray-700'}"
				>
					Next
				</button>
			</div>
		{/if}
	{/if}
</div>