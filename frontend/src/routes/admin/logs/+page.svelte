<script lang="ts">
	import { onMount } from 'svelte';
	import { Chart, registerables } from 'chart.js';
	import { 
		FileText, Search, Filter, Download,
		AlertCircle, Info, AlertTriangle, XCircle,
		Clock, RefreshCw
	} from 'lucide-svelte';
	import ExportButton from '$lib/components/ExportButton.svelte';
	import { api } from '$lib/api';
	
	Chart.register(...registerables);
	
	let activityChart: Chart | null = null;
	let searchQuery = '';
	let selectedLevel = '';
	let selectedTimeRange = '24h';
	let autoRefresh = false;
	let refreshInterval: number | null = null;
	let loading = false;
	let currentPage = 1;
	let totalPages = 1;
	let pageSize = 100;
	
	interface LogEntry {
		id?: string;
		level: string;
		message: string;
		timestamp: string;
		source?: string;
		fields?: Record<string, unknown>;
		[key: string]: unknown;
	}

	interface RequestLog {
		id?: string;
		method: string;
		path: string;
		status: number;
		statusCode?: number;
		execTimeMs?: number;
		duration?: string;
		timestamp: string;
		createdAt?: string;
		userAgent?: string;
		userIp?: string;
		level?: string;
		[key: string]: unknown;
	}

	interface LogsResponse {
		logs: LogEntry[];
		total: number;
	}

	interface RequestLogsResponse {
		logs: RequestLog[];
		total: number;
	}

	// Real log data from API
	let logs: LogEntry[] = [];
	let requestLogs: RequestLog[] = [];
	let activeTab = 'requests';
	
	// Stats
	let totalLogs = 0;
	let errorCount = 0;
	let warningCount = 0;
	let infoCount = 0;
	
	// Separate counts for each tab
	let requestLogsCount = 0;
	let applicationLogsCount = 0;
	
	function getLogLevel(log: RequestLog | LogEntry): string {
		if ('level' in log && log.level) return log.level;
		if ('status' in log) {
			const status = log.status as number;
			return status >= 500 ? 'error' : status >= 400 ? 'warning' : 'info';
		}
		return 'info';
	}

	function getLevelIcon(level: string) {
		switch(level) {
			case 'error': return XCircle;
			case 'warning': return AlertTriangle;
			case 'info': return Info;
			default: return AlertCircle;
		}
	}

	function getLevelColor(level: string) {
		switch(level) {
			case 'error': return 'var(--danger-color)';
			case 'warning': return 'var(--warning-color)';
			case 'info': return 'var(--info-color)';
			case 'debug': return 'var(--secondary-color)';
			default: return 'var(--text-muted)';
		}
	}
	
	
	
	function toggleAutoRefresh() {
		autoRefresh = !autoRefresh;
		
		if (autoRefresh) {
			refreshInterval = setInterval(() => {
				loadLogs();
			}, 5000) as unknown as number;
		} else if (refreshInterval) {
			clearInterval(refreshInterval);
			refreshInterval = null;
		}
		if (autoRefresh) {
			refreshInterval = setInterval(() => {
				// Refresh logs
				console.log('Refreshing logs...');
			}, 5000) as unknown as number;
		} else if (refreshInterval) {
			clearInterval(refreshInterval);
			refreshInterval = null;
		}
	}
	
	function exportLogs() {
		console.log('Exporting logs...');
	}
	
	async function loadLogs() {
		loading = true;
		try {
			if (activeTab === 'requests') {
				// Load request logs
				const params = new URLSearchParams({
					page: currentPage.toString(),
					size: pageSize.toString(),
					range: selectedTimeRange
				});
				
				if (searchQuery) {
					params.append('path', searchQuery);
				}
				
				const response = await api.get<RequestLogsResponse>(`/admin/logs/requests?${params}`);
				requestLogs = response.logs || [];
				totalLogs = response.total || 0;
				requestLogsCount = totalLogs;
				totalPages = Math.ceil(totalLogs / pageSize);
				
				// Calculate stats for request logs
				errorCount = requestLogs.filter(l => l.status >= 500).length;
				warningCount = requestLogs.filter(l => l.status >= 400 && l.status < 500).length;
				infoCount = requestLogs.filter(l => l.status < 400).length;
			} else if (activeTab === 'application') {
				// Load application logs
				const params = new URLSearchParams({
					page: currentPage.toString(),
					size: pageSize.toString(),
					range: selectedTimeRange
				});
				
				if (selectedLevel && selectedLevel !== 'all') {
					params.append('level', selectedLevel);
				}
				
				if (searchQuery) {
					params.append('search', searchQuery);
				}
				
				const response = await api.get<LogsResponse>(`/admin/logs?${params}`);
				logs = response.logs || [];
				totalLogs = response.total || 0;
				applicationLogsCount = totalLogs;
				totalPages = Math.ceil(totalLogs / pageSize);
				
				// Calculate stats
				errorCount = logs.filter(l => l.level === 'error').length;
				warningCount = logs.filter(l => l.level === 'warning' || l.level === 'warn').length;
				infoCount = logs.filter(l => l.level === 'info').length;
			}
		} catch (error) {
			console.error('Failed to load logs:', error);
			logs = [];
			requestLogs = [];
		} finally {
			loading = false;
		}
	}
	
	async function handleSearch() {
		currentPage = 1;
		await loadLogs();
	}
	
	async function handleFilterChange() {
		currentPage = 1;
		await loadLogs();
		await updateChart();
	}
	
	async function handleTabChange(tab: string) {
		activeTab = tab;
		currentPage = 1;
		await loadLogs();
		await loadTabCounts();
	}
	
	async function loadTabCounts() {
		try {
			// Load counts for both tabs
			const [requestResponse, appResponse] = await Promise.all([
				api.get<RequestLogsResponse>(`/admin/logs/requests?page=1&size=1&range=${selectedTimeRange}`),
				api.get<LogsResponse>(`/admin/logs?page=1&size=1&range=${selectedTimeRange}`)
			]);

			requestLogsCount = requestResponse.total || 0;
			applicationLogsCount = appResponse.total || 0;
		} catch (error) {
			console.error('Failed to load tab counts:', error);
		}
	}
	
	async function loadChartData() {
		try {
			// Get logs statistics for the chart
			const [appLogsResponse, requestLogsResponse] = await Promise.all([
				api.get<LogsResponse>(`/admin/logs?page=1&size=1000&range=${selectedTimeRange}`),
				api.get<RequestLogsResponse>(`/admin/logs/requests?page=1&size=1000&range=${selectedTimeRange}`)
			]);

			const chartAppLogs = appLogsResponse.logs || [];
			const chartRequestLogs = requestLogsResponse.logs || [];

			// Process logs by hour for the chart
			const hours = Array.from({length: 24}, (_, i) => i);
			const appLogsByHour = new Array(24).fill(0);
			const requestLogsByHour = new Array(24).fill(0);
			const errorsByHour = new Array(24).fill(0);

			// Count application logs by hour
			chartAppLogs.forEach((log: LogEntry) => {
				const hour = new Date(log.timestamp).getHours();
				appLogsByHour[hour]++;
				if (log.level === 'error' || log.level === 'fatal') {
					errorsByHour[hour]++;
				}
			});

			// Count request logs by hour
			chartRequestLogs.forEach((log: RequestLog) => {
				const hour = new Date(log.timestamp).getHours();
				requestLogsByHour[hour]++;
				if (log.status >= 500) {
					errorsByHour[hour]++;
				}
			});
			
			return {
				labels: hours.map(h => `${h}:00`),
				appLogs: appLogsByHour,
				requestLogs: requestLogsByHour,
				errors: errorsByHour
			};
		} catch (error) {
			console.error('Failed to load chart data:', error);
			return {
				labels: Array.from({length: 24}, (_, i) => `${i}:00`),
				appLogs: new Array(24).fill(0),
				requestLogs: new Array(24).fill(0),
				errors: new Array(24).fill(0)
			};
		}
	}
	
	async function updateChart() {
		const chartData = await loadChartData();
		
		if (activityChart) {
			activityChart.data.labels = chartData.labels;
			activityChart.data.datasets = [
				{
					label: 'Application Logs',
					data: chartData.appLogs,
					borderColor: '#3b82f6',
					backgroundColor: 'rgba(59, 130, 246, 0.1)',
					tension: 0.4
				},
				{
					label: 'Request Logs',
					data: chartData.requestLogs,
					borderColor: '#10b981',
					backgroundColor: 'rgba(16, 185, 129, 0.1)',
					tension: 0.4
				},
				{
					label: 'Errors',
					data: chartData.errors,
					borderColor: '#ef4444',
					backgroundColor: 'rgba(239, 68, 68, 0.1)',
					tension: 0.4
				}
			];
			activityChart.update();
		}
	}
	
	onMount(() => {
		// Load initial data asynchronously
		(async () => {
			await loadLogs();
			await loadTabCounts();
			const chartData = await loadChartData();
		
		// Initialize activity chart with real data
		const ctx = document.getElementById('activity-chart') as HTMLCanvasElement;
		if (ctx) {
			activityChart = new Chart(ctx, {
				type: 'line',
				data: {
					labels: chartData.labels,
					datasets: [
						{
							label: 'Application Logs',
							data: chartData.appLogs,
							borderColor: '#3b82f6',
							backgroundColor: 'rgba(59, 130, 246, 0.1)',
							tension: 0.4
						},
						{
							label: 'Request Logs',
							data: chartData.requestLogs,
							borderColor: '#10b981',
							backgroundColor: 'rgba(16, 185, 129, 0.1)',
							tension: 0.4
						},
						{
							label: 'Errors',
							data: chartData.errors,
							borderColor: '#ef4444',
							backgroundColor: 'rgba(239, 68, 68, 0.1)',
							tension: 0.4
						}
					]
				},
				options: {
					responsive: true,
					maintainAspectRatio: false,
					plugins: {
						legend: {
							position: 'bottom',
							labels: {
								usePointStyle: true,
								padding: 15
							}
						},
						tooltip: {
							mode: 'index',
							intersect: false
						}
					},
					scales: {
						y: {
							beginAtZero: true,
							grid: {
								color: 'rgba(0, 0, 0, 0.05)'
							}
						},
						x: {
							grid: {
								display: false
							}
						}
					},
					interaction: {
						mode: 'nearest',
						axis: 'x',
						intersect: false
					}
				}
			});
		}
		})();

		return () => {
			if (activityChart) activityChart.destroy();
			if (refreshInterval) clearInterval(refreshInterval);
		};
	});
