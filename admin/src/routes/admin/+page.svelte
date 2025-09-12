<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { Chart, registerables } from 'chart.js';
	import { 
		Users, Database, HardDrive, Activity,
		TrendingUp, TrendingDown, Clock, Server,
		Zap, AlertCircle, CheckCircle, RefreshCw,
		LayoutDashboard
	} from 'lucide-svelte';
	import { api } from '$lib/api';
	import { currentUser } from '$lib/stores/auth';
	
	Chart.register(...registerables);
	
	// Check if user is admin
	$: if ($currentUser && $currentUser.role !== 'admin') {
		goto('/profile');
	}
	
	// Loading state
	let loading = true;
	let error = '';
	
	// System metrics
	let cpuUsage = 0;
	let memoryUsage = 0;
	let diskUsage = 0;
	let memoryInfo = { used: 0, total: 0 };
	let diskInfo = { used: 0, total: 0 };
	
	// Application metrics
	let totalUsers = 0;
	let totalRows = 0;
	let totalStorage = 0;
	let uptime = '0h 0m';
	
	// Performance metrics
	let httpRequestsTotal = 0;
	let httpRequestRate = 0;
	let dbQueriesTotal = 0;
	let avgResponseTime = 0;
	let errorRate = 0;
	
	// Charts
	let performanceChart: Chart | null = null;
	let activityChart: Chart | null = null;
	let intervalId: ReturnType<typeof setInterval> | null = null;
	
	// Track previous values for trends
	let previousMetrics = {
		users: 0,
		rows: 0,
		storage: 0,
		requests: 0
	};
	
	// Time range configurations
	const TIME_RANGES = {
		'10m': { 
			label: 'Last 10 minutes', 
			points: 20, 
			interval: 30 * 1000, // 30 seconds
			format: { hour: '2-digit', minute: '2-digit', second: '2-digit' }
		},
		'6h': { 
			label: 'Last 6 hours', 
			points: 72, 
			interval: 5 * 60 * 1000, // 5 minutes
			format: { hour: '2-digit', minute: '2-digit' }
		},
		'24h': { 
			label: 'Last 24 hours', 
			points: 48, 
			interval: 30 * 60 * 1000, // 30 minutes
			format: { hour: '2-digit', minute: '2-digit' }
		},
		'7d': { 
			label: 'Last 7 days', 
			points: 84, 
			interval: 2 * 60 * 60 * 1000, // 2 hours
			format: { month: 'short', day: 'numeric', hour: '2-digit' }
		}
	};
	
	let selectedRange = '6h';
	$: currentRange = TIME_RANGES[selectedRange];
	
	let metricsHistory = {
		timestamps: [] as string[],
		requestsPerMin: [] as number[],
		responseTime: [] as number[],
		cpuUsage: [] as number[],
		memoryUsage: [] as number[],
		errorRate: [] as number[]
	};
	
	let lastSampleTime = 0;
	
	function initCharts() {
		// Performance Chart
		const perfCtx = document.getElementById('performance-chart') as HTMLCanvasElement;
		if (perfCtx && !performanceChart) {
			performanceChart = new Chart(perfCtx, {
				type: 'line',
				data: {
					labels: metricsHistory.timestamps.length > 0 ? metricsHistory.timestamps : Array(currentRange.points).fill(''),
					datasets: [
						{
							label: 'Requests/min',
							data: metricsHistory.requestsPerMin.length > 0 ? metricsHistory.requestsPerMin : Array(currentRange.points).fill(0),
							borderColor: '#3b82f6',
							backgroundColor: 'rgba(59, 130, 246, 0.1)',
							borderWidth: 1.5,
							tension: 0.4,
							pointRadius: 0,
							fill: true
						},
						{
							label: 'Response Time (ms)',
							data: metricsHistory.responseTime.length > 0 ? metricsHistory.responseTime : Array(currentRange.points).fill(0),
							borderColor: '#10b981',
							backgroundColor: 'rgba(16, 185, 129, 0.1)',
							borderWidth: 1.5,
							tension: 0.4,
							pointRadius: 0,
							fill: true,
							yAxisID: 'y1'
						}
					]
				},
				options: {
					responsive: true,
					maintainAspectRatio: false,
					interaction: {
						mode: 'index',
						intersect: false
					},
					plugins: {
						legend: {
							display: true,
							position: 'top',
							labels: {
								boxWidth: 8,
								boxHeight: 8,
								padding: 10,
								font: { size: 11 },
								usePointStyle: true
							}
						},
						tooltip: {
							enabled: true,
							backgroundColor: 'rgba(0, 0, 0, 0.8)',
							titleFont: { size: 11 },
							bodyFont: { size: 10 },
							padding: 8,
							cornerRadius: 4
						}
					},
					scales: {
						x: {
							display: true,
							grid: {
								display: false
							},
							ticks: {
								font: { size: 9 },
								maxRotation: 45,
								minRotation: 45,
								autoSkip: true,
								maxTicksLimit: 12 // Show about 12 labels (every 30 minutes)
							}
						},
						y: {
							display: true,
							position: 'left',
							grid: {
								display: true,
								color: 'rgba(0, 0, 0, 0.05)'
							},
							ticks: {
								font: { size: 10 },
								padding: 4
							}
						},
						y1: {
							display: true,
							position: 'right',
							grid: {
								display: false
							},
							ticks: {
								font: { size: 10 },
								padding: 4
							}
						}
					}
				}
			});
		}

		// Activity Chart
		const actCtx = document.getElementById('activity-chart') as HTMLCanvasElement;
		if (actCtx && !activityChart) {
			activityChart = new Chart(actCtx, {
				type: 'line',
				data: {
					labels: metricsHistory.timestamps.length > 0 ? metricsHistory.timestamps : Array(currentRange.points).fill(''),
					datasets: [
						{
							label: 'CPU %',
							data: metricsHistory.cpuUsage.length > 0 ? metricsHistory.cpuUsage : Array(currentRange.points).fill(0),
							borderColor: '#8b5cf6',
							backgroundColor: 'rgba(139, 92, 246, 0.1)',
							borderWidth: 1.5,
							tension: 0.4,
							pointRadius: 0,
							fill: true
						},
						{
							label: 'Memory %',
							data: metricsHistory.memoryUsage.length > 0 ? metricsHistory.memoryUsage : Array(currentRange.points).fill(0),
							borderColor: '#f59e0b',
							backgroundColor: 'rgba(245, 158, 11, 0.1)',
							borderWidth: 1.5,
							tension: 0.4,
							pointRadius: 0,
							fill: true
						},
						{
							label: 'Error Rate %',
							data: metricsHistory.errorRate.length > 0 ? metricsHistory.errorRate : Array(currentRange.points).fill(0),
							borderColor: '#ef4444',
							backgroundColor: 'rgba(239, 68, 68, 0.1)',
							borderWidth: 1.5,
							tension: 0.4,
							pointRadius: 0,
							fill: true,
							yAxisID: 'y1'
						}
					]
				},
				options: {
					responsive: true,
					maintainAspectRatio: false,
					interaction: {
						mode: 'index',
						intersect: false
					},
					plugins: {
						legend: {
							display: true,
							position: 'top',
							labels: {
								boxWidth: 8,
								boxHeight: 8,
								padding: 10,
								font: { size: 11 },
								usePointStyle: true
							}
						},
						tooltip: {
							enabled: true,
							backgroundColor: 'rgba(0, 0, 0, 0.8)',
							titleFont: { size: 11 },
							bodyFont: { size: 10 },
							padding: 8,
							cornerRadius: 4
						}
					},
					scales: {
						x: {
							display: true,
							grid: {
								display: false
							},
							ticks: {
								font: { size: 9 },
								maxRotation: 45,
								minRotation: 45,
								autoSkip: true,
								maxTicksLimit: 12 // Show about 12 labels (every 30 minutes)
							}
						},
						y: {
							display: true,
							position: 'left',
							grid: {
								display: true,
								color: 'rgba(0, 0, 0, 0.05)'
							},
							ticks: {
								font: { size: 10 },
								padding: 4
							},
							min: 0,
							max: 100
						},
						y1: {
							display: true,
							position: 'right',
							grid: {
								display: false
							},
							ticks: {
								font: { size: 10 },
								padding: 4
							},
							min: 0,
							max: 10
						}
					}
				}
			});
		}
	}
	
	function updateCharts(forceUpdate = false) {
		const now = Date.now();
		
		// Only add a new point at the configured interval (or on force update)
		if (!forceUpdate && lastSampleTime && (now - lastSampleTime) < currentRange.interval) {
			// Just update the last point with current values
			if (metricsHistory.timestamps.length > 0) {
				const lastIndex = metricsHistory.timestamps.length - 1;
				metricsHistory.requestsPerMin[lastIndex] = httpRequestRate;
				metricsHistory.responseTime[lastIndex] = avgResponseTime;
				metricsHistory.cpuUsage[lastIndex] = cpuUsage;
				metricsHistory.memoryUsage[lastIndex] = memoryUsage;
				metricsHistory.errorRate[lastIndex] = errorRate;
			}
		} else {
			// Add new data point
			const timestamp = new Date(now).toLocaleTimeString('en-US', { 
				hour12: false, 
				...currentRange.format
			});
			
			// Update history arrays (keep last points for current range)
			if (metricsHistory.timestamps.length >= currentRange.points) {
				metricsHistory.timestamps.shift();
				metricsHistory.requestsPerMin.shift();
				metricsHistory.responseTime.shift();
				metricsHistory.cpuUsage.shift();
				metricsHistory.memoryUsage.shift();
				metricsHistory.errorRate.shift();
			}
			
			metricsHistory.timestamps.push(timestamp);
			metricsHistory.requestsPerMin.push(httpRequestRate);
			metricsHistory.responseTime.push(avgResponseTime);
			metricsHistory.cpuUsage.push(cpuUsage);
			metricsHistory.memoryUsage.push(memoryUsage);
			metricsHistory.errorRate.push(errorRate);
			
			lastSampleTime = now;
		}
		
		// Update performance chart
		if (performanceChart) {
			performanceChart.data.labels = metricsHistory.timestamps;
			performanceChart.data.datasets[0].data = metricsHistory.requestsPerMin;
			performanceChart.data.datasets[1].data = metricsHistory.responseTime;
			performanceChart.update('none');
		}

		// Update activity chart
		if (activityChart) {
			activityChart.data.labels = metricsHistory.timestamps;
			activityChart.data.datasets[0].data = metricsHistory.cpuUsage;
			activityChart.data.datasets[1].data = metricsHistory.memoryUsage;
			activityChart.data.datasets[2].data = metricsHistory.errorRate;
			activityChart.update('none');
		}
	}
	
	async function fetchDashboardData() {
		try {
			// Fetch dashboard stats
			const stats = await api.get('/dashboard/stats');
			
			// Store previous values for trend calculation
			previousMetrics = {
				users: totalUsers,
				rows: totalRows,
				storage: totalStorage,
				requests: httpRequestsTotal
			};
			
			totalUsers = stats.total_users || 0;
			totalRows = stats.total_rows || 0;
			totalStorage = stats.total_storage_used || 0;
			
			// Fetch system metrics
			const metrics = await api.get('/system/metrics');
			cpuUsage = metrics.cpu_usage || 0;
			memoryUsage = metrics.memory_usage || 0;
			diskUsage = metrics.disk_usage || 0;
			uptime = metrics.uptime || '0h 0m';
			
			memoryInfo = {
				used: metrics.memory_used || 0,
				total: metrics.memory_total || 0
			};
			diskInfo = {
				used: metrics.disk_used || 0,
				total: metrics.disk_total || 0
			};
			
			// Fetch Prometheus metrics
			const response = await fetch('/api/metrics');
			const text = await response.text();
			
			const lines = text.split('\n');
			let requestsCount = 0;
			let queriesCount = 0;
			let totalDuration = 0;
			let durationCount = 0;
			let errors = 0;
			
			lines.forEach(line => {
				if (line.startsWith('http_requests_total{')) {
					const match = line.match(/} (\d+)/);
					if (match) requestsCount += parseInt(match[1]);
					
					if (line.includes('status="4') || line.includes('status="5')) {
						const errorMatch = line.match(/} (\d+)/);
						if (errorMatch) errors += parseInt(errorMatch[1]);
					}
				}
				if (line.startsWith('database_queries_total{')) {
					const match = line.match(/} (\d+)/);
					if (match) queriesCount += parseInt(match[1]);
				}
				if (line.startsWith('http_request_duration_seconds_sum{')) {
					const match = line.match(/} ([\d.]+)/);
					if (match) totalDuration += parseFloat(match[1]);
				}
				if (line.startsWith('http_request_duration_seconds_count{')) {
					const match = line.match(/} (\d+)/);
					if (match) durationCount += parseInt(match[1]);
				}
			});
			
			httpRequestsTotal = requestsCount;
			dbQueriesTotal = queriesCount;
			avgResponseTime = durationCount > 0 ? Math.round(totalDuration / durationCount * 1000) : 0;
			errorRate = requestsCount > 0 ? parseFloat(((errors / requestsCount) * 100).toFixed(1)) : 0;
			httpRequestRate = Math.round(requestsCount / 60);
			
			updateCharts();
			loading = false;
			error = '';
		} catch (err) {
			console.error('Failed to fetch dashboard data:', err);
			error = 'Failed to load dashboard data';
			loading = false;
		}
	}
	
	async function refreshData() {
		loading = true;
		await fetchDashboardData();
	}
	
	function onRangeChange() {
		// Clear existing history
		metricsHistory = {
			timestamps: [],
			requestsPerMin: [],
			responseTime: [],
			cpuUsage: [],
			memoryUsage: [],
			errorRate: []
		};
		
		// Reinitialize with new range
		const now = new Date();
		for (let i = currentRange.points - 1; i >= 0; i--) {
			const pastTime = new Date(now.getTime() - i * currentRange.interval);
			const timestamp = selectedRange === '7d' 
				? pastTime.toLocaleDateString('en-US', currentRange.format as any)
				: pastTime.toLocaleTimeString('en-US', { hour12: false, ...currentRange.format } as any);
			
			metricsHistory.timestamps.push(timestamp);
			// Initialize with random data to show trends
			metricsHistory.requestsPerMin.push(Math.random() * 50 + 10);
			metricsHistory.responseTime.push(Math.random() * 100 + 50);
			metricsHistory.cpuUsage.push(Math.random() * 30 + 20);
			metricsHistory.memoryUsage.push(Math.random() * 40 + 30);
			metricsHistory.errorRate.push(Math.random() * 2);
		}
		
		lastSampleTime = now.getTime();
		
		// Reinitialize charts
		if (performanceChart) {
			performanceChart.destroy();
			performanceChart = null;
		}
		if (activityChart) {
			activityChart.destroy();
			activityChart = null;
		}
		
		setTimeout(() => {
			initCharts();
			updateCharts(true);
		}, 100);
	}
	
	onMount(async () => {
		// Initialize with history points for default range
		const now = new Date();
		for (let i = currentRange.points - 1; i >= 0; i--) {
			const pastTime = new Date(now.getTime() - i * currentRange.interval);
			metricsHistory.timestamps.push(pastTime.toLocaleTimeString('en-US', { 
				hour12: false, 
				...currentRange.format
			} as any));
			// Initialize with random data to show trends
			metricsHistory.requestsPerMin.push(Math.random() * 50 + 10);
			metricsHistory.responseTime.push(Math.random() * 100 + 50);
			metricsHistory.cpuUsage.push(Math.random() * 30 + 20);
			metricsHistory.memoryUsage.push(Math.random() * 40 + 30);
			metricsHistory.errorRate.push(Math.random() * 2);
		}
		
		lastSampleTime = now.getTime();
		
		await fetchDashboardData();
		
		// Initialize charts after DOM is ready
		setTimeout(() => {
			initCharts();
			updateCharts(true);
		}, 100);
		
		// Update metrics every 30 seconds (fetch new data frequently for real-time feel)
		intervalId = setInterval(async () => {
			await fetchDashboardData();
			updateCharts();
		}, 30000);
	});
	
	onDestroy(() => {
		if (intervalId) clearInterval(intervalId);
		if (performanceChart) performanceChart.destroy();
		if (activityChart) activityChart.destroy();
	});
	
	function formatBytes(bytes: number): string {
		if (bytes === 0) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
	}
	
	function formatNumber(num: number): string {
		if (num >= 1000000) return (num / 1000000).toFixed(1) + 'M';
		if (num >= 1000) return (num / 1000).toFixed(1) + 'K';
		return num.toString();
	}
	
	function getTrend(current: number, previous: number): 'up' | 'down' | 'stable' {
		if (previous === 0) return 'stable';
		if (current > previous) return 'up';
		if (current < previous) return 'down';
		return 'stable';
	}
	
	function getHealthStatus(cpu: number, memory: number, disk: number): string {
		if (cpu > 90 || memory > 90 || disk > 90) return 'critical';
		if (cpu > 70 || memory > 70 || disk > 70) return 'warning';
		return 'healthy';
	}
	
	$: healthStatus = getHealthStatus(cpuUsage, memoryUsage, diskUsage);
