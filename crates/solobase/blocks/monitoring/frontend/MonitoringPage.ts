import { html } from '@solobase/ui';
import { useState, useEffect, useRef } from 'preact/hooks';
import { Chart, registerables } from 'chart.js';
import {
	Activity, AlertCircle, CheckCircle, RefreshCw,
	Clock, Server, Zap
} from 'lucide-preact';
import { api, PageHeader } from '@solobase/ui';

Chart.register(...registerables);

interface BlockStats {
	count: number;
	avgMs: number;
	errors: number;
}

interface FlowStats {
	count: number;
	avgMs: number;
	errors: number;
}

interface LiveStats {
	totalMessages: number;
	totalErrors: number;
	perBlock: Record<string, BlockStats>;
	perFlow: Record<string, FlowStats>;
	perKind: Record<string, number>;
}

interface HistorySnapshot {
	id: string;
	periodStart: string;
	periodEnd: string;
	totalMessages: number;
	totalErrors: number;
	perBlockJson: string;
	perFlowJson: string;
	perKindJson: string;
}

export function MonitoringPage() {
	const [loading, setLoading] = useState(true);
	const [error, setError] = useState('');
	const [stats, setStats] = useState<LiveStats>({ totalMessages: 0, totalErrors: 0, perBlock: {}, perFlow: {}, perKind: {} });
	const [uptime, setUptime] = useState('');
	const [selectedRange, setSelectedRange] = useState('24h');

	// Chart refs
	const timeSeriesCanvasRef = useRef<HTMLCanvasElement>(null);
	const kindCanvasRef = useRef<HTMLCanvasElement>(null);
	const timeSeriesChartRef = useRef<Chart | null>(null);
	const kindChartRef = useRef<Chart | null>(null);

	async function fetchData() {
		try {
			const [liveStats, debugTime] = await Promise.all([
				api.get<LiveStats>('/admin/monitoring/live'),
				api.get<{ startTime: string; now: string }>('/debug/time'),
			]);

			setStats(liveStats);
			setUptime(formatUptime(debugTime.startTime, debugTime.now));
			setError('');

			await loadHistory();
			updateKindChart(liveStats.perKind);
		} catch (err) {
			console.error('Failed to fetch monitoring data:', err);
			setError('Failed to load monitoring data');
		} finally {
			setLoading(false);
		}
	}

	async function loadHistory() {
		try {
			const snapshots = await api.get<HistorySnapshot[]>(`/admin/monitoring/history?range=${selectedRange}`);
			updateTimeSeriesChart(snapshots || []);
		} catch (err) {
			console.error('Failed to load history:', err);
		}
	}

	function formatUptime(startTime: string, now: string): string {
		const start = new Date(startTime).getTime();
		const current = new Date(now).getTime();
		if (isNaN(start) || isNaN(current)) return '0m';
		const diff = current - start;
		const days = Math.floor(diff / (1000 * 60 * 60 * 24));
		const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
		const mins = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));
		if (days > 0) return `${days}d ${hours}h ${mins}m`;
		if (hours > 0) return `${hours}h ${mins}m`;
		return `${mins}m`;
	}

	function updateTimeSeriesChart(snapshots: HistorySnapshot[]) {
		const canvas = timeSeriesCanvasRef.current;
		if (!canvas) return;

		const labels = snapshots.map(s => {
			const d = new Date(s.periodEnd);
			return d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: false });
		});
		const messages = snapshots.map(s => s.totalMessages);
		const errors = snapshots.map(s => s.totalErrors);

		if (timeSeriesChartRef.current) {
			timeSeriesChartRef.current.data.labels = labels;
			timeSeriesChartRef.current.data.datasets[0].data = messages;
			timeSeriesChartRef.current.data.datasets[1].data = errors;
			timeSeriesChartRef.current.update('none');
			return;
		}

		timeSeriesChartRef.current = new Chart(canvas, {
			type: 'line',
			data: {
				labels,
				datasets: [
					{ label: 'Messages', data: messages, borderColor: '#3b82f6', backgroundColor: 'rgba(59,130,246,0.1)', borderWidth: 1.5, tension: 0.4, pointRadius: 0, fill: true },
					{ label: 'Errors', data: errors, borderColor: '#ef4444', backgroundColor: 'rgba(239,68,68,0.1)', borderWidth: 1.5, tension: 0.4, pointRadius: 0, fill: true, yAxisID: 'y1' },
				],
			},
			options: {
				responsive: true, maintainAspectRatio: false,
				interaction: { mode: 'index', intersect: false },
				plugins: {
					legend: { display: true, position: 'top', labels: { boxWidth: 8, boxHeight: 8, padding: 10, font: { size: 11 }, usePointStyle: true } },
					tooltip: { enabled: true, backgroundColor: 'rgba(0,0,0,0.8)', titleFont: { size: 11 }, bodyFont: { size: 10 }, padding: 8, cornerRadius: 4 },
				},
				scales: {
					x: { display: true, grid: { display: false }, ticks: { font: { size: 9 }, maxRotation: 45, minRotation: 45, autoSkip: true, maxTicksLimit: 12 } },
					y: { display: true, position: 'left', grid: { display: true, color: 'rgba(0,0,0,0.05)' }, ticks: { font: { size: 10 }, padding: 4 }, beginAtZero: true },
					y1: { display: true, position: 'right', grid: { display: false }, ticks: { font: { size: 10 }, padding: 4 }, beginAtZero: true },
				},
			},
		});
	}

	function updateKindChart(perKind: Record<string, number>) {
		const canvas = kindCanvasRef.current;
		if (!canvas) return;

		const entries = Object.entries(perKind).sort((a, b) => b[1] - a[1]).slice(0, 10);
		const labels = entries.map(([k]) => k);
		const data = entries.map(([, v]) => v);
		const colors = ['#3b82f6', '#10b981', '#f59e0b', '#8b5cf6', '#ef4444', '#06b6d4', '#ec4899', '#84cc16', '#f97316', '#6366f1'];

		if (kindChartRef.current) {
			kindChartRef.current.data.labels = labels;
			kindChartRef.current.data.datasets[0].data = data;
			kindChartRef.current.update('none');
			return;
		}

		kindChartRef.current = new Chart(canvas, {
			type: 'doughnut',
			data: {
				labels,
				datasets: [{ data, backgroundColor: colors.slice(0, data.length), borderWidth: 1, borderColor: '#fff' }],
			},
			options: {
				responsive: true, maintainAspectRatio: false,
				plugins: {
					legend: { display: true, position: 'right', labels: { boxWidth: 10, padding: 8, font: { size: 10 } } },
				},
			},
		});
	}

	useEffect(() => {
		fetchData();
		const id = setInterval(fetchData, 30000);
		return () => {
			clearInterval(id);
			if (timeSeriesChartRef.current) timeSeriesChartRef.current.destroy();
			if (kindChartRef.current) kindChartRef.current.destroy();
		};
	}, []);

	useEffect(() => {
		loadHistory();
	}, [selectedRange]);

	const errorRate = stats.totalMessages > 0 ? ((stats.totalErrors / stats.totalMessages) * 100).toFixed(1) : '0.0';
	const blockEntries = Object.entries(stats.perBlock).sort((a, b) => b[1].count - a[1].count);
	const flowEntries = Object.entries(stats.perFlow).sort((a, b) => b[1].count - a[1].count);

	return html`
		<>
			<div class="monitoring-container">
				<${PageHeader} title="Dashboard">
					<div style=${{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
						${uptime ? html`<span style=${{ color: '#6b7280', fontSize: '0.875rem' }}>Uptime: ${uptime}</span>` : null}
						<button class="refresh-button" onClick=${() => { setLoading(true); fetchData(); }} disabled=${loading} type="button">
							<${RefreshCw} size=${16} class=${loading ? 'spinning' : ''} />
							${loading ? 'Refreshing...' : 'Refresh'}
						</button>
					</div>
				</${PageHeader}>

				<div class="monitoring-content">
					${error ? html`<div class="error-banner"><${AlertCircle} size=${18} /> ${error}</div>` : null}

					<div class="metrics-grid">
						${renderStatCard('messages', 'Messages Processed', stats.totalMessages.toLocaleString(), 'Total block executions', Zap)}
						${renderStatCard('errors', 'Errors', stats.totalErrors.toLocaleString(), `${errorRate}% error rate`, AlertCircle)}
						${renderStatCard('blocks', 'Active Blocks', blockEntries.length.toString(), 'Unique blocks executed', Server)}
						${renderStatCard('flows', 'Active Flows', flowEntries.length.toString(), 'Unique flows executed', Activity)}
					</div>

					<div class="charts-row">
						<div class="chart-card wide">
							<div class="card-header">
								<h3>Message Throughput</h3>
								<select class="time-range-select" value=${selectedRange} onChange=${(e: Event) => setSelectedRange((e.target as HTMLSelectElement).value)}>
									<option value="1h">Last Hour</option>
									<option value="6h">Last 6 Hours</option>
									<option value="24h">Last 24 Hours</option>
									<option value="7d">Last 7 Days</option>
								</select>
							</div>
							<div class="chart-container"><canvas ref=${timeSeriesCanvasRef}></canvas></div>
						</div>
						<div class="chart-card narrow">
							<div class="card-header"><h3>Message Kinds</h3></div>
							<div class="chart-container doughnut"><canvas ref=${kindCanvasRef}></canvas></div>
						</div>
					</div>

					<div class="tables-row">
						<div class="data-card">
							<div class="card-header"><h3>Per-Block Stats</h3></div>
							<div class="table-container">
								<table class="table">
									<thead><tr><th>Block</th><th>Count</th><th>Avg (ms)</th><th>Errors</th></tr></thead>
									<tbody>
										${blockEntries.length === 0
											? html`<tr><td colspan="4" class="empty-cell">No block data yet</td></tr>`
											: blockEntries.map(([name, s]) => html`
												<tr key=${name}>
													<td class="block-name">${name}</td>
													<td>${s.count.toLocaleString()}</td>
													<td>${s.avgMs.toFixed(1)}</td>
													<td>${s.errors > 0 ? html`<span class="error-badge">${s.errors}</span>` : '0'}</td>
												</tr>
											`)}
									</tbody>
								</table>
							</div>
						</div>

						<div class="data-card">
							<div class="card-header"><h3>Per-Flow Stats</h3></div>
							<div class="table-container">
								<table class="table">
									<thead><tr><th>Flow</th><th>Executions</th><th>Avg (ms)</th><th>Error Rate</th></tr></thead>
									<tbody>
										${flowEntries.length === 0
											? html`<tr><td colspan="4" class="empty-cell">No flow data yet</td></tr>`
											: flowEntries.map(([id, s]) => {
												const rate = s.count > 0 ? ((s.errors / s.count) * 100).toFixed(1) : '0.0';
												return html`
													<tr key=${id}>
														<td class="block-name">${id}</td>
														<td>${s.count.toLocaleString()}</td>
														<td>${s.avgMs.toFixed(1)}</td>
														<td>${parseFloat(rate) > 0 ? html`<span class="error-badge">${rate}%</span>` : '0%'}</td>
													</tr>
												`;
											})}
									</tbody>
								</table>
							</div>
						</div>
					</div>

					<div class="status-bar">
						<span class="status-item"><${CheckCircle} size=${12} /> All systems operational</span>
						<span class="status-item"><${Clock} size=${12} /> Auto-refresh: 30s</span>
					</div>
				</div>
			</div>
		<//>
	`;
}

function renderStatCard(type: string, label: string, value: string, description: string, Icon: any) {
	return html`
		<div class="stat-card ${type}">
			<div class="stat-content">
				<div class="stat-info">
					<div class="stat-label">${label}</div>
					<div class="stat-value">${value}</div>
					<div class="stat-desc">${description}</div>
				</div>
				<div class="stat-icon"><${Icon} size=${16} /></div>
			</div>
		</div>
	`;
}
