// Users page functionality

// Update user role
window.updateUserRole = async function(userId, role) {
    try {
        const response = await fetch(`/admin/users/${userId}/role`, {
            method: 'PUT',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ role: role })
        });
        
        if (response.ok) {
            // Show success message
            showToast('Role updated successfully', 'success');
            // Reload the page to reflect changes
            setTimeout(() => window.location.reload(), 1000);
        } else {
            const error = await response.json();
            showToast('Error updating role: ' + error.error, 'error');
        }
    } catch (error) {
        showToast('Error updating role: ' + error.message, 'error');
    }
}

// Lock user
window.lockUser = async function(userId) {
    if (!confirm('Are you sure you want to lock this user?')) {
        return;
    }
    
    try {
        const response = await fetch(`/admin/users/${userId}/lock`, {
            method: 'POST'
        });
        
        if (response.ok) {
            showToast('User locked successfully', 'success');
            setTimeout(() => window.location.reload(), 1000);
        } else {
            const error = await response.json();
            showToast('Error locking user: ' + error.error, 'error');
        }
    } catch (error) {
        showToast('Error locking user: ' + error.message, 'error');
    }
}

// Unlock user
window.unlockUser = async function(userId) {
    try {
        const response = await fetch(`/admin/users/${userId}/unlock`, {
            method: 'POST'
        });
        
        if (response.ok) {
            showToast('User unlocked successfully', 'success');
            setTimeout(() => window.location.reload(), 1000);
        } else {
            const error = await response.json();
            showToast('Error unlocking user: ' + error.error, 'error');
        }
    } catch (error) {
        showToast('Error unlocking user: ' + error.message, 'error');
    }
}

// Send password reset
window.sendPasswordReset = async function(userId) {
    if (!confirm('Send password reset email to this user?')) {
        return;
    }
    
    try {
        const response = await fetch(`/admin/users/${userId}/password-reset`, {
            method: 'POST'
        });
        
        if (response.ok) {
            showToast('Password reset email sent successfully', 'success');
        } else {
            const error = await response.json();
            showToast('Error sending password reset: ' + error.error, 'error');
        }
    } catch (error) {
        showToast('Error sending password reset: ' + error.message, 'error');
    }
}

// Show toast notification
function showToast(message, type = 'info') {
    // Create toast element if it doesn't exist
    let toast = document.getElementById('toast-notification');
    if (!toast) {
        toast = document.createElement('div');
        toast.id = 'toast-notification';
        toast.style.cssText = `
            position: fixed;
            top: 20px;
            right: 20px;
            padding: 16px 24px;
            border-radius: 8px;
            color: white;
            font-weight: 500;
            z-index: 9999;
            transition: opacity 0.3s ease;
            max-width: 400px;
        `;
        document.body.appendChild(toast);
    }
    
    // Set background color based on type
    const colors = {
        success: '#10b981',
        error: '#ef4444',
        info: '#3b82f6',
        warning: '#f59e0b'
    };
    toast.style.backgroundColor = colors[type] || colors.info;
    
    // Set message and show
    toast.textContent = message;
    toast.style.opacity = '1';
    toast.style.display = 'block';
    
    // Hide after 3 seconds
    setTimeout(() => {
        toast.style.opacity = '0';
        setTimeout(() => {
            toast.style.display = 'none';
        }, 300);
    }, 3000);
}

// Initialize tooltips for action buttons
document.addEventListener('DOMContentLoaded', function() {
    // Add hover effects to action buttons
    const buttons = document.querySelectorAll('.action-buttons button');
    buttons.forEach(button => {
        button.addEventListener('mouseenter', function() {
            this.style.transform = 'scale(1.05)';
        });
        button.addEventListener('mouseleave', function() {
            this.style.transform = 'scale(1)';
        });
    });
    
    // Add change tracking to role selects
    const roleSelects = document.querySelectorAll('.role-select');
    roleSelects.forEach(select => {
        const originalValue = select.value;
        select.addEventListener('change', function() {
            if (this.value === 'deleted') {
                if (!confirm('Setting role to "deleted" will ban this user from making any API calls. Are you sure?')) {
                    this.value = originalValue;
                    return;
                }
            }
        });
    });
});