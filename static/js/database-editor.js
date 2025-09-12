// Database Editor JavaScript

// Initialize on load
document.addEventListener('DOMContentLoaded', () => {
    // Initialize Lucide icons
    if (window.lucide) {
        lucide.createIcons();
    }
    
    // Initialize keyboard shortcuts
    initKeyboardShortcuts();
    
    // Initialize history dropdown
    updateHistoryDropdown();
});

// Table Selection from Dropdown
function selectTableFromDropdown(value) {
    if (!value) return;
    
    const [schema, table] = value.split('.');
    if (schema && table) {
        selectTable(schema, table);
    }
}

// Filter tables in dropdown
function filterTablesInDropdown(searchTerm) {
    const select = document.querySelector('.table-select');
    if (!select) return;
    
    searchTerm = searchTerm.toLowerCase();
    const options = select.querySelectorAll('option');
    
    options.forEach(option => {
        if (option.value === '') return; // Skip the placeholder
        
        const text = option.textContent.toLowerCase();
        if (text.includes(searchTerm)) {
            option.style.display = '';
        } else {
            option.style.display = 'none';
        }
    });
    
    // Auto-expand schemas with matching tables
    schemas.forEach(schema => {
        const visibleTables = schema.querySelectorAll('.db-table-item[style="display: flex;"]');
        if (visibleTables.length > 0) {
            schema.classList.add('expanded');
        }
    });
}

function selectTable(schema, table) {
    // Remove active class from all tables
    document.querySelectorAll('.db-table-item').forEach(item => {
        item.classList.remove('active');
    });
    
    // Add active class to selected table
    const selectedTable = document.querySelector(`[data-schema="${schema}"][data-table="${table}"]`);
    if (selectedTable) {
        selectedTable.classList.add('active');
    }
    
    // Load table data via HTMX or fetch
    loadTableData(schema, table);
}

function loadTableData(schema, table) {
    // Use HTMX to load table data if available, otherwise fallback to full page reload
    if (window.htmx) {
        // Make an HTMX request to load just the content
        htmx.ajax('GET', `/database?schema=${schema}&table=${table}`, {
            target: '#main-content',
            swap: 'outerHTML'
        });
        
        // Update the URL without reloading
        window.history.pushState({}, '', `/database?schema=${schema}&table=${table}`);
    } else {
        // Fallback to full page reload if HTMX is not available
        window.location.href = `/database?schema=${schema}&table=${table}`;
    }
}

// Tab Switching
function switchTab(tab) {
    // Update tab buttons
    document.querySelectorAll('.db-tab').forEach(t => {
        t.classList.remove('active');
    });
    event.target.closest('.db-tab').classList.add('active');
    
    // Show/hide content
    if (tab === 'table') {
        document.getElementById('tableView').style.display = 'flex';
        document.getElementById('sqlView').style.display = 'none';
    } else {
        document.getElementById('tableView').style.display = 'none';
        document.getElementById('sqlView').style.display = 'flex';
    }
}

// Table Actions
function refreshTable() {
    location.reload();
}

function showFilterPanel() {
    document.getElementById('filterPanel').classList.add('open');
}

function closeFilterPanel() {
    document.getElementById('filterPanel').classList.remove('open');
}

function showSortPanel() {
    // Similar to filter panel
    console.log('Show sort panel');
}

// Insert Modal Functions
function showInsertRow() {
    const modal = document.getElementById('insertModal');
    if (modal) {
        modal.style.display = 'flex';
        // Re-initialize Lucide icons
        if (window.lucide) {
            lucide.createIcons();
        }
        
        // Focus first input
        setTimeout(() => {
            const firstInput = modal.querySelector('input:not([disabled])');
            if (firstInput) {
                firstInput.focus();
            }
        }, 100);
    }
}

function closeInsertModal() {
    const modal = document.getElementById('insertModal');
    if (modal) {
        modal.style.display = 'none';
        
        // Clear form
        const form = document.getElementById('insertForm');
        if (form) {
            form.reset();
        }
    }
}

function submitInsertForm() {
    const form = document.getElementById('insertForm');
    if (!form) return;
    
    const formData = new FormData(form);
    const data = {};
    
    // Get current table from URL
    const urlParams = new URLSearchParams(window.location.search);
    const table = urlParams.get('table');
    const schema = urlParams.get('schema') || 'public';
    
    // Collect form data
    for (let [key, value] of formData.entries()) {
        // Skip empty ID fields (auto-generated)
        if (key === 'id' && !value) continue;
        
        // Convert empty strings to null for nullable fields
        if (value === '') {
            const input = form.querySelector(`[name="${key}"]`);
            if (!input.required) {
                data[key] = null;
            }
        } else {
            data[key] = value;
        }
    }
    
    // Send data to server
    fetch('/api/database/insert', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({
            table: schema + '.' + table,
            data: data
        })
    })
    .then(response => {
        if (response.ok) {
            closeInsertModal();
            location.reload();
        } else {
            response.text().then(text => {
                alert('Error inserting row: ' + text);
            });
        }
    })
    .catch(error => {
        alert('Error inserting row: ' + error.message);
    });
}

