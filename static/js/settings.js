// Settings page functionality

// Tab switching functionality
window.switchTab = function(tabName) {
    // Update tab buttons
    document.querySelectorAll('.tab-btn').forEach(btn => {
        if (btn.dataset.tab === tabName) {
            btn.classList.add('active');
        } else {
            btn.classList.remove('active');
        }
    });
    
    // Update tab panes
    document.querySelectorAll('.tab-pane').forEach(pane => {
        pane.classList.remove('active');
    });
    
    const targetPane = document.getElementById(`${tabName}-tab`);
    if (targetPane) {
        targetPane.classList.add('active');
        
        // Re-initialize Lucide icons in the new tab
        if (window.lucide) {
            setTimeout(() => lucide.createIcons(), 100);
        }
    }
    
    // Save selected tab to localStorage
    localStorage.setItem('selectedSettingsTab', tabName);
}

// Restore previously selected tab
document.addEventListener('DOMContentLoaded', () => {
    const savedTab = localStorage.getItem('selectedSettingsTab');
    if (savedTab) {
        switchTab(savedTab);
    } else {
        // Initialize Lucide icons for the default tab
        if (window.lucide) {
            setTimeout(() => lucide.createIcons(), 100);
        }
    }
});

// Save application settings
window.saveAppSettings = async function(event) {
    event.preventDefault();
    
    const form = event.target;
    const formData = new FormData(form);
    
    try {
        const response = await fetch('/api/v1/admin/settings/app', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                appName: formData.get('appName'),
                maintenanceMode: formData.get('maintenanceMode') === 'on'
            })
        });
        
        if (response.ok) {
            alert('Application settings saved successfully');
        } else {
            const error = await response.json();
            alert('Error saving settings: ' + error.message);
        }
    } catch (error) {
        alert('Error saving settings: ' + error.message);
    }
}

// Save auth settings
window.saveAuthSettings = async function(event) {
    event.preventDefault();
    
    const form = event.target;
    const formData = new FormData(form);
    
    try {
        const response = await fetch('/api/v1/admin/settings/auth', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                enableSignup: formData.get('enableSignup') === 'on',
                enableAPI: formData.get('enableAPI') === 'on'
            })
        });
        
        if (response.ok) {
            alert('Authentication settings saved successfully');
        } else {
            const error = await response.json();
            alert('Error saving settings: ' + error.message);
        }
    } catch (error) {
        alert('Error saving settings: ' + error.message);
    }
}

// Save email settings
window.saveEmailSettings = async function(event) {
    event.preventDefault();
    
    const form = event.target;
    const formData = new FormData(form);
    
    try {
        const response = await fetch('/api/v1/admin/settings/email', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                smtpHost: formData.get('smtpHost'),
                smtpPort: parseInt(formData.get('smtpPort'))
            })
        });
        
        if (response.ok) {
            alert('Email settings saved successfully');
        } else {
            const error = await response.json();
            alert('Error saving settings: ' + error.message);
        }
    } catch (error) {
        alert('Error saving settings: ' + error.message);
    }
}

// Save rate limit settings
window.saveRateLimitSettings = async function(event) {
    event.preventDefault();
    
    const form = event.target;
    const formData = new FormData(form);
    
    try {
        const response = await fetch('/api/v1/admin/settings/ratelimit', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                enabled: formData.get('rateLimitEnabled') === 'on',
                requestsPerMinute: parseInt(formData.get('rateLimitRPM'))
            })
        });
        
        if (response.ok) {
            alert('Rate limit settings saved successfully');
        } else {
            const error = await response.json();
            alert('Error saving settings: ' + error.message);
        }
    } catch (error) {
        alert('Error saving settings: ' + error.message);
    }
}

