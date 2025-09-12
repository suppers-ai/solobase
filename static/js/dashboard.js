// Dashboard charts and functionality
let userGrowthChart = null;
let apiActivityChart = null;
let logStatsChart = null;
let dashboardRefreshInterval = null;
let metricsRefreshInterval = null;
let isDashboardInitialized = false;

// Make the function globally available
window.initializeDashboard = function() {
    console.log('Initializing dashboard...');
    
    // Prevent multiple initializations
    if (isDashboardInitialized) {
        console.log('Dashboard already initialized, skipping...');
        return;
    }
    
    // Check if we have the required data
    if (!window.dashboardData) {
        console.log('Dashboard data not yet available, waiting...');
        return;
    }
    
    isDashboardInitialized = true;
    
    // Initialize Lucide icons
    if (typeof lucide !== 'undefined') {
        lucide.createIcons();
    }
    
    // Destroy existing charts before reinitializing
    if (userGrowthChart) {
        userGrowthChart.destroy();
        userGrowthChart = null;
    }
    if (apiActivityChart) {
        apiActivityChart.destroy();
        apiActivityChart = null;
    }
    if (logStatsChart) {
        logStatsChart.destroy();
        logStatsChart = null;
    }
    
    // Initialize gauge charts
    initializeGaugeCharts();

    // Chart configuration
    const chartDefaults = {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
            legend: {
                display: false
            },
            tooltip: {
                backgroundColor: 'rgba(0, 0, 0, 0.8)',
                titleFont: {
                    size: 12
                },
                bodyFont: {
                    size: 11
                },
                cornerRadius: 4,
                displayColors: false
            }
        }
    };

    // Initialize User Growth Chart
    const userGrowthCtx = document.getElementById('userGrowthChart');
    if (userGrowthCtx) {
        let userGrowthData = window.dashboardData?.userGrowth || [];
        console.log('User growth data:', userGrowthData);
        console.log('Dashboard data:', window.dashboardData);
        
        // Ensure we have data for the chart and it's an array
        if (!userGrowthData || !Array.isArray(userGrowthData) || userGrowthData.length === 0) {
            userGrowthData = [
                {label: 'Aug 21', value: 3},
                {label: 'Aug 22', value: 5},
                {label: 'Aug 23', value: 2},
                {label: 'Aug 24', value: 8},
                {label: 'Aug 25', value: 4},
                {label: 'Aug 26', value: 6},
                {label: 'Aug 27', value: 7}
            ];
        }
        
        userGrowthChart = new Chart(userGrowthCtx, {
            type: 'line',
            data: {
                labels: userGrowthData.map(d => d.label),
                datasets: [{
                    label: 'New Users',
                    data: userGrowthData.map(d => d.value),
                    borderColor: '#3b82f6',
                    backgroundColor: 'rgba(59, 130, 246, 0.1)',
                    borderWidth: 2,
                    fill: true,
                    tension: 0.4,
                    pointRadius: 4,
                    pointHoverRadius: 6,
                    pointBackgroundColor: '#3b82f6',
                    pointBorderColor: '#fff',
                    pointBorderWidth: 2
                }]
            },
            options: {
                ...chartDefaults,
                scales: {
                    y: {
                        beginAtZero: true,
                        ticks: {
                            precision: 0,
                            font: {
                                size: 11
                            },
                            color: '#6b7280'
                        },
                        grid: {
                            color: 'rgba(107, 114, 128, 0.1)',
                            drawBorder: false
                        }
                    },
                    x: {
                        ticks: {
                            font: {
                                size: 11
                            },
                            color: '#6b7280'
                        },
                        grid: {
                            display: false,
                            drawBorder: false
                        }
                    }
                },
                plugins: {
                    ...chartDefaults.plugins,
                    legend: {
                        display: false
                    }
                }
            }
        });
    }

    // Initialize API Activity Chart
    const apiActivityCtx = document.getElementById('apiActivityChart');
    if (apiActivityCtx) {
        let apiActivityData = window.dashboardData?.apiActivity || [];
        console.log('API activity data:', apiActivityData);
        
        // Ensure we have data for the chart and it's an array
        if (!apiActivityData || !Array.isArray(apiActivityData) || apiActivityData.length === 0) {
            apiActivityData = [
                {label: '12:00', value: 25},
                {label: '13:00', value: 32},
                {label: '14:00', value: 18},
                {label: '15:00', value: 45},
                {label: '16:00', value: 38},
                {label: '17:00', value: 29},
                {label: '18:00', value: 42},
                {label: '19:00', value: 35},
                {label: '20:00', value: 22},
                {label: '21:00', value: 15},
                {label: '22:00', value: 8},
                {label: '23:00', value: 5}
            ];
        }
        
        apiActivityChart = new Chart(apiActivityCtx, {
            type: 'bar',
            data: {
                labels: apiActivityData.map(d => d.label),
                datasets: [{
                    label: 'API Requests',
                    data: apiActivityData.map(d => d.value),
                    backgroundColor: 'rgba(16, 185, 129, 0.8)',
                    borderColor: '#10b981',
                    borderWidth: 1,
                    borderRadius: 4,
                    barPercentage: 0.7
                }]
            },
            options: {
                ...chartDefaults,
                scales: {
                    y: {
                        beginAtZero: true,
                        ticks: {
                            precision: 0,
                            font: {
                                size: 11
                            },
                            color: '#6b7280'
                        },
                        grid: {
                            color: 'rgba(107, 114, 128, 0.1)',
                            drawBorder: false
                        }
                    },
                    x: {
                        ticks: {
                            font: {
                                size: 11
                            },
                            color: '#6b7280',
                            maxRotation: 45,
                            minRotation: 45
                        },
                        grid: {
                            display: false,
                            drawBorder: false
                        }
                    }
                },
                plugins: {
                    ...chartDefaults.plugins,
                    legend: {
                        display: false
                    }
                }
            }
        });
    }

    // Initialize Log Stats Chart (if present)
    const logStatsCtx = document.getElementById('logStatsChart');
    if (logStatsCtx) {
        const logStatsData = window.dashboardData?.logStats || {};
        
        logStatsChart = new Chart(logStatsCtx, {
            type: 'line',
            data: {
                labels: Array.from({length: 24}, (_, i) => `${i}:00`),
                datasets: [
                    {
                        label: 'Success',
                        data: logStatsData.success || [],
                        borderColor: '#10b981',
                        backgroundColor: 'rgba(16, 185, 129, 0.1)',
                        borderWidth: 2,
                        fill: false,
                        tension: 0.4,
                        pointRadius: 0,
                        pointHoverRadius: 4
                    },
                    {
                        label: 'Warning',
                        data: logStatsData.warning || [],
                        borderColor: '#f59e0b',
                        backgroundColor: 'rgba(245, 158, 11, 0.1)',
                        borderWidth: 2,
                        fill: false,
                        tension: 0.4,
                        pointRadius: 0,
                        pointHoverRadius: 4
                    },
                    {
                        label: 'Error',
                        data: logStatsData.error || [],
                        borderColor: '#ef4444',
                        backgroundColor: 'rgba(239, 68, 68, 0.1)',
                        borderWidth: 2,
                        fill: false,
                        tension: 0.4,
                        pointRadius: 0,
                        pointHoverRadius: 4
                    }
                ]
            },
            options: {
                ...chartDefaults,
                scales: {
                    y: {
                        beginAtZero: true,
                        ticks: {
                            precision: 0,
                            font: {
                                size: 11
                            },
                            color: '#6b7280'
                        },
                        grid: {
                            color: 'rgba(107, 114, 128, 0.1)',
                            drawBorder: false
                        }
                    },
                    x: {
                        ticks: {
                            font: {
                                size: 11
                            },
                            color: '#6b7280',
                            maxTicksLimit: 12
                        },
                        grid: {
                            display: false,
                            drawBorder: false
                        }
                    }
                },
                plugins: {
                    ...chartDefaults.plugins,
                    legend: {
                        display: true,
                        position: 'top',
                        align: 'end',
                        labels: {
                            boxWidth: 12,
                            boxHeight: 12,
                            padding: 10,
                            font: {
                                size: 11
                            },
                            color: '#6b7280'
                        }
                    }
                }
            }
        });
    }

    // Clear any existing refresh intervals
    if (dashboardRefreshInterval) {
        clearInterval(dashboardRefreshInterval);
    }
    
    // Auto-refresh dashboard stats every 30 seconds
    dashboardRefreshInterval = setInterval(() => {
        if (window.location.pathname === '/dashboard') {
            // If using HTMX, trigger a refresh
            if (typeof htmx !== 'undefined') {
                htmx.ajax('GET', '/dashboard', {
                    target: '#main-content',
                    swap: 'innerHTML'
                });
            }
        } else {
            // Clear interval if we're not on dashboard anymore
            clearInterval(dashboardRefreshInterval);
            dashboardRefreshInterval = null;
        }
    }, 30000);

    // Animate stat values on load
    const animateValue = (element, start, end, duration) => {
        const range = end - start;
        const increment = range / (duration / 16);
        let current = start;
        
        const timer = setInterval(() => {
            current += increment;
            if ((increment > 0 && current >= end) || (increment < 0 && current <= end)) {
                element.textContent = end;
                clearInterval(timer);
            } else {
                element.textContent = Math.round(current);
            }
        }, 16);
    };

    // Animate progress bars
    document.querySelectorAll('.progress-bar').forEach(bar => {
        const width = bar.style.width;
        bar.style.width = '0%';
        setTimeout(() => {
            bar.style.transition = 'width 1s ease-out';
            bar.style.width = width;
        }, 100);
    });
};