// Handle Enter key in form
document.addEventListener('DOMContentLoaded', () => {
    const form = document.getElementById('insertForm');
    if (form) {
        form.addEventListener('keypress', (e) => {
            if (e.key === 'Enter' && e.target.tagName !== 'TEXTAREA') {
                e.preventDefault();
                submitInsertForm();
            }
        });
    }
});

// Cell Editing
function editCell(cell) {
    // Check if already editing
    if (cell.querySelector('input')) return;
    
    const originalValue = cell.textContent;
    const column = cell.dataset.column;
    
    // Create input
    const input = document.createElement('input');
    input.type = 'text';
    input.className = 'cell-input';
    input.value = originalValue === 'NULL' ? '' : originalValue;
    
    // Replace cell content with input
    cell.innerHTML = '';
    cell.appendChild(input);
    input.focus();
    input.select();
    
    // Handle save/cancel
    input.addEventListener('blur', () => {
        saveCell(cell, input, originalValue, column);
    });
    
    input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            saveCell(cell, input, originalValue, column);
        } else if (e.key === 'Escape') {
            cell.textContent = originalValue;
        }
    });
}

function saveCell(cell, input, originalValue, column) {
    const newValue = input.value;
    
    if (newValue !== originalValue) {
        // Save to server
        const rowId = cell.closest('tr').dataset.rowId;
        const table = cell.closest('table').dataset.table || getCurrentTable();
        
        fetch('/api/database/update', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                table: table,
                id: rowId,
                column: column,
                value: newValue || null
            })
        }).then(response => {
            if (response.ok) {
                cell.textContent = newValue || 'NULL';
            } else {
                cell.textContent = originalValue;
            }
        });
    } else {
        cell.textContent = originalValue;
    }
}

// Helper function to get current table from URL
function getCurrentTable() {
    const urlParams = new URLSearchParams(window.location.search);
    const table = urlParams.get('table');
    const schema = urlParams.get('schema') || 'public';
    return schema + '.' + table;
}

// Row Actions
function duplicateRow(button) {
    const row = button.closest('tr');
    const cells = row.querySelectorAll('.grid-cell');
    const data = {};
    
    cells.forEach(cell => {
        const column = cell.dataset.column;
        if (column) {
            data[column] = cell.textContent === 'NULL' ? null : cell.textContent;
        }
    });
    
    // Send to server
    fetch('/api/database/duplicate', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify(data)
    }).then(response => {
        if (response.ok) {
            location.reload();
        }
    });
}

function deleteRow(button) {
    if (!confirm('Are you sure you want to delete this row?')) return;
    
    const row = button.closest('tr');
    const rowId = row.dataset.rowId;
    
    fetch(`/api/database/delete/${rowId}`, {
        method: 'DELETE'
    }).then(response => {
        if (response.ok) {
            row.remove();
        }
    });
}

// Checkbox handling
function toggleAllRows(checkbox) {
    const checkboxes = document.querySelectorAll('input[name="row-select"]');
    checkboxes.forEach(cb => {
        cb.checked = checkbox.checked;
    });
}

// Pagination
function previousPage() {
    const urlParams = new URLSearchParams(window.location.search);
    const page = parseInt(urlParams.get('page') || '1');
    if (page > 1) {
        urlParams.set('page', page - 1);
        window.location.search = urlParams.toString();
    }
}

function nextPage() {
    const urlParams = new URLSearchParams(window.location.search);
    const page = parseInt(urlParams.get('page') || '1');
    urlParams.set('page', page + 1);
    window.location.search = urlParams.toString();
}

function changePageSize(size) {
    const urlParams = new URLSearchParams(window.location.search);
    urlParams.set('pageSize', size);
    urlParams.set('page', '1');
    window.location.search = urlParams.toString();
}

