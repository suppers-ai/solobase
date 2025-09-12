import { api } from './api.js';
import { modal } from './modal.js';

class CollectionsManager {
    constructor() {
        this.currentCollection = null;
        this.init();
    }

    init() {
        this.setupEventListeners();
        this.loadCollections();
    }

    setupEventListeners() {
        // Create button
        const createBtn = document.getElementById('createCollectionBtn');
        if (createBtn) {
            createBtn.addEventListener('click', () => this.showCreateModal());
        }

        // Form submission
        const form = document.getElementById('collectionForm');
        if (form) {
            form.addEventListener('submit', (e) => this.handleFormSubmit(e));
        }

        // Cancel button
        const cancelBtn = document.getElementById('cancelBtn');
        if (cancelBtn) {
            cancelBtn.addEventListener('click', () => modal.close());
        }

        // Table actions
        document.addEventListener('click', (e) => {
            if (e.target.classList.contains('view-collection')) {
                const name = e.target.dataset.name;
                this.viewCollection(name);
            } else if (e.target.classList.contains('edit-collection')) {
                const name = e.target.dataset.name;
                this.showEditModal(name);
            } else if (e.target.classList.contains('delete-collection')) {
                const name = e.target.dataset.name;
                this.deleteCollection(name);
            }
        });
    }

    async loadCollections() {
        try {
            const collections = await api.collections.list();
            this.renderTable(collections);
        } catch (error) {
            console.error('Failed to load collections:', error);
        }
    }

    renderTable(collections) {
        const tbody = document.querySelector('.data-table tbody');
        if (!tbody) return;

        if (collections.length === 0) {
            tbody.innerHTML = `
                <tr>
                    <td colspan="6" style="text-align: center; padding: 2rem;">
                        No collections yet. Create your first collection to get started.
                    </td>
                </tr>
            `;
            return;
        }

        tbody.innerHTML = collections.map(col => `
            <tr>
                <td class="collection-name">${col.name}</td>
                <td>${col.type}</td>
                <td>
                    <code class="schema-preview">${this.formatSchema(col.schema)}</code>
                </td>
                <td>${col.record_count || 0}</td>
                <td>${new Date(col.created_at).toLocaleDateString()}</td>
                <td>
                    <button class="btn btn-sm view-collection" data-name="${col.name}">
                        View
                    </button>
                    <button class="btn btn-sm btn-secondary edit-collection" data-name="${col.name}">
                        Edit
                    </button>
                    <button class="btn btn-sm btn-danger delete-collection" data-name="${col.name}">
                        Delete
                    </button>
                </td>
            </tr>
        `).join('');
    }

    formatSchema(schema) {
        if (!schema || !schema.fields) return '{}';
        const fields = schema.fields.map(f => f.name).slice(0, 3);
        if (schema.fields.length > 3) {
            fields.push('...');
        }
        return `{ ${fields.join(', ')} }`;
    }

    showCreateModal() {
        this.currentCollection = null;
        document.getElementById('modalTitle').textContent = 'Create Collection';
        document.getElementById('collectionForm').reset();
        
        // Set default schema
        document.getElementById('collectionSchema').value = JSON.stringify({
            fields: [
                { name: 'name', type: 'string', required: true },
                { name: 'description', type: 'string' },
                { name: 'value', type: 'number' }
            ]
        }, null, 2);
        
        modal.open('collectionModal');
    }

    async showEditModal(name) {
        try {
            const collection = await api.collections.get(name);
            this.currentCollection = collection;
            
            document.getElementById('modalTitle').textContent = 'Edit Collection';
            document.getElementById('collectionName').value = collection.name;
            document.getElementById('collectionType').value = collection.type;
            document.getElementById('collectionSchema').value = JSON.stringify(collection.schema, null, 2);
            
            // Disable name field for editing
            document.getElementById('collectionName').disabled = true;
            
            modal.open('collectionModal');
        } catch (error) {
            console.error('Failed to load collection:', error);
            alert('Failed to load collection details');
        }
    }

    async handleFormSubmit(e) {
        e.preventDefault();
        
        const formData = {
            name: document.getElementById('collectionName').value,
            type: document.getElementById('collectionType').value,
        };

        // Parse schema
        try {
            formData.schema = JSON.parse(document.getElementById('collectionSchema').value);
        } catch (error) {
            alert('Invalid JSON schema');
            return;
        }

        try {
            if (this.currentCollection) {
                await api.collections.update(this.currentCollection.name, {
                    type: formData.type,
                    schema: formData.schema
                });
            } else {
                await api.collections.create(formData);
            }
            
            modal.close();
            this.loadCollections();
            
            // Re-enable name field
            document.getElementById('collectionName').disabled = false;
        } catch (error) {
            console.error('Failed to save collection:', error);
            alert('Failed to save collection: ' + error.message);
        }
    }

    viewCollection(name) {
        window.location.href = `/collections/${name}`;
    }

    async deleteCollection(name) {
        if (!confirm(`Are you sure you want to delete the collection "${name}"? This will delete all records in the collection.`)) {
            return;
        }

        try {
            await api.collections.delete(name);
            this.loadCollections();
        } catch (error) {
            console.error('Failed to delete collection:', error);
            alert('Failed to delete collection: ' + error.message);
        }
    }
}

// Initialize when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => new CollectionsManager());
} else {
    new CollectionsManager();
}

export default CollectionsManager;