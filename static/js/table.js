// Table helper functions

// Copy text to clipboard
async function copyToClipboard(text) {
    try {
        await navigator.clipboard.writeText(text);
        
        // Show toast notification
        const toast = document.createElement('div');
        toast.className = 'toast';
        toast.textContent = 'Copied to clipboard!';
        toast.style.cssText = `
            position: fixed;
            bottom: 20px;
            right: 20px;
            background: #333;
            color: white;
            padding: 10px 20px;
            border-radius: 4px;
            font-size: 14px;
            z-index: 10000;
            animation: fadeInOut 2s ease-in-out;
        `;
        
        // Add animation
        const style = document.createElement('style');
        style.textContent = `
            @keyframes fadeInOut {
                0% { opacity: 0; transform: translateY(10px); }
                20% { opacity: 1; transform: translateY(0); }
                80% { opacity: 1; transform: translateY(0); }
                100% { opacity: 0; transform: translateY(-10px); }
            }
        `;
        document.head.appendChild(style);
        
        document.body.appendChild(toast);
        setTimeout(() => toast.remove(), 2000);
    } catch (err) {
        console.error('Failed to copy:', err);
    }
}

// Toggle all checkboxes
function toggleSelectAll(checkbox) {
    const checkboxes = document.querySelectorAll('input[name="selected[]"]');
    checkboxes.forEach(cb => {
        cb.checked = checkbox.checked;
    });
}

// Sort table by column
let sortState = {};

function sortTable(columnKey) {
    const currentSort = sortState[columnKey] || 'none';
    const newSort = currentSort === 'none' ? 'asc' : 
                   currentSort === 'asc' ? 'desc' : 'none';
    
    // Reset all other columns
    Object.keys(sortState).forEach(key => {
        if (key !== columnKey) sortState[key] = 'none';
    });
    
    sortState[columnKey] = newSort;
    
    // Update URL with sort parameters
    const url = new URL(window.location);
    if (newSort === 'none') {
        url.searchParams.delete('sort');
        url.searchParams.delete('order');
    } else {
        url.searchParams.set('sort', columnKey);
        url.searchParams.set('order', newSort);
    }
    
    window.location.href = url.toString();
}

// Navigate to page
function goToPage(baseURL, pageNumber) {
    const url = new URL(window.location.origin + baseURL);
    const currentParams = new URLSearchParams(window.location.search);
    
    // Preserve existing parameters
    currentParams.forEach((value, key) => {
        if (key !== 'page') {
            url.searchParams.set(key, value);
        }
    });
    
    // Set new page
    if (pageNumber > 1) {
        url.searchParams.set('page', pageNumber);
    }
    
    window.location.href = url.toString();
}

// Filter table
function filterTable(searchTerm) {
    const rows = document.querySelectorAll('.data-table tbody tr');
    const term = searchTerm.toLowerCase();
    
    rows.forEach(row => {
        const text = row.textContent.toLowerCase();
        row.style.display = text.includes(term) ? '' : 'none';
    });
    
    // Update results count
    const visibleRows = document.querySelectorAll('.data-table tbody tr:not([style*="display: none"])');
    const info = document.querySelector('.table-info');
    if (info) {
        info.textContent = `Showing ${visibleRows.length} results`;
    }
}

// Initialize table search
document.addEventListener('DOMContentLoaded', () => {
    const searchInput = document.querySelector('.search-input');
    if (searchInput) {
        searchInput.addEventListener('input', (e) => {
            filterTable(e.target.value);
        });
    }
});

// Export functions for use in other scripts
window.copyToClipboard = copyToClipboard;
window.toggleSelectAll = toggleSelectAll;
window.sortTable = sortTable;
window.goToPage = goToPage;
window.filterTable = filterTable;