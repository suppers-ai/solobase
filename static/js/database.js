// Database page functionality

// Select table
window.selectTable = function(tableName) {
    if (tableName) {
        window.location.href = `/database?table=${tableName}`;
    } else {
        window.location.href = '/database';
    }
}

// Show create collection modal
window.showCreateCollectionModal = function() {
    document.getElementById('createCollectionModal').style.display = 'block';
}

// Hide create collection modal
window.hideCreateCollectionModal = function() {
    document.getElementById('createCollectionModal').style.display = 'none';
    document.getElementById('createCollectionForm').reset();
}

// Create collection
window.createCollection = async function(event) {
    event.preventDefault();
    
    const form = event.target;
    const formData = new FormData(form);
    
    let schema = {};
    const schemaText = formData.get('schema');
    if (schemaText) {
        try {
            schema = JSON.parse(schemaText);
        } catch (e) {
            alert('Invalid JSON schema');
            return;
        }
    }
    
    try {
        const response = await fetch('/api/v1/collections', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                name: formData.get('name'),
                schema: schema
            })
        });
        
        if (response.ok) {
            window.location.reload();
        } else {
            const error = await response.json();
            alert('Error creating collection: ' + error.message);
        }
    } catch (error) {
        alert('Error creating collection: ' + error.message);
    }
}

// Show add record modal
window.showAddRecordModal = function() {
    const table = new URLSearchParams(window.location.search).get('table');
    if (!table) return;
    
    // TODO: Dynamically generate form fields based on table schema
    document.getElementById('recordFields').innerHTML = `
        <div class="form-group">
            <label>Record Data (JSON)</label>
            <textarea name="data" rows="10" required>{}</textarea>
        </div>
    `;
    
    document.getElementById('addRecordModal').style.display = 'block';
}

// Hide add record modal
window.hideAddRecordModal = function() {
    document.getElementById('addRecordModal').style.display = 'none';
    document.getElementById('addRecordForm').reset();
}

// Add record
window.addRecord = async function(event) {
    event.preventDefault();
    
    const table = new URLSearchParams(window.location.search).get('table');
    if (!table) return;
    
    const form = event.target;
    const formData = new FormData(form);
    
    let data = {};
    try {
        data = JSON.parse(formData.get('data'));
    } catch (e) {
        alert('Invalid JSON data');
        return;
    }
    
    try {
        const response = await fetch(`/api/v1/collections/${table}/records`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(data)
        });
        
        if (response.ok) {
            window.location.reload();
        } else {
            const error = await response.json();
            alert('Error adding record: ' + error.message);
        }
    } catch (error) {
        alert('Error adding record: ' + error.message);
    }
}

// Export data as CSV
window.exportData = async function() {
    const table = new URLSearchParams(window.location.search).get('table');
    if (!table) return;
    
    try {
        const response = await fetch(`/api/v1/collections/${table}/export`);
        if (response.ok) {
            const blob = await response.blob();
            const url = window.URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `${table}_export.csv`;
            a.click();
            window.URL.revokeObjectURL(url);
        } else {
            alert('Error exporting data');
        }
    } catch (error) {
        alert('Error exporting data: ' + error.message);
    }
}

// Delete record
window.deleteRecord = async function(table, recordId) {
    if (!confirm('Are you sure you want to delete this record?')) {
        return;
    }
    
    try {
        const response = await fetch(`/api/v1/collections/${table}/records/${recordId}`, {
            method: 'DELETE'
        });
        
        if (response.ok) {
            window.location.reload();
        } else {
            const error = await response.json();
            alert('Error deleting record: ' + error.message);
        }
    } catch (error) {
        alert('Error deleting record: ' + error.message);
    }
}

// Close modals on escape key
document.addEventListener('keydown', function(event) {
    if (event.key === 'Escape') {
        hideCreateCollectionModal();
        hideAddRecordModal();
    }
});

// Close modals on outside click
document.getElementById('createCollectionModal')?.addEventListener('click', function(event) {
    if (event.target === this) {
        hideCreateCollectionModal();
    }
});

document.getElementById('addRecordModal')?.addEventListener('click', function(event) {
    if (event.target === this) {
        hideAddRecordModal();
    }
});