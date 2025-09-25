<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { api } from '$lib/api';

	let loading = true;
	let purchase: any = null;
	let error: string | null = null;

	async function loadPurchaseDetails() {
		// Get session_id from URL params (Stripe adds this)
		const sessionId = $page.url.searchParams.get('session_id');

		if (!sessionId) {
			// If no session ID, just show success message
			loading = false;
			return;
		}

		try {
			// You could fetch purchase details by session ID if needed
			// For now, just show success
			loading = false;
		} catch (err) {
			console.error('Error loading purchase details:', err);
			error = 'Failed to load purchase details';
			loading = false;
		}
	}

	onMount(() => {
		// Clear the cart from localStorage
		localStorage.removeItem('product_cart');
		loadPurchaseDetails();
	});
</script>

<div class="min-h-screen flex items-center justify-center bg-gray-50">
	<div class="max-w-md w-full">
		<div class="bg-white rounded-lg shadow-lg p-8 text-center">
			<!-- Success Icon -->
			<div class="mx-auto w-16 h-16 bg-green-100 rounded-full flex items-center justify-center mb-4">
				<svg class="w-8 h-8 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
				</svg>
			</div>

			<h1 class="text-2xl font-bold text-gray-900 mb-2">Payment Successful!</h1>

			<p class="text-gray-600 mb-6">
				Thank you for your purchase. You will receive an email confirmation shortly.
			</p>

			{#if $page.url.searchParams.get('session_id')}
				<div class="bg-gray-50 rounded-md p-4 mb-6">
					<p class="text-sm text-gray-600">
						Session ID:
					</p>
					<p class="text-xs font-mono text-gray-500 break-all">
						{$page.url.searchParams.get('session_id')}
					</p>
				</div>
			{/if}

			<div class="space-y-3">
				<a
					href="/products/checkout"
					class="block w-full py-2 px-4 bg-blue-600 text-white font-semibold rounded-md hover:bg-blue-700 transition-colors"
				>
					Continue Shopping
				</a>

				<a
					href="/ext/products/purchases"
					class="block w-full py-2 px-4 bg-gray-200 text-gray-800 font-semibold rounded-md hover:bg-gray-300 transition-colors"
				>
					View Your Purchases
				</a>
			</div>
		</div>

		<div class="mt-6 text-center">
			<p class="text-sm text-gray-500">
				Need help? <a href="/support" class="text-blue-600 hover:underline">Contact Support</a>
			</p>
		</div>
	</div>
</div>