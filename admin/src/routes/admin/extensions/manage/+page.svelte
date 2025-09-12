<script lang="ts">
	import { onMount } from "svelte";
	import { api } from "$lib/api";
	import { requireAdmin } from "$lib/utils/auth";

	let extensions: any[] = [];
	let loading = true;
	let error: string | null = null;
	let selectedExtension: any = null;
	let showDetails = false;

	// Extension icons mapping
	const extensionIcons: Record<string, string> = {
		"Products & Pricing": "📦",
		hugo: "🌐",
		analytics: "📊",
		cloudstorage: "☁️",
		webhooks: "🔗",
	};

	// Extension configuration details
	const extensionDetails: Record<string, any> = {
		"Products & Pricing": {
			features: [
				"Dynamic product catalog management",
				"Flexible pricing rules and tiers",
				"Variable-based pricing calculations",
				"Sales tracking and analytics",
				"Multi-currency support",
				"Product templates and categories",
			],
			configuration: {
				"Default Currency": "USD",
				"Max Product Images": "10",
				"Enable User Sales": "Yes",
				"Template Sets": "ecommerce, subscription, service",
			},
			usage: `
				1. Navigate to Products section in the main menu
				2. Create product types and categories
				3. Add products with pricing rules
				4. Configure variables for dynamic pricing
				5. Monitor sales through the dashboard
			`,
			apiEndpoints: [
				"GET /api/products - List all products",
				"POST /api/products - Create new product",
				"GET /api/pricing/{id} - Get pricing rules",
				"POST /api/sales - Record a sale",
			],
		},
		hugo: {
			features: [
				"Static site generation with Hugo",
				"Multiple site management",
				"Theme customization",
				"Automated builds and deployments",
				"Version control integration",
				"SEO optimization tools",
			],
			configuration: {
				"Hugo Binary Path": "/usr/local/bin/hugo",
				"Max Sites Per User": "10",
				"Build Timeout": "10 minutes",
				"Default Theme": "default",
				"Storage Bucket": "hugo-sites",
			},
			usage: `
				1. Create a new Hugo site from the dashboard
				2. Choose a theme or upload custom theme
				3. Add content through the editor
				4. Build and preview your site
				5. Deploy to production with one click
			`,
			apiEndpoints: [
				"GET /api/hugo/sites - List all sites",
				"POST /api/hugo/sites - Create new site",
				"POST /api/hugo/build/{id} - Build site",
				"GET /api/hugo/preview/{id} - Preview site",
			],
		},
		analytics: {
			features: [
				"Real-time visitor tracking",
				"Page view analytics",
				"User behavior insights",
				"Custom event tracking",
				"Conversion funnels",
				"Detailed reports and exports",
			],
			configuration: {
				"Tracking Enabled": "Yes",
				"Data Retention": "90 days",
				"Sample Rate": "100%",
				"Track Authenticated Users": "Yes",
				"Cookie Duration": "30 days",
			},
			usage: `
				1. Include tracking script in your pages
				2. View real-time analytics dashboard
				3. Set up custom events for tracking
				4. Create conversion goals
				5. Export reports for analysis
			`,
			apiEndpoints: [
				"POST /api/analytics/track - Track event",
				"GET /api/analytics/pageviews - Get pageviews",
				"GET /api/analytics/stats - Get statistics",
				"GET /api/analytics/reports - Generate reports",
			],
		},
		cloudstorage: {
			features: [
				"Advanced file sharing with public links and user-specific access",
				"Granular permission control (view, edit, admin)",
				"Storage quota management per user/organization",
				"Bandwidth monitoring and usage limits",
				"Comprehensive access logging and audit trails",
				"Real-time analytics on file access patterns",
				"Automatic expiration for shared links",
				"Inheritance of permissions to child objects",
			],
			configuration: {
				"Default Storage Limit": "5 GB per user",
				"Default Bandwidth Limit": "10 GB per month",
				"Enable Sharing": "Yes",
				"Enable Access Logs": "Yes",
				"Enable Quotas": "Yes",
				"Bandwidth Reset Period": "Monthly",
				"Max Share Expiration": "30 days",
				"Public Sharing Allowed": "Yes",
			},
			usage: `
				1. Navigate to CloudStorage extension page
				2. Create shares with specific permissions (view/edit/admin)
				3. Set expiration dates for temporary access
				4. Monitor storage and bandwidth usage
				5. Track file access through detailed logs
				6. Set per-user or per-organization quotas
				7. View analytics on popular files and access patterns
				8. Generate public links for external sharing
			`,
			apiEndpoints: [
				"POST /ext/cloudstorage/api/shares - Create share link",
				"GET /ext/cloudstorage/api/shares - List active shares",
				"DELETE /ext/cloudstorage/api/shares/{id} - Revoke share",
				"GET /ext/cloudstorage/api/quota - Get quota information",
				"PUT /ext/cloudstorage/api/quota - Update user quotas",
				"GET /ext/cloudstorage/api/access-logs - View access logs",
				"GET /ext/cloudstorage/api/stats - Get usage statistics",
				"GET /share/{token} - Access shared content",
			],
		},
		webhooks: {
			features: [
				"HTTP webhook management",
				"Event-driven triggers",
				"Retry logic with backoff",
				"Request/response logging",
				"Custom headers and auth",
				"Webhook testing tools",
			],
			configuration: {
				"Max Retries": "3",
				Timeout: "30 seconds",
				"Retry Delay": "5 seconds",
				"Log Retention": "7 days",
				"Max Webhooks Per User": "50",
			},
			usage: `
				1. Create webhook endpoints
				2. Configure trigger events
				3. Set authentication if needed
				4. Test webhook with sample data
				5. Monitor webhook activity logs
			`,
			apiEndpoints: [
				"GET /api/webhooks - List webhooks",
				"POST /api/webhooks - Create webhook",
				"POST /api/webhooks/{id}/test - Test webhook",
				"GET /api/webhooks/{id}/logs - View logs",
			],
		},
	};

	onMount(async () => {
		// Check admin access
		if (!requireAdmin()) return;

		await loadExtensions();
	});

	async function loadExtensions() {
		try {
			loading = true;
			error = null;
			const response = await api.getExtensions();
			if (response.error) {
				throw new Error(response.error);
			}
			// The API returns the extensions array directly in response.data
			extensions = Array.isArray(response.data) ? response.data : [];
			console.log("Loaded extensions:", extensions);
		} catch (err) {
			console.error("Failed to load extensions:", err);
			error = err.message || "Failed to load extensions";
			extensions = [];
		} finally {
			loading = false;
		}
	}

	async function toggleExtension(name: string, enable: boolean) {
		try {
			const response = await api.toggleExtension(name, enable);
			if (response.error) {
				throw new Error(response.error);
			}
			await loadExtensions();
		} catch (err: any) {
			alert(`Error: ${err.message}`);
		}
	}

	function openDetails(extension: any) {
		selectedExtension = extension;
		showDetails = true;
	}

	function closeDetails() {
		showDetails = false;
		selectedExtension = null;
	}

	function getIcon(name: string) {
		return extensionIcons[name] || "🧩";
	}

	function getDetails(name: string) {
		return (
			extensionDetails[name] || {
				features: ["No details available"],
				configuration: {},
				usage: "No usage information available",
				apiEndpoints: [],
			}
		);
	}
