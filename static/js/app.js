// Main application entry point

(function() {
    'use strict';

    // Get current page from URL path
    const path = window.location.pathname;

    // Global functionality

    // Handle logout
    const logoutLinks = document.querySelectorAll('a[href="/auth/logout"]');
    logoutLinks.forEach(link => {
        link.addEventListener('click', (e) => {
            e.preventDefault();
            if (confirm('Are you sure you want to logout?')) {
                window.location.href = '/auth/logout';
            }
        });
    });

    // Flash messages
    const flashMessages = document.querySelectorAll('.flash-message');
    flashMessages.forEach(msg => {
        setTimeout(() => {
            msg.style.transition = 'opacity 0.5s';
            msg.style.opacity = '0';
            setTimeout(() => msg.remove(), 500);
        }, 5000);
    });

    // Form validation
    const forms = document.querySelectorAll('form[data-validate]');
    forms.forEach(form => {
        form.addEventListener('submit', (e) => {
            const requiredFields = form.querySelectorAll('[required]');
            let isValid = true;
            
            requiredFields.forEach(field => {
                if (!field.value.trim()) {
                    field.classList.add('error');
                    isValid = false;
                } else {
                    field.classList.remove('error');
                }
            });
            
            if (!isValid) {
                e.preventDefault();
                alert('Please fill in all required fields');
            }
        });
    });

    // Table sorting
    const sortableHeaders = document.querySelectorAll('th[data-sortable]');
    sortableHeaders.forEach(header => {
        header.style.cursor = 'pointer';
        header.addEventListener('click', () => {
            const table = header.closest('table');
            const tbody = table.querySelector('tbody');
            const rows = Array.from(tbody.querySelectorAll('tr'));
            const columnIndex = Array.from(header.parentElement.children).indexOf(header);
            const isAscending = header.classList.contains('sort-asc');
            
            rows.sort((a, b) => {
                const aText = a.children[columnIndex].textContent.trim();
                const bText = b.children[columnIndex].textContent.trim();
                
                // Try to parse as number
                const aNum = parseFloat(aText);
                const bNum = parseFloat(bText);
                
                if (!isNaN(aNum) && !isNaN(bNum)) {
                    return isAscending ? bNum - aNum : aNum - bNum;
                }
                
                // Sort as string
                return isAscending 
                    ? bText.localeCompare(aText)
                    : aText.localeCompare(bText);
            });
            
            // Update classes
            sortableHeaders.forEach(h => {
                h.classList.remove('sort-asc', 'sort-desc');
            });
            header.classList.add(isAscending ? 'sort-desc' : 'sort-asc');
            
            // Re-append sorted rows
            rows.forEach(row => tbody.appendChild(row));
        });
    });

    // Loading overlay for HTMX requests
    const loadingOverlay = document.getElementById('loading-overlay');
    
    if (loadingOverlay) {
        // Show loader when HTMX request starts
        document.body.addEventListener('htmx:beforeRequest', (event) => {
            // Only show for navigation requests (clicking menu items)
            if (event.detail.elt.matches('.sidebar-nav-link, .breadcrumb-item, a[hx-get]')) {
                loadingOverlay.style.display = 'flex';
            }
        });
        
        // Hide loader when HTMX request completes
        document.body.addEventListener('htmx:afterRequest', () => {
            loadingOverlay.style.display = 'none';
        });
        
        // Also hide on swap completion for safety
        document.body.addEventListener('htmx:afterSwap', (event) => {
            loadingOverlay.style.display = 'none';
            
            // Cleanup dashboard if navigating away from it
            const currentPath = window.location.pathname;
            if (currentPath === '/dashboard' && window.cleanupDashboard) {
                // Check if we're still on dashboard after swap
                setTimeout(() => {
                    if (window.location.pathname !== '/dashboard') {
                        window.cleanupDashboard();
                    }
                }, 100);
            }
            
            // Initialize dashboard if navigating to it
            if (currentPath === '/dashboard' && window.initializeDashboard) {
                // Small delay to ensure DOM is ready
                setTimeout(() => {
                    if (window.location.pathname === '/dashboard') {
                        window.initializeDashboard();
                    }
                }, 100);
            }
        });
        
        // Hide loader on errors too
        document.body.addEventListener('htmx:responseError', () => {
            loadingOverlay.style.display = 'none';
        });
        
        document.body.addEventListener('htmx:timeout', () => {
            loadingOverlay.style.display = 'none';
        });
    }
    
    // Cleanup on page unload
    window.addEventListener('beforeunload', () => {
        if (window.cleanupDashboard) {
            window.cleanupDashboard();
        }
    });

})();