</script>

<div class="logs-page">
	<!-- Header -->
	<div class="page-header">
		<div class="header-content">
			<div class="header-left">
				<div class="header-title">
					<FileText size={24} />
					<h1>System Logs</h1>
				</div>
				<div class="header-meta">
					<span class="meta-item">{totalLogs.toLocaleString()} total</span>
					<span class="meta-separator">•</span>
					<span class="meta-item error">{errorCount} errors</span>
					<span class="meta-separator">•</span>
					<span class="meta-item warning">{warningCount} warnings</span>
					<span class="meta-separator">•</span>
					<span class="meta-item info">{infoCount} info</span>
				</div>
			</div>
			<div class="header-info">
				<span class="info-item {autoRefresh ? 'active' : 'paused'}">
					<RefreshCw size={14} class={autoRefresh ? 'spinning' : ''} />
					{autoRefresh ? 'Auto-refresh' : 'Paused'}
				</span>
			</div>
		</div>
	</div>

	<!-- Content Area -->
	<div class="content-area">
	
	<!-- Activity Chart -->
	<div class="card">
		<div class="card-header">
			<h3 class="card-title">Log Activity</h3>
			<select class="form-select" style="width: auto" bind:value={selectedTimeRange} on:change={handleFilterChange}>
				<option value="1h">Last Hour</option>
				<option value="24h">Last 24 Hours</option>
				<option value="7d">Last 7 Days</option>
				<option value="30d">Last 30 Days</option>
			</select>
		</div>
		<div style="height: 250px;">
			<canvas id="activity-chart"></canvas>
		</div>
	</div>
	
	<!-- Logs Table -->
	<div class="card">
		<!-- Tab Navigation -->
		<div class="tabs">
			<button 
				class="tab {activeTab === 'requests' ? 'active' : ''}" 
				on:click={() => handleTabChange('requests')}
			>
				Request Logs ({requestLogsCount})
			</button>
			<button 
				class="tab {activeTab === 'application' ? 'active' : ''}" 
				on:click={() => handleTabChange('application')}
			>
				Application Logs ({applicationLogsCount})
			</button>
		</div>
		
		<div class="logs-header">
			<div class="logs-filters">
				<div class="search-box">
					<Search size={16} />
					<input 
						type="text" 
						placeholder="Search logs..."
						bind:value={searchQuery}
					/>
				</div>
				
				<select class="filter-select" bind:value={selectedLevel} on:change={handleFilterChange}>
					<option value="">All Levels</option>
					<option value="error">Errors</option>
					<option value="warning">Warnings</option>
					<option value="info">Info</option>
					<option value="debug">Debug</option>
				</select>
				
				<button 
					class="btn btn-secondary btn-sm {autoRefresh ? 'active' : ''}"
					on:click={toggleAutoRefresh}
				>
					<RefreshCw size={16} class={autoRefresh ? 'spinning' : ''} />
					{autoRefresh ? 'Auto' : 'Manual'}
				</button>
			</div>
			
			<ExportButton 
				data={logs}
				filename="logs"
				disabled={logs.length === 0}
			/>
		</div>
		
		<div class="table-container">
			<table class="table logs-table">
				<thead>
					<tr>
						<th style="width: 40px;">Level</th>
						<th style="width: 150px;">Timestamp</th>
						<th style="width: 100px;">Source</th>
						<th>Message</th>
						<th style="width: 200px;">Metadata</th>
					</tr>
				</thead>
				<tbody>
					{#if activeTab === 'requests'}
						{#each requestLogs as log}
							{@const level = getLogLevel(log)}
							<tr>
								<td>
									<div class="log-level" style="color: {getLevelColor(level)}">
										<svelte:component this={getLevelIcon(level)} size={16} />
									</div>
								</td>
								<td class="log-timestamp">
									<Clock size={12} style="display: inline; margin-right: 4px; color: var(--text-muted)" />
									{new Date(log.createdAt || log.timestamp).toLocaleString() || 'N/A'}
								</td>
								<td>
									<span class="log-source">{log.method || 'HTTP'}</span>
								</td>
								<td class="log-message">{log.path} - Status: {log.status}</td>
								<td>
									<div class="log-metadata">
										<span class="metadata-item">
											<span class="metadata-key">Duration:</span>
											<span class="metadata-value">{log.duration || '0ms'}</span>
										</span>
										{#if log.userAgent}
											<span class="metadata-item">
												<span class="metadata-key">IP:</span>
												<span class="metadata-value">{log.userIP || 'N/A'}</span>
											</span>
										{/if}
									</div>
								</td>
							</tr>
						{/each}
					{:else}
						{#each logs as log}
							<tr>
								<td>
									<div class="log-level" style="color: {getLevelColor(log.level)}">
										<svelte:component this={getLevelIcon(log.level)} size={16} />
									</div>
								</td>
								<td class="log-timestamp">
									<Clock size={12} style="display: inline; margin-right: 4px; color: var(--text-muted)" />
									{new Date(log.timestamp).toLocaleString() || 'N/A'}
								</td>
								<td>
									<span class="log-source">{log.source || 'system'}</span>
								</td>
								<td class="log-message">{log.message}</td>
								<td>
									{#if log.fields}
										<div class="log-metadata">
											{#each Object.entries(log.fields) as [key, value]}
												<span class="metadata-item">
													<span class="metadata-key">{key}:</span>
													<span class="metadata-value">{value}</span>
												</span>
											{/each}
										</div>
									{:else}
										-
									{/if}
								</td>
							</tr>
						{/each}
					{/if}
				</tbody>
			</table>
		</div>
	</div>
</div>
</div>

<style>
	/* Page Layout */
	.logs-page {
		height: 100%;
		display: flex;
		flex-direction: column;
		background: #f8fafc;
	}

	/* Header */
	.page-header {
		background: white;
		border-bottom: 1px solid #e2e8f0;
		padding: 1.5rem 2rem;
	}

	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.header-left {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.header-title {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.header-title h1 {
		font-size: 1.5rem;
		font-weight: 600;
		color: #0f172a;
		margin: 0;
	}

	.header-meta {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		margin-left: 2.25rem;
	}

	.meta-item {
		font-size: 0.8125rem;
		color: #64748b;
	}

	.meta-item.error {
		color: #ef4444;
	}

	.meta-item.warning {
		color: #f59e0b;
	}

	.meta-item.info {
		color: #3b82f6;
	}

	.meta-separator {
		color: #cbd5e1;
		margin: 0 0.25rem;
	}

	.header-info {
		display: flex;
		gap: 1.5rem;
	}

	.info-item {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
	}

	.info-item.active {
		color: #22c55e;
	}

	.info-item.paused {
		color: #94a3b8;
	}

	/* Content Area */
	.content-area {
		flex: 1;
		padding: 0.75rem 1.5rem 1.5rem;
		overflow: auto;
	}

	.card {
		background: white;
		border-radius: 0.75rem;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
		padding: 1.5rem;
		margin-bottom: 1.5rem;
	}

	/* Search and Filters */
	.search-box {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		max-width: 280px;
		height: 36px;
		padding: 0 0.75rem;
		border: 1px solid #e2e8f0;
		border-radius: 6px;
		background: white;
		transition: all 0.15s;
	}
	
	.search-box:focus-within {
		border-color: #3b82f6;
		box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
	}

	.search-box svg {
		color: #94a3b8;
		flex-shrink: 0;
	}
	
	.search-box input {
		border: none;
		background: none;
		outline: none;
		flex: 1;
		font-size: 0.875rem;
		color: #475569;
		padding: 0;
	}
	
	.search-box input::placeholder {
		color: #94a3b8;
	}

	.filter-select {
		height: 36px;
		padding: 0 0.75rem;
		padding-right: 2rem;
		border: 1px solid #e2e8f0;
		border-radius: 0.375rem;
		background: white;
		font-size: 0.8125rem;
		color: #475569;
		cursor: pointer;
		appearance: none;
		transition: all 0.15s;
		background-image: url("data:image/svg+xml;charset=UTF-8,%3csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%2394a3b8' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3e%3cpolyline points='6 9 12 15 18 9'%3e%3c/polyline%3e%3c/svg%3e");
		background-repeat: no-repeat;
		background-position: right 0.5rem center;
		background-size: 1rem;
	}
	
	.filter-select:hover {
		border-color: #cbd5e1;
		background-color: #f8fafc;
	}
	
	.filter-select:focus {
		outline: none;
		border-color: #3b82f6;
		box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.08);
	}

	.tabs {
		display: flex;
		gap: 0;
		border-bottom: 1px solid var(--border-color);
		margin-bottom: 1rem;
	}
	
	.tab {
		padding: 0.75rem 1.5rem;
		background: transparent;
		border: none;
		border-bottom: 2px solid transparent;
		color: var(--text-secondary);
		cursor: pointer;
		font-weight: 500;
		transition: all 0.2s;
	}
	
	.tab:hover {
		color: var(--text-primary);
		background: var(--bg-hover);
	}
	
	.tab.active {
		color: var(--primary-color);
		border-bottom-color: var(--primary-color);
	}
	
	.logs-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 1rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid var(--border-color);
	}
	
	.logs-filters {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}
	
	.logs-table {
		font-size: 0.875rem;
	}
	
	.log-level {
		display: flex;
		align-items: center;
		justify-content: center;
	}
	
	.log-timestamp {
		color: var(--text-secondary);
		font-size: 0.8125rem;
		white-space: nowrap;
	}
	
	.log-source {
		display: inline-block;
		padding: 0.125rem 0.5rem;
		background: var(--bg-secondary);
		border-radius: 4px;
		font-size: 0.75rem;
		font-weight: 500;
		color: var(--text-secondary);
	}
	
	.log-message {
		color: var(--text-primary);
	}
	
	.log-metadata {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	
	.metadata-item {
		display: flex;
		gap: 0.25rem;
		font-size: 0.75rem;
	}
	
	.metadata-key {
		color: var(--text-muted);
		font-weight: 500;
	}
	
	.metadata-value {
		color: var(--text-secondary);
	}
	
	.btn.active {
		background: var(--primary-color);
		color: white;
		border-color: var(--primary-color);
	}
	
	@keyframes spin {
		from { transform: rotate(0deg); }
		to { transform: rotate(360deg); }
	}
	
	.spinning {
		animation: spin 1s linear infinite;
	}
</style>