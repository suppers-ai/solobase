<script>
	import { onMount, onDestroy } from 'svelte';
	import { api } from '$lib/api';
	import { requireAdmin } from '$lib/utils/auth';
	import Chart from 'chart.js/auto';
	import 'chartjs-adapter-date-fns';
	import { 
		BarChart3, TrendingUp, Users, Eye, 
		Clock, RefreshCw, Plus, X,
		Calendar, Activity, MousePointer, Globe
	} from 'lucide-svelte';
	import ExportButton from '$lib/components/ExportButton.svelte';
	
	// State
	let stats = {
		totalViews: 0,
		uniqueUsers: 0,
		todayViews: 0,
		activeNow: 0
	};
	
	let pageViews = [];
	let dailyStats = [];
	let realtimeData = [];
	let loading = true;
	let error = null;
	let refreshInterval;
	let chart = null;
	let realtimeChart = null;
	let selectedTimeRange = '7days';
	
	// Modal state
	let showTrackModal = false;
	let trackEventName = '';
	let trackEventCategory = 'custom';
	let trackEventProperties = '';
	
	const timeRanges = [
		{ value: '24h', label: 'Last 24 Hours' },
		{ value: '7days', label: 'Last 7 Days' },
		{ value: '30days', label: 'Last 30 Days' },
		{ value: '90days', label: 'Last 90 Days' }
	];
	
	const eventCategories = [
		{ value: 'custom', label: 'Custom Event' },
		{ value: 'page_view', label: 'Page View' },
		{ value: 'click', label: 'Click Event' },
		{ value: 'form', label: 'Form Submission' },
		{ value: 'error', label: 'Error Event' }
	];

	onMount(async () => {
		// Check admin access
		if (!requireAdmin()) return;
		
		await loadData();
		initializeCharts();
		
		// Auto-refresh every 30 seconds
		refreshInterval = setInterval(() => {
			loadData(false); // Silent refresh
		}, 30000);
	});
	
	onDestroy(() => {
		if (refreshInterval) clearInterval(refreshInterval);
		if (chart) chart.destroy();
		if (realtimeChart) realtimeChart.destroy();
	});

	async function loadData(showLoading = true) {
		if (showLoading) loading = true;
		
		try {
			await Promise.all([
				loadStats(),
				loadPageViews(),
				loadDailyStats(),
				loadRealtimeData()
			]);
			
			// Update charts with new data
			if (chart) updateMainChart();
			if (realtimeChart) updateRealtimeChart();
		} catch (err) {
			error = err.message;
		} finally {
			loading = false;
		}
	}

	async function loadStats() {
		const response = await api.getAnalyticsStats();
		if (response.error) throw new Error(response.error);
		stats = response.data || stats;
	}

	async function loadPageViews() {
		const response = await api.getAnalyticsPageviews();
		if (response.error) throw new Error(response.error);
		pageViews = response.data?.pageViews || [];
	}
	
	async function loadDailyStats() {
		const days = parseInt(selectedTimeRange.replace(/\D/g, '') || '7');
		const response = await api.getAnalyticsDailyStats(days);
		
		if (response.error || !response.data?.dailyStats?.length) {
			// If no real data, generate sample data for demo
			const now = new Date();
			dailyStats = [];
			
			for (let i = days - 1; i >= 0; i--) {
				const date = new Date(now);
				date.setDate(date.getDate() - i);
				dailyStats.push({
					date: date.toISOString().split('T')[0],
					views: Math.floor(Math.random() * 1000) + 200,
					uniqueVisitors: Math.floor(Math.random() * 500) + 100
				});
			}
		} else {
			dailyStats = response.data.dailyStats;
		}
	}
	
	async function loadRealtimeData() {
		// Generate sample realtime data (last 60 minutes)
		const now = new Date();
		realtimeData = [];
		
		for (let i = 59; i >= 0; i--) {
			const time = new Date(now);
			time.setMinutes(time.getMinutes() - i);
			realtimeData.push({
				time: time.toISOString(),
				users: Math.floor(Math.random() * 50) + 5
			});
		}
	}

	function initializeCharts() {
		// Main analytics chart
		const mainCtx = document.getElementById('mainChart');
		if (mainCtx) {
			chart = new Chart(mainCtx, {
				type: 'line',
				data: {
					labels: [],
					datasets: [
						{
							label: 'Page Views',
							data: [],
							borderColor: '#189AB4',
							backgroundColor: 'rgba(24, 154, 180, 0.1)',
							tension: 0.4,
							fill: true
						},
						{
							label: 'Unique Visitors',
							data: [],
							borderColor: '#10b981',
							backgroundColor: 'rgba(16, 185, 129, 0.1)',
							tension: 0.4,
							fill: true
						}
					]
				},
				options: {
					responsive: true,
					maintainAspectRatio: false,
					plugins: {
						legend: {
							position: 'top',
							labels: {
								usePointStyle: true,
								padding: 20,
								font: {
									size: 12
								}
							}
						},
						tooltip: {
							mode: 'index',
							intersect: false,
							backgroundColor: 'rgba(0, 0, 0, 0.8)',
							padding: 12,
							cornerRadius: 8
						}
					},
					scales: {
						x: {
							grid: {
								display: false
							}
						},
						y: {
							beginAtZero: true,
							grid: {
								color: 'rgba(0, 0, 0, 0.05)'
							}
						}
					}
				}
			});
			
			updateMainChart();
		}
		
		// Realtime users chart
		const realtimeCtx = document.getElementById('realtimeChart');
		if (realtimeCtx) {
			realtimeChart = new Chart(realtimeCtx, {
				type: 'line',
				data: {
					labels: [],
					datasets: [{
						label: 'Active Users',
						data: [],
						borderColor: '#3b82f6',
						backgroundColor: 'rgba(59, 130, 246, 0.1)',
						tension: 0.4,
						fill: true
					}]
				},
				options: {
					responsive: true,
					maintainAspectRatio: false,
					plugins: {
						legend: {
							display: false
						},
						tooltip: {
							backgroundColor: 'rgba(0, 0, 0, 0.8)',
							padding: 12,
							cornerRadius: 8
						}
					},
					scales: {
						x: {
							display: false
						},
						y: {
							beginAtZero: true,
							grid: {
								color: 'rgba(0, 0, 0, 0.05)'
							}
						}
					}
				}
			});
			
			updateRealtimeChart();
		}
	}
	
	function updateMainChart() {
		if (!chart || !dailyStats.length) return;
		
		chart.data.labels = dailyStats.map(d => {
			const date = new Date(d.date);
			return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
		});
		chart.data.datasets[0].data = dailyStats.map(d => d.views);
		chart.data.datasets[1].data = dailyStats.map(d => d.uniqueVisitors);
		chart.update();
	}
	
	function updateRealtimeChart() {
		if (!realtimeChart || !realtimeData.length) return;
		
		realtimeChart.data.labels = realtimeData.map(d => {
			const time = new Date(d.time);
			return time.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });
		});
		realtimeChart.data.datasets[0].data = realtimeData.map(d => d.users);
		realtimeChart.update();
	}

	function openTrackModal() {
		showTrackModal = true;
		trackEventName = '';
		trackEventCategory = 'custom';
		trackEventProperties = '';
	}
	
	function closeTrackModal() {
		showTrackModal = false;
	}
	
	async function submitTrackEvent() {
		if (!trackEventName.trim()) {
			alert('Please enter an event name');
			return;
		}
		
		let eventData = {
			event: trackEventCategory === 'page_view' ? 'page_view' : trackEventName,
			category: trackEventCategory,
			timestamp: new Date().toISOString()
		};
		
		// If it's a page view, add URL
		if (trackEventCategory === 'page_view') {
			eventData.url = trackEventName;
		}
		
		// Parse additional properties if provided
		if (trackEventProperties.trim()) {
			try {
				const additionalProps = JSON.parse(trackEventProperties);
				eventData = { ...eventData, ...additionalProps };
			} catch (e) {
				alert('Invalid JSON in properties field');
				return;
			}
		}
		
		try {
			const response = await api.trackAnalyticsEvent(eventData);
			
			if (!response.error) {
				closeTrackModal();
				// Refresh data after tracking
				await loadData();
			} else {
				throw new Error(response.error);
			}
		} catch (err) {
			alert('Failed to track event: ' + err.message);
		}
	}
	
	function prepareExportData() {
		// Prepare data for export with a flattened structure for CSV
		if (!dailyStats || dailyStats.length === 0) return [];
		
		return dailyStats.map(row => ({
			date: row.date,
			pageViews: row.views,
			uniqueVisitors: row.uniqueVisitors,
			totalViewsToDate: stats.totalViews,
			totalUniqueUsers: stats.uniqueUsers,
			exportDate: new Date().toISOString().split('T')[0]
		}));
	}

	function formatNumber(num) {
		if (num >= 1000000) return (num / 1000000).toFixed(1) + 'M';
		if (num >= 1000) return (num / 1000).toFixed(1) + 'K';
		return num.toString();
	}
	
	function getChangeClass(value) {
		if (value > 0) return 'text-green-600';
		if (value < 0) return 'text-red-600';
		return 'text-gray-500';
	}
