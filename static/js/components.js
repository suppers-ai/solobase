// Component Event Handler - Wires up data-* event attributes to actual event handlers
document.addEventListener('DOMContentLoaded', function() {
    
    // Wire up data-onclick attributes
    function setupClickHandlers() {
        const elements = document.querySelectorAll('[data-onclick]');
        elements.forEach(element => {
            const handler = element.getAttribute('data-onclick');
            if (handler) {
                element.onclick = function(event) {
                    try {
                        // Create a function that executes the handler code
                        const func = new Function('event', handler);
                        func.call(this, event);
                    } catch (error) {
                        console.error('Error executing onclick handler:', error, handler);
                    }
                };
            }
        });
    }
    
    // Wire up data-onchange attributes
    function setupChangeHandlers() {
        const elements = document.querySelectorAll('[data-onchange]');
        elements.forEach(element => {
            const handler = element.getAttribute('data-onchange');
            if (handler) {
                element.onchange = function(event) {
                    try {
                        // Create a function that executes the handler code
                        const func = new Function('event', handler);
                        func.call(this, event);
                    } catch (error) {
                        console.error('Error executing onchange handler:', error, handler);
                    }
                };
            }
        });
    }
    
    // Wire up data-oninput attributes
    function setupInputHandlers() {
        const elements = document.querySelectorAll('[data-oninput]');
        elements.forEach(element => {
            const handler = element.getAttribute('data-oninput');
            if (handler) {
                element.oninput = function(event) {
                    try {
                        // Create a function that executes the handler code
                        const func = new Function('event', handler);
                        func.call(this, event);
                    } catch (error) {
                        console.error('Error executing oninput handler:', error, handler);
                    }
                };
            }
        });
    }
    
    // Initialize all event handlers
    function initializeEventHandlers() {
        setupClickHandlers();
        setupChangeHandlers();
        setupInputHandlers();
    }
    
    // Run initialization
    initializeEventHandlers();
    
    // Re-initialize when content is dynamically loaded (e.g., via HTMX)
    document.addEventListener('htmx:afterSwap', initializeEventHandlers);
    document.addEventListener('htmx:afterSettle', initializeEventHandlers);
    
    // Also observe for DOM changes (for other dynamic content)
    const observer = new MutationObserver(function(mutations) {
        // Debounce to avoid excessive re-initialization
        clearTimeout(window.componentHandlerTimeout);
        window.componentHandlerTimeout = setTimeout(initializeEventHandlers, 100);
    });
    
    observer.observe(document.body, {
        childList: true,
        subtree: true
    });
});

// Helper functions for common UI operations

// Toggle dropdown menu
function toggleDropdown(button) {
    const dropdown = button.parentElement;
    const menu = dropdown.querySelector('.dropdown-menu');
    if (menu) {
        menu.classList.toggle('show');
        
        // Close when clicking outside
        document.addEventListener('click', function closeDropdown(e) {
            if (!dropdown.contains(e.target)) {
                menu.classList.remove('show');
                document.removeEventListener('click', closeDropdown);
            }
        });
    }
}

// Close modal
function closeModal(modalId) {
    const modal = document.getElementById(modalId);
    if (modal) {
        modal.classList.remove('active');
    }
}

// Open modal
function openModal(modalId) {
    const modal = document.getElementById(modalId);
    if (modal) {
        modal.classList.add('active');
    }
}

// Dismiss toast
function dismissToast(toastElement) {
    if (toastElement) {
        toastElement.style.animation = 'fadeOut 0.3s ease';
        setTimeout(() => toastElement.remove(), 300);
    }
}

// Toggle select all checkboxes
function toggleSelectAll(checkbox) {
    const checkboxes = document.querySelectorAll('input[type="checkbox"][name="user-select"]');
    checkboxes.forEach(cb => cb.checked = checkbox.checked);
}