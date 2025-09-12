package cloudstorage

const dashboardHTML = `
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Cloud Storage - Solobase</title>
    <link rel="stylesheet" href="/static/css/common.css">
    <style>
        .storage-container {
            padding: 20px;
            max-width: 1400px;
            margin: 0 auto;
        }

        .storage-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 30px;
            padding-bottom: 20px;
            border-bottom: 1px solid var(--border-color);
        }

        .storage-title {
            font-size: 28px;
            font-weight: 600;
            color: var(--text-primary);
        }

        .storage-stats {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }

        .stat-card {
            background: var(--card-bg);
            backdrop-filter: blur(10px);
            border: 1px solid var(--border-color);
            border-radius: 12px;
            padding: 20px;
            transition: transform 0.2s;
        }

        .stat-card:hover {
            transform: translateY(-2px);
        }

        .stat-label {
            font-size: 12px;
            color: var(--text-muted);
            text-transform: uppercase;
            margin-bottom: 8px;
        }

        .stat-value {
            font-size: 24px;
            font-weight: 600;
            color: var(--text-primary);
        }

        .stat-detail {
            font-size: 14px;
            color: var(--text-secondary);
            margin-top: 4px;
        }

        .buckets-section {
            margin-bottom: 30px;
        }

        .section-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 20px;
        }

        .section-title {
            font-size: 20px;
            font-weight: 600;
            color: var(--text-primary);
        }

        .buckets-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
            gap: 20px;
        }

        .bucket-card {
            background: var(--card-bg);
            backdrop-filter: blur(10px);
            border: 1px solid var(--border-color);
            border-radius: 12px;
            padding: 20px;
            cursor: pointer;
            transition: all 0.2s;
        }

        .bucket-card:hover {
            border-color: var(--primary-color);
            transform: translateY(-2px);
        }

        .bucket-name {
            font-size: 18px;
            font-weight: 600;
            color: var(--text-primary);
            margin-bottom: 8px;
        }

        .bucket-info {
            display: flex;
            justify-content: space-between;
            font-size: 14px;
            color: var(--text-secondary);
        }

        .bucket-badge {
            display: inline-block;
            padding: 2px 8px;
            border-radius: 4px;
            font-size: 12px;
            font-weight: 500;
            margin-left: 8px;
        }

        .badge-public {
            background: var(--success-bg);
            color: var(--success-color);
        }

        .badge-private {
            background: var(--warning-bg);
            color: var(--warning-color);
        }

        .files-section {
            background: var(--card-bg);
            backdrop-filter: blur(10px);
            border: 1px solid var(--border-color);
            border-radius: 12px;
            padding: 20px;
        }

        .file-browser {
            min-height: 400px;
        }

        .breadcrumb {
            display: flex;
            align-items: center;
            margin-bottom: 20px;
            padding: 10px;
            background: var(--bg-secondary);
            border-radius: 8px;
        }

        .breadcrumb-item {
            color: var(--text-secondary);
            cursor: pointer;
            padding: 4px 8px;
            border-radius: 4px;
            transition: background 0.2s;
        }

        .breadcrumb-item:hover {
            background: var(--hover-bg);
        }

        .breadcrumb-separator {
            margin: 0 8px;
            color: var(--text-muted);
        }

        .file-list {
            display: table;
            width: 100%;
        }

        .file-row {
            display: table-row;
            cursor: pointer;
            transition: background 0.2s;
        }

        .file-row:hover {
            background: var(--hover-bg);
        }

        .file-cell {
            display: table-cell;
            padding: 12px;
            border-bottom: 1px solid var(--border-color);
        }

        .file-icon {
            width: 24px;
            height: 24px;
            margin-right: 12px;
            vertical-align: middle;
        }

        .file-name {
            color: var(--text-primary);
            font-weight: 500;
        }

        .file-size {
            color: var(--text-secondary);
            text-align: right;
        }

        .file-date {
            color: var(--text-muted);
            text-align: right;
        }

        .upload-zone {
            border: 2px dashed var(--border-color);
            border-radius: 12px;
            padding: 40px;
            text-align: center;
            margin-top: 20px;
            transition: all 0.3s;
        }

        .upload-zone.drag-over {
            border-color: var(--primary-color);
            background: var(--primary-bg);
        }

        .upload-icon {
            width: 48px;
            height: 48px;
            margin: 0 auto 16px;
            opacity: 0.5;
        }

        .upload-text {
            color: var(--text-secondary);
            margin-bottom: 12px;
        }

        .modal {
            display: none;
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background: rgba(0, 0, 0, 0.5);
            backdrop-filter: blur(5px);
            z-index: 1000;
        }

        .modal.active {
            display: flex;
            align-items: center;
            justify-content: center;
        }

        .modal-content {
            background: var(--card-bg);
            border: 1px solid var(--border-color);
            border-radius: 12px;
            padding: 24px;
            max-width: 500px;
            width: 90%;
        }

        .modal-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 20px;
        }

        .modal-title {
            font-size: 20px;
            font-weight: 600;
            color: var(--text-primary);
        }

        .modal-close {
            background: none;
            border: none;
            font-size: 24px;
            color: var(--text-muted);
            cursor: pointer;
        }

        .form-group {
            margin-bottom: 20px;
        }

        .form-label {
            display: block;
            margin-bottom: 8px;
            font-weight: 500;
            color: var(--text-primary);
        }

        .form-input {
            width: 100%;
            padding: 10px 12px;
            background: var(--bg-secondary);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            color: var(--text-primary);
            transition: border-color 0.2s;
        }

        .form-input:focus {
            outline: none;
            border-color: var(--primary-color);
        }

        .form-checkbox {
            margin-right: 8px;
        }

        .button-group {
            display: flex;
            gap: 12px;
            justify-content: flex-end;
            margin-top: 24px;
        }
    </style>
</head>
<body>
    <div class="storage-container">
        <div class="storage-header">
            <h1 class="storage-title">Cloud Storage</h1>
            <button class="btn btn-primary" onclick="showCreateBucketModal()">
                <svg width="16" height="16" fill="currentColor" style="margin-right: 8px;">
                    <use href="#icon-plus"></use>
                </svg>
                New Bucket
            </button>
        </div>

        <div class="storage-stats" id="stats">
            <div class="stat-card">
                <div class="stat-label">Total Storage</div>
                <div class="stat-value" id="totalSize">0 B</div>
                <div class="stat-detail" id="totalObjects">0 objects</div>
            </div>
            <div class="stat-card">
                <div class="stat-label">Buckets</div>
                <div class="stat-value" id="totalBuckets">0</div>
                <div class="stat-detail">Storage containers</div>
            </div>
            <div class="stat-card">
                <div class="stat-label">Provider</div>
                <div class="stat-value" id="provider">Local</div>
                <div class="stat-detail">Storage backend</div>
            </div>
            <div class="stat-card">
                <div class="stat-label">Max File Size</div>
                <div class="stat-value" id="maxFileSize">100 MB</div>
                <div class="stat-detail">Per file limit</div>
            </div>
        </div>

        <div class="buckets-section">
            <div class="section-header">
                <h2 class="section-title">Buckets</h2>
            </div>
            <div class="buckets-grid" id="bucketsGrid">
                <!-- Buckets will be loaded here -->
            </div>
        </div>

        <div class="files-section" id="filesSection" style="display: none;">
            <div class="section-header">
                <h2 class="section-title" id="filesSectionTitle">Files</h2>
                <div class="button-group">
                    <button class="btn btn-secondary" onclick="closeBucket()">Close</button>
                    <button class="btn btn-primary" onclick="showUploadModal()">Upload File</button>
                </div>
            </div>
            
            <div class="breadcrumb" id="breadcrumb">
                <!-- Breadcrumb will be loaded here -->
            </div>

            <div class="file-browser">
                <div class="file-list" id="fileList">
                    <!-- Files will be loaded here -->
                </div>
            </div>

            <div class="upload-zone" id="uploadZone">
                <svg class="upload-icon" fill="currentColor">
                    <use href="#icon-upload"></use>
                </svg>
                <div class="upload-text">Drop files here to upload</div>
                <button class="btn btn-secondary" onclick="showUploadModal()">Select Files</button>
            </div>
        </div>
    </div>

    <!-- Create Bucket Modal -->
    <div class="modal" id="createBucketModal">
        <div class="modal-content">
            <div class="modal-header">
                <h3 class="modal-title">Create New Bucket</h3>
                <button class="modal-close" onclick="hideCreateBucketModal()">&times;</button>
            </div>
            <form id="createBucketForm">
                <div class="form-group">
                    <label class="form-label">Bucket Name</label>
                    <input type="text" class="form-input" id="bucketName" required 
                           pattern="[a-z0-9-]+" placeholder="my-bucket">
                </div>
                <div class="form-group">
                    <label>
                        <input type="checkbox" class="form-checkbox" id="bucketPublic">
                        Public Access
                    </label>
                </div>
                <div class="button-group">
                    <button type="button" class="btn btn-secondary" onclick="hideCreateBucketModal()">Cancel</button>
                    <button type="submit" class="btn btn-primary">Create Bucket</button>
                </div>
            </form>
        </div>
    </div>

    <!-- Upload Modal -->
    <div class="modal" id="uploadModal">
        <div class="modal-content">
            <div class="modal-header">
                <h3 class="modal-title">Upload Files</h3>
                <button class="modal-close" onclick="hideUploadModal()">&times;</button>
            </div>
            <form id="uploadForm">
                <div class="form-group">
                    <label class="form-label">Select Files</label>
                    <input type="file" class="form-input" id="fileInput" multiple required>
                </div>
                <div class="form-group">
                    <label class="form-label">Upload Path (optional)</label>
                    <input type="text" class="form-input" id="uploadPath" placeholder="folder/subfolder">
                </div>
                <div class="button-group">
                    <button type="button" class="btn btn-secondary" onclick="hideUploadModal()">Cancel</button>
                    <button type="submit" class="btn btn-primary">Upload</button>
                </div>
            </form>
        </div>
    </div>

    <!-- SVG Icons -->
    <svg style="display: none;">
        <defs>
            <g id="icon-plus">
                <path d="M12 5v14m-7-7h14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            </g>
            <g id="icon-upload">
                <path d="M7 10l5-5m0 0l5 5m-5-5v12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                <path d="M3 17v3a2 2 0 002 2h14a2 2 0 002-2v-3" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            </g>
            <g id="icon-folder">
                <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" fill="currentColor"/>
            </g>
            <g id="icon-file">
                <path d="M9 2H5a2 2 0 00-2 2v16a2 2 0 002 2h14a2 2 0 002-2V8l-6-6z" stroke="currentColor" stroke-width="2" fill="none"/>
                <path d="M9 2v6h6" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            </g>
        </defs>
    </svg>

    <script>
        let currentBucket = null;
        let currentPath = '';

        // Load initial data
        async function loadData() {
            await loadStats();
            await loadBuckets();
        }

        // Load statistics
        async function loadStats() {
            try {
                const response = await fetch('/ext/cloudstorage/api/stats');
                const data = await response.json();
                
                document.getElementById('totalSize').textContent = data.total_size_formatted || '0 B';
                document.getElementById('totalObjects').textContent = data.total_objects + ' objects';
                document.getElementById('totalBuckets').textContent = data.total_buckets || '0';
                document.getElementById('provider').textContent = capitalizeFirst(data.provider || 'local');
                document.getElementById('maxFileSize').textContent = formatBytes(data.max_file_size || 0);
            } catch (error) {
                console.error('Failed to load stats:', error);
            }
        }

        // Load buckets
        async function loadBuckets() {
            try {
                const response = await fetch('/ext/cloudstorage/api/buckets');
                const buckets = await response.json();
                
                const grid = document.getElementById('bucketsGrid');
                grid.innerHTML = '';
                
                buckets.forEach(bucket => {
                    const card = document.createElement('div');
                    card.className = 'bucket-card';
                    card.onclick = () => openBucket(bucket.Name);
                    
                    card.innerHTML = ` + "`" + `
                        <div class="bucket-name">
                            ${bucket.Name}
                            <span class="bucket-badge ${bucket.Public ? 'badge-public' : 'badge-private'}">
                                ${bucket.Public ? 'Public' : 'Private'}
                            </span>
                        </div>
                        <div class="bucket-info">
                            <span>${bucket.ObjectCount || 0} objects</span>
                            <span>${formatBytes(bucket.TotalSize || 0)}</span>
                        </div>
                    ` + "`" + `;
                    
                    grid.appendChild(card);
                });
            } catch (error) {
                console.error('Failed to load buckets:', error);
            }
        }

        // Open bucket
        async function openBucket(bucketName) {
            currentBucket = bucketName;
            currentPath = '';
            
            document.getElementById('filesSection').style.display = 'block';
            document.getElementById('filesSectionTitle').textContent = 'Files in ' + bucketName;
            
            updateBreadcrumb();
            await loadFiles();
        }

        // Close bucket view
        function closeBucket() {
            currentBucket = null;
            currentPath = '';
            document.getElementById('filesSection').style.display = 'none';
        }

        // Load files in current bucket/path
        async function loadFiles() {
            if (!currentBucket) return;
            
            try {
                const url = '/ext/cloudstorage/api/objects?bucket=' + encodeURIComponent(currentBucket) +
                           (currentPath ? '&prefix=' + encodeURIComponent(currentPath) : '');
                           
                const response = await fetch(url);
                const files = await response.json();
                
                const fileList = document.getElementById('fileList');
                fileList.innerHTML = ` + "`" + `
                    <div class="file-row" style="font-weight: 600; border-bottom: 2px solid var(--border-color);">
                        <div class="file-cell">Name</div>
                        <div class="file-cell file-size">Size</div>
                        <div class="file-cell file-date">Modified</div>
                    </div>
                ` + "`" + `;
                
                files.forEach(file => {
                    const row = document.createElement('div');
                    row.className = 'file-row';
                    
                    const icon = file.is_folder ? '#icon-folder' : '#icon-file';
                    const name = file.key.split('/').pop() || file.key;
                    
                    row.innerHTML = ` + "`" + `
                        <div class="file-cell">
                            <svg class="file-icon" fill="currentColor">
                                <use href="${icon}"></use>
                            </svg>
                            <span class="file-name">${name}</span>
                        </div>
                        <div class="file-cell file-size">${file.is_folder ? '-' : formatBytes(file.size)}</div>
                        <div class="file-cell file-date">${formatDate(file.updated_at)}</div>
                    ` + "`" + `;
                    
                    if (file.is_folder) {
                        row.onclick = () => navigateToFolder(file.key);
                    } else {
                        row.onclick = () => downloadFile(file.id, name);
                    }
                    
                    fileList.appendChild(row);
                });
            } catch (error) {
                console.error('Failed to load files:', error);
            }
        }

        // Navigate to folder
        function navigateToFolder(folderPath) {
            currentPath = folderPath;
            updateBreadcrumb();
            loadFiles();
        }

        // Update breadcrumb
        function updateBreadcrumb() {
            const breadcrumb = document.getElementById('breadcrumb');
            breadcrumb.innerHTML = '';
            
            // Root/bucket item
            const rootItem = document.createElement('span');
            rootItem.className = 'breadcrumb-item';
            rootItem.textContent = currentBucket;
            rootItem.onclick = () => {
                currentPath = '';
                loadFiles();
            };
            breadcrumb.appendChild(rootItem);
            
            // Path items
            if (currentPath) {
                const parts = currentPath.split('/').filter(p => p);
                parts.forEach((part, index) => {
                    const separator = document.createElement('span');
                    separator.className = 'breadcrumb-separator';
                    separator.textContent = '/';
                    breadcrumb.appendChild(separator);
                    
                    const item = document.createElement('span');
                    item.className = 'breadcrumb-item';
                    item.textContent = part;
                    item.onclick = () => {
                        currentPath = parts.slice(0, index + 1).join('/');
                        loadFiles();
                    };
                    breadcrumb.appendChild(item);
                });
            }
        }

        // Download file
        async function downloadFile(fileId, fileName) {
            window.open('/ext/cloudstorage/api/download/' + fileId, '_blank');
        }

        // Show create bucket modal
        function showCreateBucketModal() {
            document.getElementById('createBucketModal').classList.add('active');
        }

        // Hide create bucket modal
        function hideCreateBucketModal() {
            document.getElementById('createBucketModal').classList.remove('active');
            document.getElementById('createBucketForm').reset();
        }

        // Show upload modal
        function showUploadModal() {
            document.getElementById('uploadModal').classList.add('active');
            document.getElementById('uploadPath').value = currentPath;
        }

        // Hide upload modal
        function hideUploadModal() {
            document.getElementById('uploadModal').classList.remove('active');
            document.getElementById('uploadForm').reset();
        }

        // Create bucket form handler
        document.getElementById('createBucketForm').addEventListener('submit', async (e) => {
            e.preventDefault();
            
            const name = document.getElementById('bucketName').value;
            const isPublic = document.getElementById('bucketPublic').checked;
            
            try {
                const response = await fetch('/ext/cloudstorage/api/buckets', {
                    method: 'POST',
                    headers: {'Content-Type': 'application/json'},
                    body: JSON.stringify({name, public: isPublic})
                });
                
                if (response.ok) {
                    hideCreateBucketModal();
                    await loadBuckets();
                    await loadStats();
                } else {
                    alert('Failed to create bucket');
                }
            } catch (error) {
                console.error('Failed to create bucket:', error);
                alert('Failed to create bucket');
            }
        });

        // Upload form handler
        document.getElementById('uploadForm').addEventListener('submit', async (e) => {
            e.preventDefault();
            
            const files = document.getElementById('fileInput').files;
            const path = document.getElementById('uploadPath').value;
            
            for (const file of files) {
                const formData = new FormData();
                formData.append('file', file);
                formData.append('bucket', currentBucket);
                formData.append('path', path);
                
                try {
                    await fetch('/ext/cloudstorage/api/upload', {
                        method: 'POST',
                        body: formData
                    });
                } catch (error) {
                    console.error('Failed to upload file:', file.name, error);
                }
            }
            
            hideUploadModal();
            await loadFiles();
            await loadStats();
        });

        // Drag and drop support
        const uploadZone = document.getElementById('uploadZone');
        
        uploadZone.addEventListener('dragover', (e) => {
            e.preventDefault();
            uploadZone.classList.add('drag-over');
        });
        
        uploadZone.addEventListener('dragleave', () => {
            uploadZone.classList.remove('drag-over');
        });
        
        uploadZone.addEventListener('drop', async (e) => {
            e.preventDefault();
            uploadZone.classList.remove('drag-over');
            
            const files = e.dataTransfer.files;
            for (const file of files) {
                const formData = new FormData();
                formData.append('file', file);
                formData.append('bucket', currentBucket);
                formData.append('path', currentPath);
                
                try {
                    await fetch('/ext/cloudstorage/api/upload', {
                        method: 'POST',
                        body: formData
                    });
                } catch (error) {
                    console.error('Failed to upload file:', file.name, error);
                }
            }
            
            await loadFiles();
            await loadStats();
        });

        // Helper functions
        function formatBytes(bytes) {
            if (bytes === 0) return '0 B';
            const k = 1024;
            const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
            const i = Math.floor(Math.log(bytes) / Math.log(k));
            return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
        }

        function formatDate(dateStr) {
            const date = new Date(dateStr);
            return date.toLocaleDateString() + ' ' + date.toLocaleTimeString();
        }

        function capitalizeFirst(str) {
            return str.charAt(0).toUpperCase() + str.slice(1);
        }

        // Load initial data on page load
        loadData();
    </script>
</body>
</html>
`
