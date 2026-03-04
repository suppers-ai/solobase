import { html } from '@solobase/ui';
import { useState, useEffect, useRef } from 'preact/hooks';
import { Chart, registerables } from 'chart.js';
import {
	FileText,
	AlertCircle, Info, AlertTriangle, XCircle,
	Clock, RefreshCw
} from 'lucide-preact';
import { api, PageHeader, SearchInput, ExportButton } from '@solobase/ui';

Chart.register(...registerables);

interface LogEntry {
	id?: string;
	level: string;
	message: string;
	timestamp: string;
	source?: string;
	fields?: Record<string, unknown>;
	[key: string]: unknown;
}

interface MessageLog {
	id?: string;
	flowId: string;
	blockName: string;
	messageKind: string;
	action: string;
	durationMs: number;
	traceId?: string;
	error?: string;
	userId?: string;
	metaSnapshot?: string;
	createdAt: string;
	[key: string]: unknown;
}

interface LogsResponse {
	logs: LogEntry[];
	total: number;
}

interface MessageLogsResponse {
	logs: MessageLog[];
	total: number;
}

function getActionLevel(action: string): string {
	if (action === 'error') return 'error';
	if (action === 'drop') return 'warning';
	return 'info';
}

function getLevelIcon(level: string) {
	switch (level) {
		case 'error': return XCircle;
		case 'warning': return AlertTriangle;
		case 'info': return Info;
		default: return AlertCircle;
	}
}

function getLevelColor(level: string) {
	switch (level) {
		case 'error': return 'var(--danger-color)';
		case 'warning': return 'var(--warning-color)';
		case 'info': return 'var(--info-color)';
		case 'debug': return 'var(--secondary-color)';
		default: return 'var(--text-muted)';
	}
}

