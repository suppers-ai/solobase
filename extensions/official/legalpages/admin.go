package legalpages

const adminTemplate = `
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Legal Pages Management</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            background: #f5f5f5;
            color: #333;
        }

        .container {
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
        }

        .header {
            background: white;
            padding: 20px;
            border-radius: 8px;
            margin-bottom: 20px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }

        .header h1 {
            font-size: 24px;
            color: #2c3e50;
        }

        .tabs {
            display: flex;
            gap: 10px;
            margin-bottom: 20px;
            background: white;
            padding: 10px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }

        .tab {
            padding: 10px 20px;
            background: #f0f0f0;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-size: 14px;
            transition: background 0.3s;
        }

        .tab:hover {
            background: #e0e0e0;
        }

        .tab.active {
            background: #3498db;
            color: white;
        }

        .content-section {
            display: none;
            background: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }

        .content-section.active {
            display: block;
        }

        .form-group {
            margin-bottom: 20px;
        }

        .form-group label {
            display: block;
            margin-bottom: 5px;
            font-weight: 600;
            color: #555;
        }

        .form-group input[type="text"] {
            width: 100%;
            padding: 10px;
            border: 1px solid #ddd;
            border-radius: 4px;
            font-size: 14px;
        }

        .editor-container {
            border: 1px solid #ddd;
            border-radius: 4px;
            overflow: hidden;
        }

        .editor-toolbar {
            background: #f8f9fa;
            padding: 10px;
            border-bottom: 1px solid #ddd;
            display: flex;
            gap: 5px;
            flex-wrap: wrap;
        }

        .editor-btn {
            padding: 5px 10px;
            background: white;
            border: 1px solid #ddd;
            border-radius: 3px;
            cursor: pointer;
            font-size: 14px;
        }

        .editor-btn:hover {
            background: #f0f0f0;
        }

        .editor-content {
            min-height: 400px;
            padding: 15px;
            font-size: 14px;
            line-height: 1.6;
            outline: none;
        }

        .button-group {
            display: flex;
            gap: 10px;
            margin-top: 20px;
        }

        .btn {
            padding: 10px 20px;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-size: 14px;
            transition: opacity 0.3s;
        }

        .btn:hover {
            opacity: 0.9;
        }

        .btn-primary {
            background: #3498db;
            color: white;
        }

        .btn-secondary {
            background: #95a5a6;
            color: white;
        }

        .btn-success {
            background: #27ae60;
            color: white;
        }

        .alert {
            padding: 15px;
            border-radius: 4px;
            margin-bottom: 20px;
        }

        .alert-success {
            background: #d4edda;
            color: #155724;
            border: 1px solid #c3e6cb;
        }

        .alert-error {
            background: #f8d7da;
            color: #721c24;
            border: 1px solid #f5c6cb;
        }

        .alert-info {
            background: #d1ecf1;
            color: #0c5460;
            border: 1px solid #bee5eb;
        }

        .preview-modal {
            display: none;
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background: rgba(0,0,0,0.5);
            z-index: 1000;
        }

        .preview-modal.active {
            display: flex;
            align-items: center;
            justify-content: center;
        }

        .preview-content {
            background: white;
            width: 90%;
            max-width: 800px;
            max-height: 80vh;
            overflow-y: auto;
            padding: 30px;
            border-radius: 8px;
            position: relative;
        }

        .preview-close {
            position: absolute;
            top: 10px;
            right: 10px;
            padding: 5px 10px;
            background: #e74c3c;
            color: white;
            border: none;
            border-radius: 4px;
            cursor: pointer;
        }

        .version-info {
            background: #f8f9fa;
            padding: 10px;
            border-radius: 4px;
            margin-bottom: 10px;
            font-size: 12px;
            color: #666;
        }

        .loading {
            display: inline-block;
            width: 20px;
            height: 20px;
            border: 3px solid rgba(0,0,0,.1);
            border-radius: 50%;
            border-top-color: #3498db;
            animation: spin 1s ease-in-out infinite;
        }

        @keyframes spin {
            to { transform: rotate(360deg); }
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Legal Pages Management</h1>
        </div>

        <div id="alerts"></div>

        <div class="tabs">
            <button class="tab active" data-tab="terms">Terms and Conditions</button>
            <button class="tab" data-tab="privacy">Privacy Policy</button>
        </div>

        <!-- Terms and Conditions Section -->
        <div id="terms-section" class="content-section active">
            <div class="form-group">
                <label for="terms-title">Document Title</label>
                <input type="text" id="terms-title" placeholder="Terms and Conditions" value="Terms and Conditions">
            </div>

            <div class="form-group">
                <label>Content</label>
                <div class="editor-container">
                    <div class="editor-toolbar">
                        <button class="editor-btn" onclick="formatText('bold', 'terms')">Bold</button>
                        <button class="editor-btn" onclick="formatText('italic', 'terms')">Italic</button>
                        <button class="editor-btn" onclick="formatText('underline', 'terms')">Underline</button>
                        <button class="editor-btn" onclick="formatText('h1', 'terms')">H1</button>
                        <button class="editor-btn" onclick="formatText('h2', 'terms')">H2</button>
                        <button class="editor-btn" onclick="formatText('h3', 'terms')">H3</button>
                        <button class="editor-btn" onclick="formatText('ul', 'terms')">• List</button>
                        <button class="editor-btn" onclick="formatText('ol', 'terms')">1. List</button>
                        <button class="editor-btn" onclick="insertLink('terms')">Link</button>
                    </div>
                    <div id="terms-editor" class="editor-content" contenteditable="true">
                        <p>Enter your terms and conditions content here...</p>
                    </div>
                </div>
            </div>

            <div class="version-info" id="terms-version">
                No saved versions yet
            </div>

            <div class="button-group">
                <button class="btn btn-primary" onclick="saveDocument('terms')">Save Draft</button>
                <button class="btn btn-secondary" onclick="previewDocument('terms')">Preview</button>
                <button class="btn btn-success" onclick="publishDocument('terms')">Save & Publish</button>
            </div>
        </div>

        <!-- Privacy Policy Section -->
        <div id="privacy-section" class="content-section">
            <div class="form-group">
                <label for="privacy-title">Document Title</label>
                <input type="text" id="privacy-title" placeholder="Privacy Policy" value="Privacy Policy">
            </div>

            <div class="form-group">
                <label>Content</label>
                <div class="editor-container">
                    <div class="editor-toolbar">
                        <button class="editor-btn" onclick="formatText('bold', 'privacy')">Bold</button>
                        <button class="editor-btn" onclick="formatText('italic', 'privacy')">Italic</button>
                        <button class="editor-btn" onclick="formatText('underline', 'privacy')">Underline</button>
                        <button class="editor-btn" onclick="formatText('h1', 'privacy')">H1</button>
                        <button class="editor-btn" onclick="formatText('h2', 'privacy')">H2</button>
                        <button class="editor-btn" onclick="formatText('h3', 'privacy')">H3</button>
                        <button class="editor-btn" onclick="formatText('ul', 'privacy')">• List</button>
                        <button class="editor-btn" onclick="formatText('ol', 'privacy')">1. List</button>
                        <button class="editor-btn" onclick="insertLink('privacy')">Link</button>
                    </div>
                    <div id="privacy-editor" class="editor-content" contenteditable="true">
                        <p>Enter your privacy policy content here...</p>
                    </div>
                </div>
            </div>

            <div class="version-info" id="privacy-version">
                No saved versions yet
            </div>

            <div class="button-group">
                <button class="btn btn-primary" onclick="saveDocument('privacy')">Save Draft</button>
                <button class="btn btn-secondary" onclick="previewDocument('privacy')">Preview</button>
                <button class="btn btn-success" onclick="publishDocument('privacy')">Save & Publish</button>
            </div>
        </div>
    </div>

    <!-- Preview Modal -->
    <div id="preview-modal" class="preview-modal">
        <div class="preview-content">
            <button class="preview-close" onclick="closePreview()">Close</button>
            <div id="preview-body"></div>
        </div>
    </div>

    <script>
        // Tab switching
        document.querySelectorAll('.tab').forEach(tab => {
            tab.addEventListener('click', function() {
                const tabName = this.dataset.tab;

                // Update tab styles
                document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
                this.classList.add('active');

                // Update content sections
                document.querySelectorAll('.content-section').forEach(section => {
                    section.classList.remove('active');
                });
                document.getElementById(tabName + '-section').classList.add('active');
            });
        });

        // Load existing documents on page load
        window.addEventListener('DOMContentLoaded', function() {
            loadDocument('terms');
            loadDocument('privacy');
        });

        // Load document from API
        async function loadDocument(type) {
            try {
                const response = await fetch('/api/ext/legalpages/api/documents/' + type);
                if (response.ok) {
                    const doc = await response.json();
                    if (doc && doc.content) {
                        document.getElementById(type + '-editor').innerHTML = doc.content;
                        document.getElementById(type + '-title').value = doc.title || (type === 'terms' ? 'Terms and Conditions' : 'Privacy Policy');
                        document.getElementById(type + '-version').textContent = 'Version ' + doc.version + ' - ' + (doc.is_published ? 'Published' : 'Draft');
                    }
                }
            } catch (error) {
                console.error('Error loading document:', error);
            }
        }

        // Save document
        async function saveDocument(type) {
            const title = document.getElementById(type + '-title').value;
            const content = document.getElementById(type + '-editor').innerHTML;

            showAlert('info', 'Saving document...');

            try {
                const response = await fetch('/api/ext/legalpages/api/documents/' + type, {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({ title, content })
                });

                if (response.ok) {
                    const doc = await response.json();
                    showAlert('success', 'Document saved successfully!');
                    document.getElementById(type + '-version').textContent = 'Version ' + doc.version + ' - Draft';
                } else {
                    showAlert('error', 'Failed to save document');
                }
            } catch (error) {
                showAlert('error', 'Error saving document: ' + error.message);
            }
        }

        // Publish document
        async function publishDocument(type) {
            await saveDocument(type);
            // The save endpoint auto-publishes in our implementation
            showAlert('success', 'Document published successfully!');
        }

        // Preview document
        async function previewDocument(type) {
            const title = document.getElementById(type + '-title').value;
            const content = document.getElementById(type + '-editor').innerHTML;

            document.getElementById('preview-body').innerHTML = '<h1>' + title + '</h1>' + content;
            document.getElementById('preview-modal').classList.add('active');
        }

        // Close preview
        function closePreview() {
            document.getElementById('preview-modal').classList.remove('active');
        }

        // Text formatting
        function formatText(command, editorType) {
            const editor = document.getElementById(editorType + '-editor');
            editor.focus();

            switch(command) {
                case 'bold':
                    document.execCommand('bold', false, null);
                    break;
                case 'italic':
                    document.execCommand('italic', false, null);
                    break;
                case 'underline':
                    document.execCommand('underline', false, null);
                    break;
                case 'h1':
                    document.execCommand('formatBlock', false, '<h1>');
                    break;
                case 'h2':
                    document.execCommand('formatBlock', false, '<h2>');
                    break;
                case 'h3':
                    document.execCommand('formatBlock', false, '<h3>');
                    break;
                case 'ul':
                    document.execCommand('insertUnorderedList', false, null);
                    break;
                case 'ol':
                    document.execCommand('insertOrderedList', false, null);
                    break;
            }
        }

        // Insert link
        function insertLink(editorType) {
            const url = prompt('Enter URL:');
            if (url) {
                const editor = document.getElementById(editorType + '-editor');
                editor.focus();
                document.execCommand('createLink', false, url);
            }
        }

        // Show alert
        function showAlert(type, message) {
            const alertsContainer = document.getElementById('alerts');
            const alert = document.createElement('div');
            alert.className = 'alert alert-' + type;
            alert.textContent = message;

            alertsContainer.innerHTML = '';
            alertsContainer.appendChild(alert);

            setTimeout(() => {
                alert.remove();
            }, 5000);
        }
    </script>
</body>
</html>
`