// Save logging settings
window.saveLoggingSettings = async function(event) {
    event.preventDefault();
    
    const form = event.target;
    const formData = new FormData(form);
    
    try {
        const response = await fetch('/api/v1/admin/settings/logging', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                logLevel: formData.get('logLevel')
            })
        });
        
        if (response.ok) {
            alert('Logging settings saved successfully');
        } else {
            const error = await response.json();
            alert('Error saving settings: ' + error.message);
        }
    } catch (error) {
        alert('Error saving settings: ' + error.message);
    }
}

// Test database connection
window.testDatabaseConnection = async function() {
    try {
        const response = await fetch('/api/v1/admin/test/database');
        
        if (response.ok) {
            alert('Database connection successful');
        } else {
            alert('Database connection failed');
        }
    } catch (error) {
        alert('Error testing database connection: ' + error.message);
    }
}

// Reset database
window.resetDatabase = async function() {
    if (!confirm('Are you sure you want to reset the database? This action cannot be undone.')) {
        return;
    }
    
    if (!confirm('This will DELETE ALL DATA. Are you absolutely sure?')) {
        return;
    }
    
    try {
        const response = await fetch('/api/v1/admin/database/reset', {
            method: 'POST'
        });
        
        if (response.ok) {
            alert('Database reset successfully');
            window.location.href = '/auth/logout';
        } else {
            const error = await response.json();
            alert('Error resetting database: ' + error.message);
        }
    } catch (error) {
        alert('Error resetting database: ' + error.message);
    }
}

// Test storage connection
window.testStorageConnection = async function() {
    try {
        const response = await fetch('/api/v1/admin/test/storage');
        
        if (response.ok) {
            alert('Storage connection successful');
        } else {
            alert('Storage connection failed');
        }
    } catch (error) {
        alert('Error testing storage connection: ' + error.message);
    }
}

// Send test email
window.sendTestEmail = async function() {
    const email = prompt('Enter recipient email address:');
    if (!email) return;
    
    try {
        const response = await fetch('/api/v1/admin/test/email', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ to: email })
        });
        
        if (response.ok) {
            alert('Test email sent successfully');
        } else {
            const error = await response.json();
            alert('Error sending test email: ' + error.message);
        }
    } catch (error) {
        alert('Error sending test email: ' + error.message);
    }
}

// Clear all caches
window.clearAllCaches = async function() {
    if (!confirm('Are you sure you want to clear all caches?')) {
        return;
    }
    
    try {
        const response = await fetch('/api/v1/admin/cache/clear', {
            method: 'POST'
        });
        
        if (response.ok) {
            alert('All caches cleared successfully');
            window.location.reload();
        } else {
            const error = await response.json();
            alert('Error clearing caches: ' + error.message);
        }
    } catch (error) {
        alert('Error clearing caches: ' + error.message);
    }
}

// Export all data
window.exportAllData = async function() {
    try {
        const response = await fetch('/api/v1/admin/export');
        
        if (response.ok) {
            const blob = await response.blob();
            const url = window.URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = 'solobase_export.zip';
            a.click();
            window.URL.revokeObjectURL(url);
        } else {
            alert('Error exporting data');
        }
    } catch (error) {
        alert('Error exporting data: ' + error.message);
    }
}

// Delete all data
window.deleteAllData = async function() {
    if (!confirm('Are you sure you want to delete ALL data? This action cannot be undone.')) {
        return;
    }
    
    if (!confirm('This will DELETE EVERYTHING including users, files, and all database records. Are you absolutely sure?')) {
        return;
    }
    
    const confirmation = prompt('Type "DELETE ALL" to confirm:');
    if (confirmation !== 'DELETE ALL') {
        alert('Deletion cancelled');
        return;
    }
    
    try {
        const response = await fetch('/api/v1/admin/data/delete-all', {
            method: 'DELETE'
        });
        
        if (response.ok) {
            alert('All data deleted successfully');
            window.location.href = '/auth/logout';
        } else {
            const error = await response.json();
            alert('Error deleting data: ' + error.message);
        }
    } catch (error) {
        alert('Error deleting data: ' + error.message);
    }
}