export function LogsPage() {
	const [searchQuery, setSearchQuery] = useState('');
	const [selectedLevel, setSelectedLevel] = useState('');
	const [selectedAction, setSelectedAction] = useState('');
	const [selectedTimeRange, setSelectedTimeRange] = useState('24h');
	const [autoRefresh, setAutoRefresh] = useState(false);
	const [loading, setLoading] = useState(false);
	const [currentPage, setCurrentPage] = useState(1);
	const [totalPages, setTotalPages] = useState(1);
	const [pageSize] = useState(100);

	const [logs, setLogs] = useState<LogEntry[]>([]);
	const [messageLogs, setMessageLogs] = useState<MessageLog[]>([]);
	const [activeTab, setActiveTab] = useState('messages');

	// Stats
	const [totalLogs, setTotalLogs] = useState(0);
	const [errorCount, setErrorCount] = useState(0);
	const [warningCount, setWarningCount] = useState(0);
	const [infoCount, setInfoCount] = useState(0);

	// Separate counts for each tab
	const [messageLogsCount, setMessageLogsCount] = useState(0);
	const [applicationLogsCount, setApplicationLogsCount] = useState(0);

	// Refs
	const chartCanvasRef = useRef<HTMLCanvasElement>(null);
	const activityChartRef = useRef<Chart | null>(null);
	const refreshIntervalRef = useRef<number | null>(null);

	function toggleAutoRefresh() {
		const next = !autoRefresh;
		setAutoRefresh(next);

		if (refreshIntervalRef.current) {
			clearInterval(refreshIntervalRef.current);
			refreshIntervalRef.current = null;
		}

		if (next) {
			refreshIntervalRef.current = setInterval(() => {
				loadLogs();
			}, 5000) as unknown as number;
		}
	}

	async function loadLogs() {
		setLoading(true);
		try {
			if (activeTab === 'messages') {
				const params = new URLSearchParams({
					page: currentPage.toString(),
					size: pageSize.toString(),
					range: selectedTimeRange
				});

				if (searchQuery) {
					params.append('kind', searchQuery);
				}

				if (selectedAction) {
					params.append('action', selectedAction);
				}

				const response = await api.get<MessageLogsResponse>(`/admin/logs/messages?${params}`);
				const mLogs = response.logs || [];
				const total = response.total || 0;
				setMessageLogs(mLogs);
				setTotalLogs(total);
				setMessageLogsCount(total);
				setTotalPages(Math.ceil(total / pageSize));

				setErrorCount(mLogs.filter(l => l.action === 'error').length);
				setWarningCount(mLogs.filter(l => l.action === 'drop').length);
				setInfoCount(mLogs.filter(l => l.action !== 'error' && l.action !== 'drop').length);
			} else if (activeTab === 'application') {
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
				const aLogs = response.logs || [];
				const total = response.total || 0;
				setLogs(aLogs);
				setTotalLogs(total);
				setApplicationLogsCount(total);
				setTotalPages(Math.ceil(total / pageSize));

				setErrorCount(aLogs.filter(l => l.level === 'error').length);
				setWarningCount(aLogs.filter(l => l.level === 'warning' || l.level === 'warn').length);
				setInfoCount(aLogs.filter(l => l.level === 'info').length);
			}
		} catch (error) {
			console.error('Failed to load logs:', error);
			setLogs([]);
			setMessageLogs([]);
		} finally {
			setLoading(false);
		}
	}

	async function handleFilterChange() {
		setCurrentPage(1);
		await loadLogs();
		await updateChart();
	}

	async function handleTabChange(tab: string) {
		setActiveTab(tab);
		setCurrentPage(1);
	}

	async function loadTabCounts() {
		try {
			const [messageResponse, appResponse] = await Promise.all([
				api.get<MessageLogsResponse>(`/admin/logs/messages?page=1&size=1&range=${selectedTimeRange}`),
				api.get<LogsResponse>(`/admin/logs?page=1&size=1&range=${selectedTimeRange}`)
			]);

			setMessageLogsCount(messageResponse.total || 0);
			setApplicationLogsCount(appResponse.total || 0);
		} catch (error) {
			console.error('Failed to load tab counts:', error);
		}
	}

	async function loadChartData() {
		try {
			const [appLogsResponse, messageLogsResponse] = await Promise.all([
				api.get<LogsResponse>(`/admin/logs?page=1&size=1000&range=${selectedTimeRange}`),
				api.get<MessageLogsResponse>(`/admin/logs/messages?page=1&size=1000&range=${selectedTimeRange}`)
			]);

			const chartAppLogs = appLogsResponse.logs || [];
			const chartMsgLogs = messageLogsResponse.logs || [];

			const hours = Array.from({ length: 24 }, (_, i) => i);
			const appLogsByHour = new Array(24).fill(0);
			const msgLogsByHour = new Array(24).fill(0);
			const errorsByHour = new Array(24).fill(0);

			chartAppLogs.forEach((log: LogEntry) => {
				const hour = new Date(log.timestamp).getHours();
				appLogsByHour[hour]++;
				if (log.level === 'error' || log.level === 'fatal') {
					errorsByHour[hour]++;
				}
			});

			chartMsgLogs.forEach((log: MessageLog) => {
				const hour = new Date(log.createdAt).getHours();
				msgLogsByHour[hour]++;
				if (log.action === 'error') {
					errorsByHour[hour]++;
				}
			});

			return {
				labels: hours.map(h => `${h}:00`),
				appLogs: appLogsByHour,
				messageLogs: msgLogsByHour,
				errors: errorsByHour
			};
		} catch (error) {
			console.error('Failed to load chart data:', error);
			return {
				labels: Array.from({ length: 24 }, (_, i) => `${i}:00`),
				appLogs: new Array(24).fill(0),
				messageLogs: new Array(24).fill(0),
				errors: new Array(24).fill(0)
			};
		}
	}

	async function updateChart() {
		const chartData = await loadChartData();

		if (activityChartRef.current) {
			activityChartRef.current.data.labels = chartData.labels;
			activityChartRef.current.data.datasets = [
				{
					label: 'Application Logs',
					data: chartData.appLogs,
					borderColor: '#3b82f6',
					backgroundColor: 'rgba(59, 130, 246, 0.1)',
					tension: 0.4
				},
				{
					label: 'Message Logs',
					data: chartData.messageLogs,
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
			activityChartRef.current.update();
		}
	}

	function initChart(chartData: { labels: string[]; appLogs: number[]; messageLogs: number[]; errors: number[] }) {
		if (!chartCanvasRef.current) return;

		if (activityChartRef.current) {
			activityChartRef.current.destroy();
		}

		activityChartRef.current = new Chart(chartCanvasRef.current, {
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
						label: 'Message Logs',
						data: chartData.messageLogs,
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

	// Initial load + chart setup
	useEffect(() => {
		(async () => {
			await loadLogs();
			await loadTabCounts();
			const chartData = await loadChartData();
			setTimeout(() => initChart(chartData), 100);
		})();

		return () => {
			if (activityChartRef.current) activityChartRef.current.destroy();
			if (refreshIntervalRef.current) clearInterval(refreshIntervalRef.current);
		};
	}, []);

	// Reload when activeTab changes
	useEffect(() => {
		loadLogs();
		loadTabCounts();
	}, [activeTab]);

	return html`
		<>
			<div class="logs-page">
				<!-- Header -->
				<${PageHeader} title="System Logs" icon=${FileText}>
					<span class="meta-item">${totalLogs.toLocaleString()} total</span>
					<span class="meta-separator">\u2022</span>
					<span class="meta-item error">${errorCount} errors</span>
					<span class="meta-separator">\u2022</span>
					<span class="meta-item warning">${warningCount} warnings</span>
					<span class="meta-separator">\u2022</span>
					<span class="meta-item info">${infoCount} info</span>
				</${PageHeader}>

				<!-- Content Area -->
				<div class="content-area">

					<!-- Activity Chart -->
					<div class="card">
						<div class="card-header">
							<h3 class="card-title">Log Activity</h3>
							<select
								class="form-select"
								style=${{ width: 'auto' }}
								value=${selectedTimeRange}
								onChange=${(e: Event) => {
									setSelectedTimeRange((e.target as HTMLSelectElement).value);
									handleFilterChange();
								}}
							>
								<option value="1h">Last Hour</option>
								<option value="24h">Last 24 Hours</option>
								<option value="7d">Last 7 Days</option>
								<option value="30d">Last 30 Days</option>
							</select>
						</div>
						<div style=${{ height: '250px' }}>
							<canvas ref=${chartCanvasRef}></canvas>
						</div>
					</div>

					<!-- Logs Table -->
					<div class="card">
						<!-- Tab Navigation -->
						<div class="tabs">
							<button
								class="tab ${activeTab === 'messages' ? 'active' : ''}"
								onClick=${() => handleTabChange('messages')}
								type="button"
							>
								Message Logs (${messageLogsCount})
							</button>
							<button
								class="tab ${activeTab === 'application' ? 'active' : ''}"
								onClick=${() => handleTabChange('application')}
								type="button"
							>
								Application Logs (${applicationLogsCount})
							</button>
						</div>

						<div class="logs-header">
							<div class="logs-filters">
								<${SearchInput}
									value=${searchQuery}
									onChange=${setSearchQuery}
									placeholder=${activeTab === 'messages' ? 'Search by kind...' : 'Search logs...'}
									maxWidth="280px"
								/>

								${activeTab === 'messages' ? html`
									<select
										class="filter-select"
										value=${selectedAction}
										onChange=${(e: Event) => {
											setSelectedAction((e.target as HTMLSelectElement).value);
											handleFilterChange();
										}}
									>
										<option value="">All Actions</option>
										<option value="continue">Continue</option>
										<option value="respond">Respond</option>
										<option value="error">Error</option>
										<option value="drop">Drop</option>
									</select>
								` : html`
									<select
										class="filter-select"
										value=${selectedLevel}
										onChange=${(e: Event) => {
											setSelectedLevel((e.target as HTMLSelectElement).value);
											handleFilterChange();
										}}
									>
										<option value="">All Levels</option>
										<option value="error">Errors</option>
										<option value="warning">Warnings</option>
										<option value="info">Info</option>
										<option value="debug">Debug</option>
									</select>
								`}

								<button
									class="btn btn-secondary btn-sm ${autoRefresh ? 'active' : ''}"
									onClick=${toggleAutoRefresh}
									type="button"
								>
									<${RefreshCw} size=${16} class=${autoRefresh ? 'spinning' : ''} />
									${autoRefresh ? 'Auto' : 'Manual'}
								</button>
							</div>

							<${ExportButton}
								data=${activeTab === 'messages' ? messageLogs : logs}
								filename="logs"
								disabled=${(activeTab === 'messages' ? messageLogs : logs).length === 0}
							/>
						</div>

						<div class="table-container">
							<table class="table logs-table">
								${activeTab === 'messages' ? html`
									<thead>
										<tr>
											<th style=${{ width: '40px' }}>Action</th>
											<th style=${{ width: '150px' }}>Timestamp</th>
											<th style=${{ width: '120px' }}>Flow</th>
											<th style=${{ width: '120px' }}>Block</th>
											<th>Kind</th>
											<th style=${{ width: '80px' }}>Duration</th>
											<th style=${{ width: '120px' }}>Details</th>
										</tr>
									</thead>
									<tbody>
										${messageLogs.map(log => {
											const level = getActionLevel(log.action);
											const LevelIcon = getLevelIcon(level);
											return html`
												<tr key=${log.id || log.createdAt}>
													<td>
														<div class="log-level" style=${{ color: getLevelColor(level) }}>
															<${LevelIcon} size=${16} />
														</div>
													</td>
													<td class="log-timestamp">
														<${Clock} size=${12} style=${{ display: 'inline', marginRight: '4px', color: 'var(--text-muted)' }} />
														${new Date(log.createdAt).toLocaleString()}
													</td>
													<td><span class="log-source">${log.flowId}</span></td>
													<td><span class="log-source">${log.blockName}</span></td>
													<td class="log-message">${log.messageKind}</td>
													<td>${log.durationMs}ms</td>
													<td>
														<div style=${{ display: 'flex', flexDirection: 'column', gap: '2px', fontSize: '0.8rem' }}>
															<span style=${{ color: 'var(--text-muted, #94a3b8)' }}>
																<span style=${{ fontWeight: 500 }}>${log.action}</span>
															</span>
															${log.error ? html`
																<span style=${{ color: 'var(--danger-color, #ef4444)', fontSize: '0.75rem' }}>
																	${log.error}
																</span>
															` : null}
														</div>
													</td>
												</tr>
											`;
										})}
									</tbody>
								` : html`
									<thead>
										<tr>
											<th style=${{ width: '40px' }}>Level</th>
											<th style=${{ width: '150px' }}>Timestamp</th>
											<th style=${{ width: '100px' }}>Source</th>
											<th>Message</th>
											<th style=${{ width: '200px' }}>Metadata</th>
										</tr>
									</thead>
									<tbody>
										${logs.map(log => {
											const LevelIcon = getLevelIcon(log.level);
											return html`
												<tr key=${log.id || log.timestamp}>
													<td>
														<div class="log-level" style=${{ color: getLevelColor(log.level) }}>
															<${LevelIcon} size=${16} />
														</div>
													</td>
													<td class="log-timestamp">
														<${Clock} size=${12} style=${{ display: 'inline', marginRight: '4px', color: 'var(--text-muted)' }} />
														${new Date(log.timestamp).toLocaleString() || 'N/A'}
													</td>
													<td>
														<span class="log-source">${log.source || 'system'}</span>
													</td>
													<td class="log-message">${log.message}</td>
													<td>
														${log.fields
															? html`
																<div style=${{ display: 'flex', flexDirection: 'column', gap: '2px', fontSize: '0.8rem' }}>
																	${Object.entries(log.fields).map(([key, value]) => html`
																		<span style=${{ color: 'var(--text-muted, #94a3b8)' }} key=${key}>
																			<span style=${{ fontWeight: 500 }}>${key}: </span>${String(value)}
																		</span>
																	`)}
																</div>
															`
															: '-'
														}
													</td>
												</tr>
											`;
										})}
									</tbody>
								`}
							</table>
						</div>
					</div>
				</div>
			</div>
		<//>
	`;
}