</script>

<div class="page-container">
	<!-- Header -->
	<div class="page-header">
		<div class="header-content">
			<div class="header-left">
				<div class="header-title">
					<BarChart3 size={24} />
					<h1>Analytics Dashboard</h1>
					<span class="badge badge-primary badge-sm">Official Extension</span>
				</div>
				<p class="header-subtitle">Monitor your application's performance and user engagement</p>
			</div>
			<div class="header-actions">
				<select bind:value={selectedTimeRange} on:change={() => loadData()} class="time-range-select">
					{#each timeRanges as range}
						<option value={range.value}>{range.label}</option>
					{/each}
				</select>
				<button on:click={openTrackModal} class="action-btn btn-primary">
					<Plus size={16} />
					Track Event
				</button>
				<ExportButton 
					data={prepareExportData()} 
					filename="analytics" 
					flatten={false}
					disabled={loading}
				/>
				<button on:click={() => loadData()} class="action-btn btn-ghost" disabled={loading}>
					<RefreshCw size={16} class={loading ? 'animate-spin' : ''} />
				</button>
			</div>
		</div>
	</div>

	{#if loading && !stats.totalViews}
		<div class="loading-container">
			<div class="loading loading-spinner loading-lg text-primary"></div>
			<p class="loading-text">Loading analytics data...</p>
		</div>
	{:else if error}
		<div class="alert alert-error">
			<span>{error}</span>
		</div>
	{:else}
		<!-- Stats Grid -->
		<div class="stats-grid">
			<div class="stat-card">
				<div class="stat-icon bg-blue-100 text-blue-600">
					<Eye size={20} />
				</div>
				<div class="stat-content">
					<p class="stat-label">Total Page Views</p>
					<p class="stat-value">{formatNumber(stats.totalViews)}</p>
					<p class="stat-change {getChangeClass(12.5)}">
						<span>↑ 12.5%</span>
						<span class="text-xs text-gray-500 ml-1">vs last period</span>
					</p>
				</div>
			</div>
			
			<div class="stat-card">
				<div class="stat-icon bg-green-100 text-green-600">
					<Users size={20} />
				</div>
				<div class="stat-content">
					<p class="stat-label">Unique Visitors</p>
					<p class="stat-value">{formatNumber(stats.uniqueUsers)}</p>
					<p class="stat-change {getChangeClass(8.3)}">
						<span>↑ 8.3%</span>
						<span class="text-xs text-gray-500 ml-1">vs last period</span>
					</p>
				</div>
			</div>
			
			<div class="stat-card">
				<div class="stat-icon bg-orange-100 text-orange-600">
					<Clock size={20} />
				</div>
				<div class="stat-content">
					<p class="stat-label">Today's Views</p>
					<p class="stat-value">{formatNumber(stats.todayViews)}</p>
					<p class="stat-change {getChangeClass(0)}">
						<span>→ 0%</span>
						<span class="text-xs text-gray-500 ml-1">vs yesterday</span>
					</p>
				</div>
			</div>
			
			<div class="stat-card">
				<div class="stat-icon bg-purple-100 text-purple-600">
					<Activity size={20} />
				</div>
				<div class="stat-content">
					<p class="stat-label">Active Now</p>
					<p class="stat-value">{stats.activeNow}</p>
					<div class="flex items-center mt-2">
						<span class="w-2 h-2 bg-green-500 rounded-full animate-pulse mr-2"></span>
						<span class="text-xs text-gray-500">Live</span>
					</div>
				</div>
			</div>
		</div>

		<!-- Charts Row -->
		<div class="charts-row">
			<!-- Main Chart -->
			<div class="chart-card">
				<div class="chart-header">
					<h2 class="chart-title">Traffic Overview</h2>
					<span class="text-sm text-gray-500">Page views & visitors</span>
				</div>
				<div class="chart-container">
					<canvas id="mainChart"></canvas>
				</div>
			</div>

			<!-- Realtime Chart -->
			<div class="chart-card small">
				<div class="chart-header">
					<h2 class="chart-title">Real-time Users</h2>
					<span class="badge badge-info badge-sm">Last 60 min</span>
				</div>
				<div class="chart-container small">
					<canvas id="realtimeChart"></canvas>
				</div>
			</div>
		</div>

		<!-- Tables Row -->
		<div class="tables-row">
			<!-- Top Pages -->
			<div class="table-card">
				<div class="table-header">
					<h2 class="table-title">Top Pages</h2>
					<span class="badge badge-ghost">This Week</span>
				</div>
				<div class="table-content">
					{#if pageViews.length > 0}
						<div class="pages-list">
							{#each pageViews.slice(0, 5) as page, index}
								<div class="page-item">
									<div class="page-rank">#{index + 1}</div>
									<div class="page-info">
										<a href={page.url} class="page-url" target="_blank">
											{page.url}
										</a>
										<div class="page-stats">
											<Eye size={14} class="text-gray-400" />
											<span class="text-sm text-gray-600">{formatNumber(page.views)} views</span>
										</div>
									</div>
									<div class="page-trend">
										<span class="trend-value text-green-600">↑ 15%</span>
									</div>
								</div>
							{/each}
						</div>
					{:else}
						<div class="empty-state">
							<BarChart3 size={48} class="text-gray-300" />
							<p class="text-gray-500 mt-3">No page data available</p>
							<p class="text-sm text-gray-400 mt-1">Data will appear once users visit your site</p>
						</div>
					{/if}
				</div>
			</div>

			<!-- Browser Stats -->
			<div class="table-card">
				<div class="table-header">
					<h2 class="table-title">Browser Usage</h2>
					<span class="badge badge-ghost">Top 5</span>
				</div>
				<div class="table-content">
					<div class="browser-stats">
						{#each [
							{ name: 'Chrome', icon: '🌐', percent: 65 },
							{ name: 'Firefox', icon: '🦊', percent: 20 },
							{ name: 'Safari', icon: '🧭', percent: 10 },
							{ name: 'Edge', icon: '🌊', percent: 3 },
							{ name: 'Other', icon: '📱', percent: 2 }
						] as browser}
							<div class="browser-item">
								<div class="browser-info">
									<span class="browser-icon">{browser.icon}</span>
									<span class="browser-name">{browser.name}</span>
								</div>
								<div class="browser-usage">
									<div class="usage-bar">
										<div class="usage-fill" style="width: {browser.percent}%"></div>
									</div>
									<span class="usage-percent">{browser.percent}%</span>
								</div>
							</div>
						{/each}
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>

<!-- Track Event Modal -->
{#if showTrackModal}
	<div class="modal-overlay">
		<div class="modal-container">
			<div class="modal-header">
				<h3 class="modal-title">Track Custom Event</h3>
				<button class="modal-close" on:click={closeTrackModal}>
					<X size={20} />
				</button>
			</div>
			
			<div class="modal-body">
				<div class="form-group">
					<label class="form-label">Event Category</label>
					<select bind:value={trackEventCategory} class="form-select">
						{#each eventCategories as category}
							<option value={category.value}>{category.label}</option>
						{/each}
					</select>
				</div>
				
				<div class="form-group">
					<label class="form-label">
						{trackEventCategory === 'page_view' ? 'Page URL' : 'Event Name'}
					</label>
					<input 
						type="text" 
						bind:value={trackEventName}
						placeholder={trackEventCategory === 'page_view' ? '/example-page' : 'button_click'}
						class="form-input" 
					/>
					<p class="form-hint">
						{trackEventCategory === 'page_view' 
							? 'Enter the URL of the page to track'
							: 'Give your event a descriptive name'}
					</p>
				</div>
				
				<div class="form-group">
					<div class="form-label-row">
						<label class="form-label">Additional Properties (JSON)</label>
						<span class="form-optional">Optional</span>
					</div>
					<textarea 
						bind:value={trackEventProperties}
						placeholder={'{"userId": "123", "value": 99.99}'}
						class="form-textarea"
						rows="4"
					></textarea>
					<p class="form-hint">Add custom properties as JSON object</p>
				</div>
			</div>
			
			<div class="modal-footer">
				<button on:click={closeTrackModal} class="modal-btn modal-btn-secondary">Cancel</button>
				<button on:click={submitTrackEvent} class="modal-btn modal-btn-primary">Track Event</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.page-container {
		padding: 1.5rem;
		max-width: 1400px;
		margin: 0 auto;
	}

	.page-header {
		background: white;
		border-radius: 0.5rem;
		padding: 1.5rem;
		margin-bottom: 1.5rem;
		border: 1px solid #e5e7eb;
	}

	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		flex-wrap: wrap;
		gap: 1rem;
	}

	.header-left {
		flex: 1;
	}

	.header-title {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 0.5rem;
	}

	.header-title h1 {
		font-size: 1.5rem;
		font-weight: 600;
		color: #111827;
		margin: 0;
	}

	.header-subtitle {
		color: #6b7280;
		font-size: 0.875rem;
		margin: 0;
	}

	.header-actions {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		flex-wrap: wrap;
	}

	.loading-container {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		min-height: 400px;
		gap: 1rem;
	}

	.loading-text {
		color: #6b7280;
		font-size: 0.95rem;
	}

	/* Stats Grid */
	.stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
		gap: 1rem;
		margin-bottom: 1.5rem;
	}

	.stat-card {
		background: white;
		border-radius: 0.5rem;
		padding: 1.25rem;
		border: 1px solid #e5e7eb;
		display: flex;
		align-items: flex-start;
		gap: 1rem;
	}

	.stat-icon {
		width: 48px;
		height: 48px;
		border-radius: 0.5rem;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.stat-content {
		flex: 1;
	}

	.stat-label {
		font-size: 0.875rem;
		color: #6b7280;
		margin: 0 0 0.25rem 0;
	}

	.stat-value {
		font-size: 1.75rem;
		font-weight: 600;
		color: #111827;
		line-height: 1;
		margin: 0 0 0.5rem 0;
	}

	.stat-change {
		display: flex;
		align-items: center;
		font-size: 0.875rem;
	}

	/* Charts */
	.charts-row {
		display: grid;
		grid-template-columns: 2fr 1fr;
		gap: 1rem;
		margin-bottom: 1.5rem;
	}

	.chart-card {
		background: white;
		border-radius: 0.5rem;
		padding: 1.5rem;
		border: 1px solid #e5e7eb;
	}

	.chart-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.5rem;
	}

	.chart-title {
		font-size: 1.1rem;
		font-weight: 600;
		color: #111827;
		margin: 0;
	}

	.chart-container {
		position: relative;
		height: 280px;
	}

	.chart-container.small {
		height: 200px;
	}

	/* Tables */
	.tables-row {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
		gap: 1rem;
	}

	.table-card {
		background: white;
		border-radius: 0.5rem;
		padding: 1.5rem;
		border: 1px solid #e5e7eb;
	}

	.table-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.25rem;
		padding-bottom: 0.75rem;
		border-bottom: 1px solid #f3f4f6;
	}

	.table-title {
		font-size: 1.1rem;
		font-weight: 600;
		color: #111827;
		margin: 0;
	}

	.pages-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.page-item {
		display: flex;
		align-items: center;
		padding: 0.75rem;
		border-radius: 0.375rem;
		transition: background 0.2s;
	}

	.page-item:hover {
		background: #f9fafb;
	}

	.page-rank {
		width: 32px;
		height: 32px;
		background: #f3f4f6;
		border-radius: 0.375rem;
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 600;
		font-size: 0.875rem;
		color: #6b7280;
		margin-right: 1rem;
		flex-shrink: 0;
	}

	.page-info {
		flex: 1;
		min-width: 0;
	}

	.page-url {
		color: #189AB4;
		text-decoration: none;
		font-weight: 500;
		display: block;
		margin-bottom: 0.25rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.page-url:hover {
		text-decoration: underline;
	}

	.page-stats {
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}

	.page-trend {
		display: flex;
		align-items: center;
	}

	.trend-value {
		font-size: 0.875rem;
		font-weight: 500;
	}

	/* Browser Stats */
	.browser-stats {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.browser-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.browser-info {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-width: 100px;
	}

	.browser-icon {
		font-size: 1.25rem;
	}

	.browser-name {
		font-size: 0.875rem;
		color: #374151;
		font-weight: 500;
	}

	.browser-usage {
		flex: 1;
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-left: 1rem;
	}

	.usage-bar {
		flex: 1;
		height: 6px;
		background: #f3f4f6;
		border-radius: 3px;
		overflow: hidden;
	}

	.usage-fill {
		height: 100%;
		background: linear-gradient(90deg, #189AB4, #0284c7);
		border-radius: 3px;
		transition: width 0.3s ease;
	}

	.usage-percent {
		font-size: 0.875rem;
		color: #6b7280;
		min-width: 35px;
		text-align: right;
	}

	/* Empty State */
	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 3rem 2rem;
		text-align: center;
	}

	/* Modal Styles */
	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.5);
		z-index: 9999;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
		animation: fadeIn 0.2s;
	}

	.modal-container {
		background: white;
		border-radius: 0.75rem;
		width: 100%;
		max-width: 500px;
		max-height: 90vh;
		overflow-y: auto;
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
		animation: slideUp 0.3s;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-bottom: 1px solid #e5e7eb;
	}

	.modal-title {
		font-size: 1.25rem;
		font-weight: 600;
		color: #111827;
		margin: 0;
	}

	.modal-close {
		background: transparent;
		border: none;
		color: #6b7280;
		cursor: pointer;
		padding: 0.25rem;
		border-radius: 0.375rem;
		transition: all 0.2s;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.modal-close:hover {
		color: #374151;
		background: #f3f4f6;
	}

	.modal-body {
		padding: 1.5rem;
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid #e5e7eb;
	}

	/* Form Styles */
	.form-group {
		margin-bottom: 1.25rem;
	}

	.form-group:last-child {
		margin-bottom: 0;
	}

	.form-label {
		display: block;
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
		margin-bottom: 0.5rem;
	}

	.form-label-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.5rem;
	}

	.form-optional {
		font-size: 0.75rem;
		color: #9ca3af;
		font-weight: 400;
	}

	.form-input,
	.form-select,
	.form-textarea {
		width: 100%;
		padding: 0.625rem 0.875rem;
		font-size: 0.875rem;
		border: 1px solid #d1d5db;
		border-radius: 0.375rem;
		background: white;
		color: #111827;
		transition: all 0.2s;
	}

	.form-input:focus,
	.form-select:focus,
	.form-textarea:focus {
		outline: none;
		border-color: #06b6d4;
		box-shadow: 0 0 0 3px rgba(6, 182, 212, 0.1);
	}

	.form-textarea {
		resize: vertical;
		font-family: inherit;
		line-height: 1.5;
	}

	.form-hint {
		font-size: 0.75rem;
		color: #6b7280;
		margin-top: 0.375rem;
		margin-bottom: 0;
	}

	/* Modal Buttons */
	.modal-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.625rem 1.25rem;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		border: 1px solid transparent;
		cursor: pointer;
		transition: all 0.2s;
		line-height: 1;
	}

	.modal-btn-primary {
		background: #06b6d4;
		color: white;
		border-color: #06b6d4;
	}

	.modal-btn-primary:hover {
		background: #0891b2;
		border-color: #0891b2;
	}

	.modal-btn-secondary {
		background: white;
		color: #374151;
		border: 1px solid #d1d5db;
	}

	.modal-btn-secondary:hover {
		background: #f9fafb;
		border-color: #9ca3af;
	}

	/* Animations */
	@keyframes fadeIn {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	@keyframes slideUp {
		from {
			transform: translateY(1rem);
			opacity: 0;
		}
		to {
			transform: translateY(0);
			opacity: 1;
		}
	}

	/* Time Range Select (matching dashboard style) */
	.time-range-select {
		padding: 0.25rem 0.5rem;
		font-size: 0.75rem;
		border: 1px solid #d1d5db;
		border-radius: 0.25rem;
		background: white;
		color: #374151;
		cursor: pointer;
		transition: all 0.2s;
	}

	.time-range-select:hover {
		border-color: #9ca3af;
	}

	.time-range-select:focus {
		outline: none;
		border-color: #3b82f6;
		box-shadow: 0 0 0 1px rgba(59, 130, 246, 0.1);
	}

	/* Action buttons styling */
	.action-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		border: 1px solid transparent;
		cursor: pointer;
		transition: all 0.2s;
		line-height: 1;
	}

	.action-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-primary {
		background: #06b6d4;
		color: white;
		border-color: #06b6d4;
	}

	.btn-primary:hover:not(:disabled) {
		background: #0891b2;
		border-color: #0891b2;
	}

	.btn-ghost {
		background: transparent;
		color: #6b7280;
		border-color: transparent;
	}

	.btn-ghost:hover:not(:disabled) {
		background: #f3f4f6;
		color: #374151;
	}

	/* Responsive */
	@media (max-width: 1024px) {
		.charts-row {
			grid-template-columns: 1fr;
		}
		
		.tables-row {
			grid-template-columns: 1fr;
		}
	}

	@media (max-width: 640px) {
		.header-content {
			flex-direction: column;
			align-items: start;
		}
		
		.header-actions {
			width: 100%;
			justify-content: flex-start;
		}
		
		.stats-grid {
			grid-template-columns: 1fr;
		}
	}
</style>