// SQL Editor Functions
function executeQuery() {
    const query = document.getElementById('sqlEditor').value;
    if (!query) return;
    
    const startTime = Date.now();
    
    fetch('/api/database/query', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({ query: query })
    })
    .then(response => response.json())
    .then(data => {
        const endTime = Date.now();
        const duration = endTime - startTime;
        
        // Display execution time
        document.getElementById('queryTime').textContent = `${duration}ms`;
        
        // Check if there's an error in the response
        if (data.error) {
            displayQueryError(data.error);
        } else {
            // Display results
            displayQueryResults(data);
            
            // Add to history
            addToHistory(query);
        }
    })
    .catch(error => {
        displayQueryError(error.message || 'An error occurred while executing the query');
    });
}

function displayQueryResults(data) {
    const container = document.getElementById('sqlResultsContainer');
    
    if (!data || data.length === 0) {
        container.innerHTML = '<p style="color: #999;">No results returned</p>';
        return;
    }
    
    // Create table
    const table = document.createElement('table');
    table.className = 'data-grid';
    
    // Create header
    const thead = document.createElement('thead');
    const headerRow = document.createElement('tr');
    Object.keys(data[0]).forEach(key => {
        const th = document.createElement('th');
        th.textContent = key;
        headerRow.appendChild(th);
    });
    thead.appendChild(headerRow);
    table.appendChild(thead);
    
    // Create body
    const tbody = document.createElement('tbody');
    data.forEach(row => {
        const tr = document.createElement('tr');
        tr.className = 'grid-row';
        Object.values(row).forEach(value => {
            const td = document.createElement('td');
            td.className = 'grid-cell';
            td.textContent = value === null ? 'NULL' : value;
            tr.appendChild(td);
        });
        tbody.appendChild(tr);
    });
    table.appendChild(tbody);
    
    container.innerHTML = '';
    container.appendChild(table);
}

function displayQueryError(error) {
    const container = document.getElementById('sqlResultsContainer');
    container.innerHTML = `<div style="color: #e74c3c; padding: 16px; background: rgba(231,76,60,0.1); border: 1px solid rgba(231,76,60,0.2); border-radius: 6px;">
        <strong>Error:</strong> ${error}
    </div>`;
}

function formatSQL() {
    const editor = document.getElementById('sqlEditor');
    // Basic SQL formatting (you could use a library for better formatting)
    let sql = editor.value;
    sql = sql.replace(/\s+/g, ' ');
    sql = sql.replace(/,/g, ',\n  ');
    sql = sql.replace(/FROM/gi, '\nFROM');
    sql = sql.replace(/WHERE/gi, '\nWHERE');
    sql = sql.replace(/JOIN/gi, '\nJOIN');
    sql = sql.replace(/ORDER BY/gi, '\nORDER BY');
    sql = sql.replace(/GROUP BY/gi, '\nGROUP BY');
    editor.value = sql;
}

function clearSQL() {
    document.getElementById('sqlEditor').value = '';
    document.getElementById('sqlResultsContainer').innerHTML = '';
    document.getElementById('queryTime').textContent = '';
}

function addToHistory(query) {
    // Store in localStorage
    let history = JSON.parse(localStorage.getItem('sqlHistory') || '[]');
    history.unshift({ query: query, timestamp: Date.now() });
    history = history.slice(0, 20); // Keep only last 20 queries
    localStorage.setItem('sqlHistory', JSON.stringify(history));
    
    // Update history dropdown
    updateHistoryDropdown();
}

function updateHistoryDropdown() {
    const select = document.querySelector('.history-select');
    const history = JSON.parse(localStorage.getItem('sqlHistory') || '[]');
    
    select.innerHTML = '<option value="">Query history...</option>';
    history.forEach((item, index) => {
        const option = document.createElement('option');
        option.value = index;
        option.textContent = item.query.substring(0, 50) + (item.query.length > 50 ? '...' : '');
        select.appendChild(option);
    });
}

function loadHistory(index) {
    if (index === '') return;
    
    const history = JSON.parse(localStorage.getItem('sqlHistory') || '[]');
    const item = history[parseInt(index)];
    if (item) {
        document.getElementById('sqlEditor').value = item.query;
    }
}

// Create Table Modal Functions
function showNewTableModal() {
    const modal = document.getElementById('createTableModal');
    if (modal) {
        modal.style.display = 'flex';
        // Initialize with default columns
        resetCreateTableForm();
        // Re-initialize Lucide icons
        if (window.lucide) {
            lucide.createIcons();
        }
    }
}

function closeCreateTableModal() {
    const modal = document.getElementById('createTableModal');
    if (modal) {
        modal.style.display = 'none';
    }
}