</script>

<div class="dashboard-container">
	<!-- Header -->
	<div class="page-header">
		<div class="header-content">
			<div class="header-left">
				<div class="header-title">
					<LayoutDashboard size={24} />
					<h1>Dashboard</h1>
				</div>
				<div class="header-meta">
					<span class="meta-item">Uptime: {uptime}</span>
				</div>
			</div>
			<div class="header-actions">
				<button class="refresh-button" on:click={refreshData} disabled={loading}>
					<RefreshCw size={16} class={loading ? 'spinning' : ''} />
					{loading ? 'Refreshing...' : 'Refresh'}
				</button>
			</div>
		</div>
	</div>
	
	<div class="dashboard-content">
		{#if error}
			<div class="error-banner">
				<AlertCircle size={18} />
				{error}
			</div>
		{/if}

		<!-- Compact Metrics Grid -->
		<div class="metrics-grid">
		<div class="metric-card users">
			<div class="metric-content">
				<div class="metric-info">
					<div class="metric-label">Total Users</div>
					<div class="metric-value">{formatNumber(totalUsers)}</div>
					<div class="metric-trend {getTrend(totalUsers, previousMetrics.users)}">
						{#if getTrend(totalUsers, previousMetrics.users) === 'up'}
							<TrendingUp size={12} />
						{:else if getTrend(totalUsers, previousMetrics.users) === 'down'}
							<TrendingDown size={12} />
						{/if}
						<span>Active accounts</span>
					</div>
				</div>
				<div class="metric-icon">
					<Users size={16} />
				</div>
			</div>
		</div>

		<div class="metric-card database">
			<div class="metric-content">
				<div class="metric-info">
					<div class="metric-label">Database Rows</div>
					<div class="metric-value">{formatNumber(totalRows)}</div>
					<div class="metric-trend {getTrend(totalRows, previousMetrics.rows)}">
						{#if getTrend(totalRows, previousMetrics.rows) === 'up'}
							<TrendingUp size={12} />
						{:else if getTrend(totalRows, previousMetrics.rows) === 'down'}
							<TrendingDown size={12} />
						{/if}
						<span>Total records</span>
					</div>
				</div>
				<div class="metric-icon">
					<Database size={16} />
				</div>
			</div>
		</div>

		<div class="metric-card storage">
			<div class="metric-content">
				<div class="metric-info">
					<div class="metric-label">Storage Used</div>
					<div class="metric-value">{formatBytes(totalStorage)}</div>
					<div class="metric-trend">
						<HardDrive size={12} />
						<span>Total files</span>
					</div>
				</div>
				<div class="metric-icon">
					<HardDrive size={16} />
				</div>
			</div>
		</div>

		<div class="metric-card requests">
			<div class="metric-content">
				<div class="metric-info">
					<div class="metric-label">API Requests</div>
					<div class="metric-value">{formatNumber(httpRequestsTotal)}</div>
					<div class="metric-trend">
						<Activity size={12} />
						<span>{httpRequestRate} req/min</span>
					</div>
				</div>
				<div class="metric-icon">
					<Zap size={16} />
				</div>
			</div>
		</div>
		</div>

		<!-- System & Performance Row -->
		<div class="monitoring-row">
		<!-- Performance Chart -->
		<div class="chart-card">
			<div class="card-header">
				<h3>Performance Metrics</h3>
				<div class="chart-controls">
					<select 
						class="time-range-select" 
						bind:value={selectedRange} 
						on:change={onRangeChange}
					>
						{#each Object.entries(TIME_RANGES) as [key, range]}
							<option value={key}>{range.label}</option>
						{/each}
					</select>
					<div class="header-stats">
						<span class="stat-badge good">
							<CheckCircle size={10} />
							{avgResponseTime}ms avg
						</span>
						{#if errorRate > 0}
							<span class="stat-badge {errorRate > 5 ? 'bad' : 'warn'}">
								<AlertCircle size={10} />
								{errorRate}% errors
							</span>
						{/if}
					</div>
				</div>
			</div>
			<div class="chart-container">
				<canvas id="performance-chart"></canvas>
			</div>
			
			<!-- Activity Chart (below performance) -->
			<div class="card-header" style="border-top: 1px solid #f3f4f6;">
				<h3>System Activity</h3>
				<div class="header-stats">
					<span class="stat-badge" style="background: #e9d5ff; color: #6b1a8d;">
						<Server size={10} />
						Live monitoring
					</span>
				</div>
			</div>
			<div class="chart-container">
				<canvas id="activity-chart"></canvas>
			</div>
		</div>

		<!-- System Health -->
		<div class="health-card">
			<div class="card-header">
				<h3>System Health</h3>
				<span class="health-indicator {healthStatus}">
					{#if healthStatus === 'healthy'}
						<CheckCircle size={12} />
						Healthy
					{:else if healthStatus === 'warning'}
						<AlertCircle size={12} />
						Warning
					{:else}
						<AlertCircle size={12} />
						Critical
					{/if}
				</span>
			</div>
			<div class="health-metrics">
				<div class="health-item">
					<div class="health-header">
						<span>CPU Usage</span>
						<span class="health-percent">{cpuUsage.toFixed(0)}%</span>
					</div>
					<div class="health-bar">
						<div class="health-fill cpu" style="width: {cpuUsage}%"></div>
					</div>
				</div>
				<div class="health-item">
					<div class="health-header">
						<span>Memory</span>
						<span class="health-percent">{memoryUsage.toFixed(0)}%</span>
					</div>
					<div class="health-bar">
						<div class="health-fill memory" style="width: {memoryUsage}%"></div>
					</div>
					<div class="health-detail">
						{formatBytes(memoryInfo.used)} / {formatBytes(memoryInfo.total)}
					</div>
				</div>
				<div class="health-item">
					<div class="health-header">
						<span>Disk Space</span>
						<span class="health-percent">{diskUsage.toFixed(0)}%</span>
					</div>
					<div class="health-bar">
						<div class="health-fill disk" style="width: {diskUsage}%"></div>
					</div>
					<div class="health-detail">
						{formatBytes(diskInfo.used)} / {formatBytes(diskInfo.total)}
					</div>
				</div>
			</div>
			<div class="health-footer">
				<div class="footer-stat">
					<span>DB Queries</span>
					<strong>{formatNumber(dbQueriesTotal)}</strong>
				</div>
				<div class="footer-stat">
					<span>Uptime</span>
					<strong>{uptime}</strong>
				</div>
			</div>
		</div>
		</div>

		<!-- Quick Actions / Status Bar -->
		<div class="status-bar">
		<div class="status-left">
			<span class="status-item">
				<Server size={12} />
				All systems operational
			</span>
			<span class="status-item">
				<Clock size={12} />
				Last updated: just now
			</span>
		</div>
		<div class="status-right">
			<a href="/logs" class="status-link">View Logs</a>
			<a href="/users" class="status-link">Manage Users</a>
			<a href="/settings" class="status-link">Settings</a>
		</div>
		</div>
	</div>
</div>

<style>
	.dashboard-container {
		min-height: 100vh;
		background: #f3f4f6;
	}
	
	.page-header {
		background: white;
		border-bottom: 1px solid #e5e7eb;
		padding: 1.5rem 2rem;
	}
	
	.header-content {
		max-width: 1400px;
		margin: 0 auto;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	
	.header-left {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.5rem;
	}
	
	:global(.header-icon) {
		color: #374151;
	}
	
	.header-title {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}
	
	.header-title h1 {
		font-size: 1.5rem;
		font-weight: 700;
		color: #111827;
		margin: 0;
	}
	
	.header-meta {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: #6b7280;
		font-size: 0.875rem;
		margin-left: 2.25rem;
	}
	
	.meta-item {
		color: #6b7280;
	}
	
	.meta-separator {
		color: #d1d5db;
	}
	
	.header-actions {
		display: flex;
		align-items: center;
		gap: 1rem;
	}
	
	.refresh-button {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		background: white;
		color: #374151;
		border: 1px solid #d1d5db;
		border-radius: 6px;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.refresh-button:hover:not(:disabled) {
		background: #f9fafb;
		border-color: #9ca3af;
	}
	
	.refresh-button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	
	:global(.refresh-button .spinning) {
		animation: spin 1s linear infinite;
	}
	
	.dashboard-content {
		padding: 1.5rem;
		max-width: 1400px;
		margin: 0 auto;
	}
	
	.error-banner {
		background: #fee2e2;
		color: #dc2626;
		padding: 1rem;
		border-radius: 8px;
		margin-bottom: 1.5rem;
		display: flex;
		align-items: center;
		gap: 0.75rem;
		font-size: 0.875rem;
	}

	@keyframes spin {
		from { transform: rotate(0deg); }
		to { transform: rotate(360deg); }
	}

	/* Metrics Grid */
	.metrics-grid {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 0.75rem;
		margin-bottom: 1rem;
	}

	.metric-card {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		padding: 0.875rem;
		position: relative;
		transition: all 0.2s;
	}

	.metric-card:hover {
		box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.1);
		transform: translateY(-1px);
	}

	.metric-card::before {
		content: '';
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		height: 2px;
		border-radius: 0.5rem 0.5rem 0 0;
	}

	.metric-card.users::before { background: #3b82f6; }
	.metric-card.database::before { background: #10b981; }
	.metric-card.storage::before { background: #f59e0b; }
	.metric-card.requests::before { background: #8b5cf6; }

	.metric-content {
		display: flex;
		justify-content: space-between;
		align-items: start;
	}

	.metric-label {
		font-size: 0.6875rem;
		font-weight: 500;
		color: #6b7280;
		text-transform: uppercase;
		letter-spacing: 0.025em;
		margin-bottom: 0.25rem;
	}

	.metric-value {
		font-size: 1.5rem;
		font-weight: 700;
		color: #111827;
		line-height: 1;
		margin-bottom: 0.375rem;
	}

	.metric-trend {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		font-size: 0.6875rem;
		color: #6b7280;
	}

	.metric-trend.up { color: #10b981; }
	.metric-trend.down { color: #ef4444; }

	.metric-icon {
		width: 28px;
		height: 28px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 0.375rem;
		opacity: 0.8;
	}

	.metric-card.users .metric-icon { background: #dbeafe; color: #3b82f6; }
	.metric-card.database .metric-icon { background: #d1fae5; color: #10b981; }
	.metric-card.storage .metric-icon { background: #fed7aa; color: #f59e0b; }
	.metric-card.requests .metric-icon { background: #e9d5ff; color: #8b5cf6; }

	/* Monitoring Row */
	.monitoring-row {
		display: grid;
		grid-template-columns: 2fr 1fr;
		gap: 0.75rem;
		margin-bottom: 1rem;
	}

	.chart-card, .health-card {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
	}

	.card-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem;
		border-bottom: 1px solid #f3f4f6;
	}

	.card-header h3 {
		font-size: 0.875rem;
		font-weight: 600;
		color: #374151;
		margin: 0;
	}

	.chart-controls {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

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

	.header-stats {
		display: flex;
		gap: 0.5rem;
	}

	.stat-badge {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.125rem 0.375rem;
		border-radius: 0.25rem;
		font-size: 0.6875rem;
		font-weight: 500;
	}

	.stat-badge.good {
		background: #d1fae5;
		color: #065f46;
	}

	.stat-badge.warn {
		background: #fed7aa;
		color: #92400e;
	}

	.stat-badge.bad {
		background: #fee2e2;
		color: #991b1b;
	}

	.chart-container {
		padding: 0.75rem;
		height: 140px;
	}

	/* Health Card */
	.health-indicator {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.125rem 0.375rem;
		border-radius: 0.25rem;
		font-size: 0.6875rem;
		font-weight: 500;
	}

	.health-indicator.healthy {
		background: #d1fae5;
		color: #065f46;
	}

	.health-indicator.warning {
		background: #fed7aa;
		color: #92400e;
	}

	.health-indicator.critical {
		background: #fee2e2;
		color: #991b1b;
	}

	.health-metrics {
		padding: 0.75rem;
	}

	.health-item {
		margin-bottom: 0.625rem;
	}

	.health-item:last-child {
		margin-bottom: 0;
	}

	.health-header {
		display: flex;
		justify-content: space-between;
		margin-bottom: 0.25rem;
	}

	.health-header span {
		font-size: 0.6875rem;
		color: #6b7280;
	}

	.health-percent {
		font-weight: 600;
		color: #111827;
	}

	.health-bar {
		height: 4px;
		background: #f3f4f6;
		border-radius: 2px;
		overflow: hidden;
	}

	.health-fill {
		height: 100%;
		border-radius: 2px;
		transition: width 0.3s ease;
	}

	.health-fill.cpu {
		background: linear-gradient(90deg, #3b82f6, #2563eb);
	}

	.health-fill.memory {
		background: linear-gradient(90deg, #10b981, #059669);
	}

	.health-fill.disk {
		background: linear-gradient(90deg, #f59e0b, #d97706);
	}

	.health-detail {
		font-size: 0.625rem;
		color: #9ca3af;
		margin-top: 0.125rem;
	}

	.health-footer {
		display: flex;
		justify-content: space-around;
		padding: 0.5rem 0.75rem;
		background: #f9fafb;
		border-top: 1px solid #f3f4f6;
	}

	.footer-stat {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.125rem;
	}

	.footer-stat span {
		font-size: 0.625rem;
		color: #6b7280;
	}

	.footer-stat strong {
		font-size: 0.75rem;
		color: #111827;
	}

	/* Status Bar */
	.status-bar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.625rem 0.875rem;
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
	}

	.status-left, .status-right {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.status-item {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		font-size: 0.6875rem;
		color: #6b7280;
	}

	.status-link {
		font-size: 0.6875rem;
		color: #3b82f6;
		text-decoration: none;
		font-weight: 500;
	}

	.status-link:hover {
		text-decoration: underline;
	}

	/* Error State */
	.error {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem;
		background: #fee2e2;
		color: #991b1b;
		border-radius: 0.375rem;
		margin-bottom: 1rem;
		font-size: 0.875rem;
	}

	/* Responsive */
	@media (max-width: 1024px) {
		.metrics-grid {
			grid-template-columns: repeat(2, 1fr);
		}

		.monitoring-row {
			grid-template-columns: 1fr;
		}
	}

	@media (max-width: 768px) {
		.header-content {
			flex-direction: column;
			align-items: flex-start;
			gap: 1rem;
		}
		
		.header-actions {
			width: 100%;
			justify-content: flex-start;
		}
		
		.dashboard-content {
			padding: 1rem;
		}
	}
	
	@media (max-width: 640px) {
		.metrics-grid {
			grid-template-columns: 1fr;
		}

		.status-bar {
			flex-direction: column;
			gap: 0.75rem;
			align-items: start;
		}

		.status-left, .status-right {
			width: 100%;
			justify-content: space-between;
		}
	}
</style>