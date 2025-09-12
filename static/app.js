// Solobase Dashboard JavaScript

// Create Collection Modal
function createCollection() {
    const modal = document.createElement('div');
    modal.className = 'modal';
    modal.innerHTML = `
        <div class="modal-content">
            <h2>Create New Collection</h2>
            <form id="createCollectionForm">
                <div class="form-group">
                    <label>Collection Name</label>
                    <input type="text" name="name" pattern="[a-z][a-z0-9_]*" required>
                    <small>Lowercase letters, numbers, and underscores only</small>
                </div>
                <div class="form-group">
                    <label>Display Name</label>
                    <input type="text" name="display_name" required>
                </div>
                <div class="form-group">
                    <label>Description</label>
                    <textarea name="description" rows="3"></textarea>
                </div>
                
                <h3>Fields</h3>
                <div id="fields">
                    <div class="field-row">
                        <input type="text" placeholder="Field name" name="field_name[]">
                        <select name="field_type[]">
                            <option value="text">Text</option>
                            <option value="number">Number</option>
                            <option value="boolean">Boolean</option>
                            <option value="date">Date</option>
                            <option value="json">JSON</option>
                            <option value="uuid">UUID</option>
                        </select>
                        <label><input type="checkbox" name="field_required[]"> Required</label>
                        <label><input type="checkbox" name="field_unique[]"> Unique</label>
                    </div>
                </div>
                <button type="button" onclick="addField()">+ Add Field</button>
                
                <div class="buttons">
                    <button type="submit">Create</button>
                    <button type="button" onclick="closeModal()">Cancel</button>
                </div>
            </form>
        </div>
    `;
    
    document.body.appendChild(modal);
    
    document.getElementById('createCollectionForm').onsubmit = async (e) => {
        e.preventDefault();
        const formData = new FormData(e.target);
        
        // Build collection object
        const collection = {
            name: formData.get('name'),
            display_name: formData.get('display_name'),
            description: formData.get('description'),
            schema: {
                fields: []
            },
            indexes: [],
            auth_rules: {
                list_rule: '@authenticated',
                view_rule: '@authenticated',
                create_rule: '@authenticated',
                update_rule: '@authenticated',
                delete_rule: '@admin'
            }
        };
        
        // Add fields
        const fieldNames = formData.getAll('field_name[]');
        const fieldTypes = formData.getAll('field_type[]');
        const fieldRequired = formData.getAll('field_required[]');
        const fieldUnique = formData.getAll('field_unique[]');
        
        fieldNames.forEach((name, i) => {
            if (name) {
                collection.schema.fields.push({
                    name: name,
                    type: fieldTypes[i],
                    required: fieldRequired.includes(i.toString()),
                    unique: fieldUnique.includes(i.toString())
                });
            }
        });
        
        try {
            const response = await fetch('/api/v1/collections', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(collection)
            });
            
            if (response.ok) {
                closeModal();
                location.reload();
            } else {
                const error = await response.text();
                alert('Error creating collection: ' + error);
            }
        } catch (err) {
            alert('Error: ' + err.message);
        }
    };
}

function addField() {
    const fieldsDiv = document.getElementById('fields');
    const fieldRow = document.createElement('div');
    fieldRow.className = 'field-row';
    fieldRow.innerHTML = `
        <input type="text" placeholder="Field name" name="field_name[]">
        <select name="field_type[]">
            <option value="text">Text</option>
            <option value="number">Number</option>
            <option value="boolean">Boolean</option>
            <option value="date">Date</option>
            <option value="json">JSON</option>
            <option value="uuid">UUID</option>
        </select>
        <label><input type="checkbox" name="field_required[]"> Required</label>
        <label><input type="checkbox" name="field_unique[]"> Unique</label>
        <button type="button" onclick="removeField(this)">Remove</button>
    `;
    fieldsDiv.appendChild(fieldRow);
}

function removeField(button) {
    button.parentElement.remove();
}

function closeModal() {
    const modal = document.querySelector('.modal');
    if (modal) {
        modal.remove();
    }
}

function viewCollection(name) {
    window.location.href = `/dashboard/database/collection/${name}`;
}

function editCollection(name) {
    // TODO: Implement edit collection
    alert('Edit collection: ' + name);
}

// Add modal styles
const style = document.createElement('style');
style.textContent = `
.modal {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0,0,0,0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
}

.modal-content {
    background: white;
    padding: 2rem;
    border-radius: 8px;
    width: 90%;
    max-width: 600px;
    max-height: 80vh;
    overflow-y: auto;
}

.modal-content h2 {
    margin-top: 0;
}

.form-group {
    margin-bottom: 1rem;
}

.form-group label {
    display: block;
    margin-bottom: 0.25rem;
    font-weight: 500;
}

.form-group input[type="text"],
.form-group input[type="email"],
.form-group textarea,
.form-group select {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid #ddd;
    border-radius: 4px;
}

.form-group small {
    display: block;
    margin-top: 0.25rem;
    color: #666;
    font-size: 0.875rem;
}

.field-row {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
    align-items: center;
}

.field-row input[type="text"] {
    flex: 1;
}

.field-row select {
    width: 120px;
}

.field-row label {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    white-space: nowrap;
}

.buttons {
    display: flex;
    gap: 1rem;
    margin-top: 2rem;
}

.buttons button {
    padding: 0.75rem 1.5rem;
}
`;
document.head.appendChild(style);