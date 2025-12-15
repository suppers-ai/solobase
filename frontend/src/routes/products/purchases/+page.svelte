<script lang="ts">
	import { onMount } from 'svelte';
	import { api, ErrorHandler } from '$lib/api';
	import { formatDateLong } from '$lib/utils/formatters';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import { ShoppingBag } from 'lucide-svelte';

	interface Purchase {
		id: number;
		status: string;
		totalCents: number;
		currency: string;
		createdAt: string;
		lineItems: Array<{
			productName: string;
			quantity: number;
			unitPrice: number;
			totalPrice: number;
		}>;
	}

	let purchases: Purchase[] = [];
	let loading = true;
	let error: string | null = null;
	let currentPage = 0;
	const limit = 10;
	let total = 0;
	let showCancelConfirm = false;
	let purchaseToCancel: number | null = null;

	interface PurchasesResponse {
		purchases?: Purchase[];
		total?: number;
	}

	async function loadPurchases() {
		loading = true;
		error = null;
		try {
			const data = await api.get<PurchasesResponse>(
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

	function cancelPurchase(purchaseId: number) {
		purchaseToCancel = purchaseId;
		showCancelConfirm = true;
	}

	async function confirmCancelPurchase() {
		if (!purchaseToCancel) return;
		showCancelConfirm = false;

		try {
			await api.post(`/ext/products/purchases/${purchaseToCancel}/cancel`, {
				reason: 'Customer requested cancellation'
			});
			await loadPurchases();
		} catch (err) {
			ErrorHandler.handle(err);
		}
		purchaseToCancel = null;
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
		<div class="bg-white rounded-lg shadow p-12">
			<EmptyState
				icon={ShoppingBag}
				title="No purchases yet"
				message="When you make a purchase, it will appear here."
			>
				<a href="/products/checkout" class="btn btn-primary">
					Start Shopping
				</a>
			</EmptyState>
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
								<p class="text-sm text-gray-600">{formatDateLong(purchase.createdAt)}</p>
							</div>
							<div class="text-right">
								<p class="text-xl font-bold">
									{formatCurrency(purchase.totalCents, purchase.currency)}
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

						{#if purchase.lineItems && purchase.lineItems.length > 0}
							<div class="border-t pt-4">
								<h4 class="text-sm font-semibold text-gray-700 mb-2">Items</h4>
								<div class="space-y-2">
									{#each purchase.lineItems as item}
										<div class="flex justify-between text-sm">
											<div>
												<span class="font-medium">{item.productName}</span>
												<span class="text-gray-500 ml-2">×{item.quantity}</span>
											</div>
											<span class="text-gray-900">
												{formatCurrency(item.totalPrice, purchase.currency)}
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

<ConfirmDialog
	bind:show={showCancelConfirm}
	title="Cancel Purchase"
	message="Are you sure you want to cancel this purchase?"
	confirmText="Cancel"
	variant="danger"
	on:confirm={confirmCancelPurchase}
/>