function resetCreateTableForm() {
    const form = document.getElementById('createTableForm');
    if (form) {
        form.reset();
        // Reset columns container to default
        const container = document.getElementById('columnsContainer');
        container.innerHTML = `
            <div class="column-row">
                <input type="text" value="id" disabled class="form-input column-name">
                <select disabled class="form-input column-type">
                    <option>SERIAL PRIMARY KEY</option>
                </select>
                <div class="column-options">
                    <label class="checkbox-label">
                        <input type="checkbox" checked disabled>
                        Primary
                    </label>
                </div>
            </div>
            <div class="column-row">
                <input type="text" value="created_at" disabled class="form-input column-name">
                <select disabled class="form-input column-type">
                    <option>TIMESTAMP DEFAULT NOW()</option>
                </select>
                <div class="column-options">
                    <label class="checkbox-label">
                        <input type="checkbox" disabled>
                        Nullable
                    </label>
                </div>
            </div>
        `;
    }
}

// Use window.columnCounter to avoid redeclaration errors
if (typeof window.columnCounter === 'undefined') {
    window.columnCounter = 0;
}

function addColumn() {
    const container = document.getElementById('columnsContainer');
    const columnId = `column_${++window.columnCounter}`;
    
    const columnRow = document.createElement('div');
    columnRow.className = 'column-row';
    columnRow.id = columnId;
    columnRow.innerHTML = `
        <input type="text" placeholder="Column name" class="form-input column-name" required>
        <select class="form-input column-type">
            <option value="TEXT">TEXT</option>
            <option value="INTEGER">INTEGER</option>
            <option value="BIGINT">BIGINT</option>
            <option value="DECIMAL">DECIMAL</option>
            <option value="BOOLEAN">BOOLEAN</option>
            <option value="DATE">DATE</option>
            <option value="TIMESTAMP">TIMESTAMP</option>
            <option value="UUID">UUID</option>
            <option value="JSON">JSON</option>
            <option value="JSONB">JSONB</option>
        </select>
        <div class="column-options">
            <label class="checkbox-label">
                <input type="checkbox" class="nullable-check">
                Nullable
            </label>
            <label class="checkbox-label">
                <input type="checkbox" class="unique-check">
                Unique
            </label>
        </div>
        <button type="button" class="btn-icon-sm" onclick="removeColumn('${columnId}')">
            <i data-lucide="trash-2"></i>
        </button>
    `;
    
    container.appendChild(columnRow);
    
    // Re-initialize Lucide icons
    if (window.lucide) {
        lucide.createIcons();
    }
}

function removeColumn(columnId) {
    const column = document.getElementById(columnId);
    if (column) {
        column.remove();
    }
}

function submitCreateTable() {
    const form = document.getElementById('createTableForm');
    if (!form.checkValidity()) {
        form.reportValidity();
        return;
    }
    
    const tableName = document.getElementById('tableName').value;
    const schema = document.getElementById('tableSchema').value;
    
    // Collect columns
    const columns = [];
    
    // Add default columns
    columns.push({
        name: 'id',
        type: 'SERIAL',
        primary: true,
        nullable: false,
        unique: true
    });
    
    columns.push({
        name: 'created_at',
        type: 'TIMESTAMP',
        default: 'NOW()',
        nullable: false
    });
    
    // Add custom columns
    const columnRows = document.querySelectorAll('#columnsContainer .column-row');
    columnRows.forEach((row, index) => {
        // Skip the first two default columns
        if (index < 2) return;
        
        const name = row.querySelector('.column-name').value;
        const type = row.querySelector('.column-type').value;
        const nullable = row.querySelector('.nullable-check')?.checked || false;
        const unique = row.querySelector('.unique-check')?.checked || false;
        
        if (name) {
            columns.push({
                name: name,
                type: type,
                nullable: nullable,
                unique: unique
            });
        }
    });
    
    // Send to server
    fetch('/api/database/create-table', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({
            schema: schema,
            table: tableName,
            columns: columns
        })
    })
    .then(response => {
        if (response.ok) {
            closeCreateTableModal();
            // Redirect to the new table
            window.location.href = `/database?schema=${schema}&table=${tableName}`;
        } else {
            return response.text().then(text => {
                alert('Error creating table: ' + text);
            });
        }
    })
    .catch(error => {
        alert('Error creating table: ' + error.message);
    });
}

// Keyboard Shortcuts
function initKeyboardShortcuts() {
    document.addEventListener('keydown', (e) => {
        // Cmd/Ctrl + Enter to execute query
        if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
            const sqlEditor = document.getElementById('sqlEditor');
            if (sqlEditor && document.activeElement === sqlEditor) {
                e.preventDefault();
                executeQuery();
            }
        }
        
        // Escape to close panels
        if (e.key === 'Escape') {
            document.querySelectorAll('.side-panel.open').forEach(panel => {
                panel.classList.remove('open');
            });
        }
    });
}