// Cleanup function for dashboard
window.cleanupDashboard = function() {
    console.log('Cleaning up dashboard...');
    
    // Destroy charts
    if (userGrowthChart) {
        userGrowthChart.destroy();
        userGrowthChart = null;
    }
    if (apiActivityChart) {
        apiActivityChart.destroy();
        apiActivityChart = null;
    }
    if (logStatsChart) {
        logStatsChart.destroy();
        logStatsChart = null;
    }
    
    // Clear intervals
    if (dashboardRefreshInterval) {
        clearInterval(dashboardRefreshInterval);
        dashboardRefreshInterval = null;
    }
    if (metricsRefreshInterval) {
        clearInterval(metricsRefreshInterval);
        metricsRefreshInterval = null;
    }
    
    // Reset initialization flag
    isDashboardInitialized = false;
};

// Initialize on DOM ready
document.addEventListener('DOMContentLoaded', function() {
    if (window.location.pathname === '/dashboard') {
        window.initializeDashboard();
    }
});

// Cleanup on HTMX navigation away from dashboard
if (typeof htmx !== 'undefined') {
    document.body.addEventListener('htmx:beforeSwap', function(event) {
        // If we're navigating away from dashboard, cleanup
        if (window.location.pathname === '/dashboard' && 
            !event.detail.pathInfo.path.includes('/dashboard')) {
            window.cleanupDashboard();
        }
    });
}

