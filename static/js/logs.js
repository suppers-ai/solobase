// Logs page functionality
let logsChart = null;
let currentFilters = {
    search: '',
    level: '',
    range: '24h',
    page: 1,
    size: 100
};

// Make initialization function globally available
window.initializeLogsPage = function() {
    console.log('Initializing logs page...');
    
    // Ensure modal is hidden on page load
    const modal = document.getElementById('logDetailsModal');
    if (modal) {
        modal.classList.remove('show');
        modal.style.display = 'none';
    }
    
    // Destroy existing chart if it exists
    if (logsChart) {
        logsChart.destroy();
        logsChart = null;
    }
    
    // Only initialize chart if we're on the logs page
    const logsContainer = document.querySelector('.logs-container');
    if (logsContainer) {
        initializeChart();
        loadChartData();
    }
    
    // Re-initialize Lucide icons
    if (window.lucide) {
        setTimeout(() => lucide.createIcons(), 100);
    }
};

// Initialize the page on DOM ready
document.addEventListener('DOMContentLoaded', () => {
    if (window.location.pathname === '/logs' || window.location.pathname.includes('/logs')) {
        window.initializeLogsPage();
    }
});

// Initialize the chart
function initializeChart() {
    const ctx = document.getElementById('logsChart');
    if (!ctx) {
        console.log('Logs chart canvas not found');
        return;
    }
    
    logsChart = new Chart(ctx, {
        type: 'line',
        data: {
            labels: [],
            datasets: [
                {
                    label: 'Success',
                    data: [],
                    borderColor: '#10b981',
                    backgroundColor: 'rgba(16, 185, 129, 0.1)',
                    tension: 0.3,
                    fill: true
                },
                {
                    label: 'Warning',
                    data: [],
                    borderColor: '#f59e0b',
                    backgroundColor: 'rgba(245, 158, 11, 0.1)',
                    tension: 0.3,
                    fill: true
                },
                {
                    label: 'Error',
                    data: [],
                    borderColor: '#ef4444',
                    backgroundColor: 'rgba(239, 68, 68, 0.1)',
                    tension: 0.3,
                    fill: true
                }
            ]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            plugins: {
                legend: {
                    display: false
                },
                tooltip: {
                    mode: 'index',
                    intersect: false,
                    backgroundColor: 'rgba(0, 0, 0, 0.8)',
                    titleColor: '#fff',
                    bodyColor: '#fff',
                    borderColor: '#333',
                    borderWidth: 1
                }
            },
            scales: {
                x: {
                    grid: {
                        display: false
                    },
                    ticks: {
                        maxRotation: 45,
                        minRotation: 45
                    }
                },
                y: {
                    beginAtZero: true,
                    grid: {
                        color: 'rgba(0, 0, 0, 0.05)'
                    },
                    ticks: {
                        stepSize: 1,
                        callback: function(value) {
                            if (Math.floor(value) === value) {
                                return value;
                            }
                        }
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

// Load chart data
async function loadChartData() {
    try {
        const response = await fetch(`/api/v1/admin/logs/stats?range=${currentFilters.range}`);
        if (response.ok) {
            const data = await response.json();
            
            logsChart.data.labels = data.labels || [];
            logsChart.data.datasets[0].data = data.success || [];
            logsChart.data.datasets[1].data = data.warning || [];
            logsChart.data.datasets[2].data = data.error || [];
            
            logsChart.update();
        }
    } catch (error) {
        console.error('Error loading chart data:', error);
    }
}

// Search logs
window.searchLogs = function(value) {
    currentFilters.search = value;
    currentFilters.page = 1;
    reloadLogs();
}

// Filter by level
window.filterByLevel = function(value) {
    currentFilters.level = value;
    currentFilters.page = 1;
    reloadLogs();
}

// Filter by time
window.filterByTime = function(value) {
    currentFilters.range = value;
    currentFilters.page = 1;
    loadChartData();
    reloadLogs();
}

// Refresh logs
window.refreshLogs = function() {
    loadChartData();
    reloadLogs();
}

// Export logs
window.exportLogs = async function() {
    try {
        const params = new URLSearchParams(currentFilters).toString();
        const response = await fetch(`/api/v1/admin/logs/export?${params}`);
        
        if (response.ok) {
            const blob = await response.blob();
            const url = window.URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `logs_${new Date().toISOString().split('T')[0]}.csv`;
            a.click();
            window.URL.revokeObjectURL(url);
        } else {
            alert('Error exporting logs');
        }
    } catch (error) {
        alert('Error exporting logs: ' + error.message);
    }
}

// Clear logs
window.clearLogs = async function() {
    const options = {
        '1h': 'Last hour',
        '24h': 'Last 24 hours',
        '7d': 'Last 7 days',
        '30d': 'Last 30 days',
        'all': 'All logs'
    };
    
    const selection = prompt('Clear logs older than:\n1h - Last hour\n24h - Last 24 hours\n7d - Last 7 days\n30d - Last 30 days\nall - All logs\n\nEnter option (e.g., "7d"):');
    
    if (!selection || !options[selection]) {
        alert('Invalid selection');
        return;
    }
    
    if (!confirm(`Are you sure you want to clear logs ${selection === 'all' ? '' : 'older than ' + options[selection]}?`)) {
        return;
    }
    
    try {
        const response = await fetch('/api/v1/admin/logs/clear', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ olderThan: selection })
        });
        
        if (response.ok) {
            const result = await response.json();
            alert(result.message);
            refreshLogs();
        } else {
            alert('Error clearing logs');
        }
    } catch (error) {
        alert('Error clearing logs: ' + error.message);
    }
}

// View log details
window.viewLogDetails = async function(logId) {
    try {
        const response = await fetch(`/api/v1/admin/logs/details?id=${logId}`);
        
        if (response.ok) {
            const log = await response.json();
            
            // Format the details for display
            let detailsHtml = `
                <div class="log-details">
                    <div class="detail-group">
                        <label>ID:</label>
                        <span>${log.id}</span>
                    </div>
                    <div class="detail-group">
                        <label>Time:</label>
                        <span>${log.createdAt}</span>
                    </div>
                    <div class="detail-group">
                        <label>Level:</label>
                        <span class="level-badge level-${log.level}">${log.level}</span>
                    </div>
                    ${log.method ? `
                    <div class="detail-group">
                        <label>Method:</label>
                        <span class="method-badge method-${log.method}">${log.method}</span>
                    </div>` : ''}
                    ${log.path ? `
                    <div class="detail-group">
                        <label>Path:</label>
                        <span>${log.path}</span>
                    </div>` : ''}
                    ${log.status ? `
                    <div class="detail-group">
                        <label>Status:</label>
                        <span class="status-badge ${getStatusClass(log.status)}">${log.status}</span>
                    </div>` : ''}
                    ${log.duration && log.duration !== '-' ? `
                    <div class="detail-group">
                        <label>Duration:</label>
                        <span>${log.duration}</span>
                    </div>` : ''}
                    ${log.userID ? `
                    <div class="detail-group">
                        <label>User ID:</label>
                        <span>${log.userID}</span>
                    </div>` : ''}
                    ${log.userIP ? `
                    <div class="detail-group">
                        <label>User IP:</label>
                        <span>${log.userIP}</span>
                    </div>` : ''}
                    <div class="detail-group full-width">
                        <label>Message:</label>
                        <div class="detail-message">${log.message}</div>
                    </div>
                    ${log.error ? `
                    <div class="detail-group full-width">
                        <label>Error:</label>
                        <div class="detail-error">${log.error}</div>
                    </div>` : ''}
                    ${log.stack ? `
                    <div class="detail-group full-width">
                        <label>Stack Trace:</label>
                        <pre class="detail-stack">${log.stack}</pre>
                    </div>` : ''}
                    ${log.caller ? `
                    <div class="detail-group full-width">
                        <label>Caller:</label>
                        <span>${log.caller}</span>
                    </div>` : ''}
                    ${log.details && Object.keys(log.details).length > 0 ? `
                    <div class="detail-group full-width">
                        <label>Additional Details:</label>
                        <pre class="detail-json">${JSON.stringify(log.details, null, 2)}</pre>
                    </div>` : ''}
                </div>
            `;
            
            document.getElementById('logDetailsContent').innerHTML = detailsHtml;
            document.getElementById('logDetailsModal').classList.add('show');
            
            // Re-initialize Lucide icons
            if (window.lucide) {
                setTimeout(() => lucide.createIcons(), 100);
            }
        } else {
            alert('Error loading log details');
        }
    } catch (error) {
        alert('Error loading log details: ' + error.message);
    }
}

// Close log details modal
window.closeLogDetails = function() {
    document.getElementById('logDetailsModal').classList.remove('show');
}

// Copy log to clipboard
window.copyLog = async function(logId) {
    try {
        const response = await fetch(`/api/v1/admin/logs/details?id=${logId}`);
        
        if (response.ok) {
            const log = await response.json();
            const logText = JSON.stringify(log, null, 2);
            
            await navigator.clipboard.writeText(logText);
            
            // Show temporary feedback
            const button = event.currentTarget;
            const originalTitle = button.title;
            button.title = 'Copied!';
            setTimeout(() => {
                button.title = originalTitle;
            }, 2000);
        }
    } catch (error) {
        alert('Error copying log: ' + error.message);
    }
}

// Toggle all logs selection
window.toggleAllLogs = function(checkbox) {
    const checkboxes = document.querySelectorAll('input[name="log-select"]');
    checkboxes.forEach(cb => {
        cb.checked = checkbox.checked;
    });
}

// Pagination functions
window.previousPage = function() {
    if (currentFilters.page > 1) {
        currentFilters.page--;
        reloadLogs();
    }
}

window.nextPage = function() {
    currentFilters.page++;
    reloadLogs();
}

window.changePageSize = function(size) {
    currentFilters.size = parseInt(size);
    currentFilters.page = 1;
    reloadLogs();
}

// Reload logs with current filters
function reloadLogs() {
    const params = new URLSearchParams(currentFilters).toString();
    window.location.href = `/logs?${params}`;
}

// Helper function to get status class
function getStatusClass(status) {
    if (status >= 200 && status < 300) {
        return 'status-success';
    } else if (status >= 300 && status < 400) {
        return 'status-redirect';
    } else if (status >= 400 && status < 500) {
        return 'status-warning';
    }
    return 'status-error';
}

// Close modal when clicking outside
document.addEventListener('click', (event) => {
    const modal = document.getElementById('logDetailsModal');
    if (event.target === modal) {
        closeLogDetails();
    }
});