</script>

<div class="min-h-screen bg-gray-50">
	<div class="container mx-auto p-6 max-w-7xl">
		<!-- Header -->
		<div
			class="bg-white rounded-lg shadow-sm p-6 mb-6 border border-gray-200"
		>
			<div class="flex items-center gap-4">
				<div
					class="w-14 h-14 bg-gradient-to-br from-cyan-500 to-cyan-600 rounded-lg flex items-center justify-center text-white text-2xl shadow-sm"
				>
					🧩
				</div>
				<div>
					<h1 class="text-2xl font-bold text-gray-900">
						Extensions Marketplace
					</h1>
					<p class="text-gray-600 text-sm mt-1">
						Discover and manage powerful extensions to enhance your
						application
					</p>
				</div>
			</div>
		</div>

		{#if loading}
			<div class="flex justify-center items-center h-64">
				<div
					class="loading loading-spinner loading-lg text-cyan-600"
				></div>
			</div>
		{:else if error}
			<div class="alert alert-error shadow-lg">
				<svg
					xmlns="http://www.w3.org/2000/svg"
					class="stroke-current flex-shrink-0 h-6 w-6"
					fill="none"
					viewBox="0 0 24 24"
					><path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z"
					/></svg
				>
				<span>Error loading extensions: {error}</span>
			</div>
		{:else if extensions.length === 0}
			<div
				class="bg-white rounded-lg shadow-sm p-12 text-center border border-gray-200"
			>
				<div class="text-5xl mb-4">📭</div>
				<h2 class="text-xl font-bold text-gray-800 mb-2">
					No Extensions Available
				</h2>
				<p class="text-gray-600 text-sm">
					Extensions will appear here once they are registered in the
					system.
				</p>
			</div>
		{:else}
			<div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
				{#each extensions as extension}
					<div
						class="bg-white rounded-lg shadow-sm hover:shadow-md transition-all duration-200 border border-gray-200 overflow-hidden"
					>
						<!-- Extension Header -->
						<div class="p-5 border-b border-gray-100">
							<div class="flex items-start justify-between">
								<div class="flex items-center gap-3">
									<div class="text-3xl">
										{getIcon(extension.name)}
									</div>
									<div>
										<h3
											class="text-lg font-semibold text-gray-900"
										>
											{extension.name}
										</h3>
										<p class="text-xs text-gray-500">
											v{extension.version} • {extension.author}
										</p>
									</div>
								</div>
								<div class="flex items-center">
									<label
										class="relative inline-flex items-center cursor-pointer"
									>
										<input
											type="checkbox"
											class="sr-only peer"
											checked={extension.enabled}
											on:change={() =>
												toggleExtension(
													extension.name,
													!extension.enabled,
												)}
										/>
										<div
											class="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-cyan-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-cyan-600"
										></div>
									</label>
								</div>
							</div>
						</div>

						<!-- Extension Body -->
						<div class="p-5">
							<p class="text-gray-600 text-sm mb-3 line-clamp-2">
								{extension.description}
							</p>

							{#if extension.tags && extension.tags.length > 0}
								<div class="flex flex-wrap gap-1 mb-3">
									{#each extension.tags.slice(0, 3) as tag}
										<span
											class="px-2 py-0.5 bg-gray-100 text-gray-600 rounded text-xs"
										>
											{tag}
										</span>
									{/each}
								</div>
							{/if}

							<!-- Status Badge -->
							<div class="flex items-center gap-2 mb-4">
								<div class="flex items-center gap-2">
									<div
										class="w-2 h-2 rounded-full {extension.enabled
											? 'bg-green-500'
											: 'bg-gray-400'}"
									></div>
									<span
										class="text-xs font-medium {extension.enabled
											? 'text-green-600'
											: 'text-gray-500'}"
									>
										{extension.enabled
											? "Active"
											: "Inactive"}
									</span>
								</div>
								{#if extension.state === "healthy"}
									<span class="text-xs text-gray-500"
										>• Healthy</span
									>
								{/if}
							</div>

							<!-- Action Buttons -->
							<div class="flex gap-2">
								<button
									on:click={() => openDetails(extension)}
									class="flex-1 px-3 py-1.5 bg-cyan-600 text-white rounded hover:bg-cyan-700 transition-colors text-sm"
								>
									View Details
								</button>
								{#if extension.enabled && extension.dashboardUrl}
									<a
										href={extension.dashboardUrl}
										class="px-3 py-1.5 bg-gray-100 text-gray-700 rounded hover:bg-gray-200 transition-colors text-sm"
									>
										Dashboard →
									</a>
								{/if}
							</div>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>

<!-- Details Modal -->
{#if showDetails && selectedExtension}
	{@const details = getDetails(selectedExtension.name)}
	<div
		class="fixed inset-0 bg-black bg-opacity-50"
		style="z-index: 9999;"
		on:click={closeDetails}
	>
		<div
			class="fixed inset-0 flex items-center justify-center p-4"
			style="z-index: 10000;"
		>
			<div
				class="bg-white rounded-lg shadow-2xl max-w-4xl w-full max-h-[90vh] overflow-hidden border border-gray-200"
				on:click|stopPropagation
			>
				<!-- Modal Header -->
				<div
					class="bg-gradient-to-r from-cyan-500 to-cyan-600 text-white p-6"
				>
					<div class="flex items-center justify-between">
						<div class="flex items-center gap-4">
							<div class="text-4xl">
								{getIcon(selectedExtension.name)}
							</div>
							<div>
								<h2 class="text-2xl font-bold">
									{selectedExtension.name}
								</h2>
								<p class="text-cyan-100">
									Version {selectedExtension.version} • {selectedExtension.author}
								</p>
							</div>
						</div>
						<button
							on:click={closeDetails}
							class="p-2 hover:bg-white/20 rounded-lg transition-colors"
						>
							<svg
								class="w-6 h-6"
								fill="none"
								stroke="currentColor"
								viewBox="0 0 24 24"
							>
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M6 18L18 6M6 6l12 12"
								></path>
							</svg>
						</button>
					</div>
				</div>

				<!-- Modal Body -->
				<div class="p-6 overflow-y-auto max-h-[calc(90vh-120px)]">
					<!-- Description -->
					<div class="mb-6">
						<h3 class="text-lg font-bold text-gray-900 mb-2">
							Description
						</h3>
						<p class="text-gray-600">
							{selectedExtension.description}
						</p>
					</div>

					<!-- Features -->
					<div class="mb-6">
						<h3 class="text-lg font-bold text-gray-900 mb-3">
							Features
						</h3>
						<div class="grid grid-cols-1 md:grid-cols-2 gap-3">
							{#each details.features as feature}
								<div class="flex items-start gap-2">
									<svg
										class="w-5 h-5 text-green-500 mt-0.5 flex-shrink-0"
										fill="none"
										stroke="currentColor"
										viewBox="0 0 24 24"
									>
										<path
											stroke-linecap="round"
											stroke-linejoin="round"
											stroke-width="2"
											d="M5 13l4 4L19 7"
										></path>
									</svg>
									<span class="text-gray-700 text-sm"
										>{feature}</span
									>
								</div>
							{/each}
						</div>
					</div>

					<!-- Configuration -->
					<div class="mb-6">
						<h3 class="text-lg font-bold text-gray-900 mb-3">
							Current Configuration
						</h3>
						<div class="bg-gray-50 rounded-lg p-4">
							<div class="grid grid-cols-1 md:grid-cols-2 gap-3">
								{#each Object.entries(details.configuration) as [key, value]}
									<div class="flex justify-between">
										<span class="text-gray-600 text-sm"
											>{key}:</span
										>
										<span
											class="text-gray-900 font-medium text-sm"
											>{value}</span
										>
									</div>
								{/each}
							</div>
						</div>
					</div>

					<!-- How to Use -->
					<div class="mb-6">
						<h3 class="text-lg font-bold text-gray-900 mb-3">
							How to Use
						</h3>
						<div class="bg-cyan-50 rounded-lg p-4">
							<pre
								class="text-sm text-gray-700 whitespace-pre-wrap font-sans">{details.usage.trim()}</pre>
						</div>
					</div>

					<!-- API Endpoints -->
					{#if details.apiEndpoints && details.apiEndpoints.length > 0}
						<div class="mb-6">
							<h3 class="text-lg font-bold text-gray-900 mb-3">
								API Endpoints
							</h3>
							<div class="bg-gray-900 rounded-lg p-4">
								<div class="space-y-2">
									{#each details.apiEndpoints as endpoint}
										<div
											class="font-mono text-sm text-green-400"
										>
											{endpoint}
										</div>
									{/each}
								</div>
							</div>
						</div>
					{/if}

					<!-- Action Buttons -->
					<div class="flex gap-3 pt-4 border-t border-gray-200">
						<button
							on:click={() =>
								toggleExtension(
									selectedExtension.name,
									!selectedExtension.enabled,
								)}
							class="px-5 py-2 {selectedExtension.enabled
								? 'bg-red-500 hover:bg-red-600'
								: 'bg-green-500 hover:bg-green-600'} text-white rounded transition-colors"
						>
							{selectedExtension.enabled
								? "Disable Extension"
								: "Enable Extension"}
						</button>
						{#if selectedExtension.enabled}
							<button
								class="px-5 py-2 bg-cyan-600 text-white rounded hover:bg-cyan-700 transition-colors"
							>
								Configure
							</button>
						{/if}
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.line-clamp-2 {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
</style>