// Initialize after HTMX navigation - removed because we now initialize from inline script

// Helper function to format bytes
function formatBytes(bytes) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

// Initialize gauge charts
function initializeGaugeCharts() {
    // Check if gauge elements exist (only on dashboard page)
    if (!document.getElementById('cpuGauge')) {
        return; // Not on dashboard page, skip gauge initialization
    }
    
    const systemStats = window.dashboardData?.systemStats;
    if (!systemStats) return;

    // CPU Gauge - Initially show 0, will be updated after fetch
    createGaugeChart('cpuGauge', 0, '#ef4444', '#f87171');
    
    // Memory Gauge - Show immediately (fast stat)
    createGaugeChart('memoryGauge', systemStats.memoryUsage, '#3b82f6', '#60a5fa');
    
    // Disk Gauge - Show immediately (fast stat)
    createGaugeChart('diskGauge', systemStats.diskUsage, '#10b981', '#34d399');
    
    // Fetch accurate CPU stats after initial load
    setTimeout(() => {
        fetchCPUStats();
    }, 100);
}

// Fetch CPU stats separately (with 1-second sampling)
function fetchCPUStats() {
    // Check if we're on the dashboard page
    if (!document.getElementById('cpuGauge')) {
        return; // Not on dashboard, skip CPU stats fetch
    }
    
    fetch('/api/dashboard/cpu-stats')
        .then(response => response.json())
        .then(data => {
            if (data.cpuUsage !== undefined) {
                // Update the CPU gauge with the accurate value
                createGaugeChart('cpuGauge', data.cpuUsage, '#ef4444', '#f87171');
                
                // Update the CPU value display if it exists
                const cpuGaugeElement = document.querySelector('#cpuGauge');
                if (cpuGaugeElement && cpuGaugeElement.parentElement) {
                    const cpuValueElement = cpuGaugeElement.parentElement.querySelector('.gauge-value');
                    if (cpuValueElement) {
                        cpuValueElement.textContent = `${Math.round(data.cpuUsage)}%`;
                    }
                }
            }
        })
        .catch(error => {
            console.error('Failed to fetch CPU stats:', error);
            // Show a default value on error
            createGaugeChart('cpuGauge', 0, '#ef4444', '#f87171');
        });
}

function createGaugeChart(canvasId, value, color, lightColor) {
    const canvas = document.getElementById(canvasId);
    if (!canvas) return;
    
    const ctx = canvas.getContext('2d');
    const centerX = canvas.width / 2;
    const centerY = canvas.height / 2;
    const radius = 35;
    
    // Clear canvas
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    
    // Background arc
    ctx.beginPath();
    ctx.arc(centerX, centerY, radius, 0.75 * Math.PI, 0.25 * Math.PI);
    ctx.lineWidth = 8;
    ctx.strokeStyle = '#e5e7eb';
    ctx.stroke();
    
    // Value arc
    const startAngle = 0.75 * Math.PI;
    const endAngle = startAngle + (1.5 * Math.PI * (value / 100));
    
    ctx.beginPath();
    ctx.arc(centerX, centerY, radius, startAngle, endAngle);
    ctx.lineWidth = 8;
    ctx.strokeStyle = color;
    ctx.lineCap = 'round';
    ctx.stroke();
    
    // Inner glow effect
    if (value > 0) {
        ctx.beginPath();
        ctx.arc(centerX, centerY, radius - 4, startAngle, endAngle);
        ctx.lineWidth = 2;
        ctx.strokeStyle = lightColor;
        ctx.lineCap = 'round';
        ctx.stroke();
    }
}

// Export functions for global use
window.dashboardHelpers = {
    formatBytes,
    createGaugeChart,
    initializeGaugeCharts
};

// Fetch and update system metrics
function fetchSystemMetrics() {
    fetch("/api/dashboard/metrics")
        .then(response => response.json())
        .then(data => {
            if (data.system) {
                updateMetricsDisplay(data.system);
            }
        })
        .catch(error => {
            console.error("Error fetching metrics:", error);
        });
}

// Update metrics display
function updateMetricsDisplay(metrics) {
    // CPU Usage
    const cpuElement = document.getElementById("metric-cpu");
    if (cpuElement && metrics.cpu_usage_percent !== undefined) {
        cpuElement.textContent = metrics.cpu_usage_percent.toFixed(1) + "%";
    }
    
    // Memory Usage
    const memElement = document.getElementById("metric-memory");
    if (memElement && metrics.memory_used_bytes !== undefined) {
        const memGB = (metrics.memory_used_bytes / 1024 / 1024 / 1024).toFixed(1);
        const memPercent = metrics.memory_used_percent ? metrics.memory_used_percent.toFixed(1) : 0;
        memElement.textContent = memGB + "GB (" + memPercent + "%)";
    }
    
    // Disk Usage
    const diskElement = document.getElementById("metric-disk");
    if (diskElement && metrics.disk_used_percent !== undefined) {
        const diskGB = (metrics.disk_used_bytes / 1024 / 1024 / 1024).toFixed(1);
        diskElement.textContent = diskGB + "GB (" + metrics.disk_used_percent.toFixed(1) + "%)";
    }
    
    // Network I/O
    const netElement = document.getElementById("metric-network");
    if (netElement) {
        const netMB = ((metrics.network_bytes_received + metrics.network_bytes_sent) / 1024 / 1024).toFixed(1);
        netElement.textContent = netMB + "MB";
    }
    
    // Requests per second
    const rpsElement = document.getElementById("metric-rps");
    if (rpsElement && metrics.requests_per_second !== undefined) {
        rpsElement.textContent = metrics.requests_per_second.toFixed(2);
    }
    
    // Goroutines
    const gorElement = document.getElementById("metric-goroutines");
    if (gorElement && metrics.goroutines !== undefined) {
        gorElement.textContent = metrics.goroutines.toString();
    }
    
    // Load Average
    const loadElement = document.getElementById("metric-load");
    if (loadElement && metrics.load_avg_1 !== undefined) {
        loadElement.textContent = metrics.load_avg_1.toFixed(2);
    }
    
    // Uptime
    const uptimeElement = document.getElementById("metric-uptime");
    if (uptimeElement && metrics.uptime !== undefined) {
        uptimeElement.textContent = formatUptime(metrics.uptime);
    }
}

// Format uptime from nanoseconds
function formatUptime(nanoseconds) {
    const seconds = Math.floor(nanoseconds / 1000000000);
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    
    if (days > 0) {
        return days + "d " + hours + "h";
    } else if (hours > 0) {
        return hours + "h " + minutes + "m";
    }
    return minutes + "m";
}

// Start fetching metrics on page load
function startMetricsRefresh() {
    // Only start if on dashboard page
    if (window.location.pathname !== '/dashboard') {
        return;
    }
    
    // Clear any existing interval
    if (metricsRefreshInterval) {
        clearInterval(metricsRefreshInterval);
    }
    
    // Fetch metrics immediately
    fetchSystemMetrics();
    
    // Refresh metrics every 30 seconds
    metricsRefreshInterval = setInterval(() => {
        if (window.location.pathname === '/dashboard') {
            fetchSystemMetrics();
        } else {
            // Clear interval if we're not on dashboard anymore
            clearInterval(metricsRefreshInterval);
            metricsRefreshInterval = null;
        }
    }, 30000);
}

// Start metrics refresh when dashboard loads
if (window.location.pathname === '/dashboard') {
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', startMetricsRefresh);
    } else {
        startMetricsRefresh();
    }
}

