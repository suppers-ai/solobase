// Storage Management JavaScript
(function() {
    'use strict';
    
    // Check if already initialized to prevent re-declaration
    if (window.storageInitialized) {
        return;
    }
    window.storageInitialized = true;

// Navigation functions
window.selectBucket = function(element) {
    let bucketName;
    if (typeof element === 'string') {
        bucketName = element;
    } else if (element && element.dataset) {
        bucketName = element.dataset.bucket;
    } else {
        console.error('Invalid bucket element or name');
        return;
    }
    
    if (!bucketName) {
        console.error('Bucket name is undefined');
        return;
    }
    
    window.location.href = `/storage?bucket=${encodeURIComponent(bucketName)}`;
}

window.navigateTo = function(element, path) {
    let bucket, navPath;
    
    if (typeof element === 'object' && element.dataset) {
        bucket = element.dataset.bucket;
        navPath = element.dataset.path || path || '';
    } else if (typeof element === 'string') {
        bucket = element;
        navPath = path || '';
    } else {
        console.error('Invalid navigation parameters');
        return;
    }
    
    let url = `/storage?bucket=${encodeURIComponent(bucket)}`;
    if (navPath) {
        url += `&path=${encodeURIComponent(navPath)}`;
    }
    window.location.href = url;
}

window.reloadStorage = function() {
    window.location.reload();
}

// Bucket management
window.showCreateBucketModal = function() {
    document.getElementById('createBucketModal').style.display = 'block';
}

window.hideCreateBucketModal = function() {
    document.getElementById('createBucketModal').style.display = 'none';
}

window.createBucket = async function(event) {
    event.preventDefault();
    const form = event.target;
    const formData = new FormData(form);
    
    try {
        const response = await fetch('/api/storage/buckets', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                name: formData.get('name'),
                public: formData.get('public') === 'on'
            })
        });
        
        if (response.ok) {
            hideCreateBucketModal();
            window.location.reload();
        } else {
            const error = await response.text();
            alert('Failed to create bucket: ' + error);
        }
    } catch (error) {
        alert('Error creating bucket: ' + error.message);
    }
}

window.filterBuckets = function(searchTerm) {
    const buckets = document.querySelectorAll('.bucket-item');
    const term = searchTerm.toLowerCase();
    
    buckets.forEach(bucket => {
        const name = bucket.querySelector('.bucket-name').textContent.toLowerCase();
        if (name.includes(term)) {
            bucket.style.display = '';
        } else {
            bucket.style.display = 'none';
        }
    });
}

// File management
window.showUploadModal = function() {
    document.getElementById('uploadModal').style.display = 'block';
}

window.hideUploadModal = function() {
    document.getElementById('uploadModal').style.display = 'none';
}

window.uploadFiles = async function(event) {
    event.preventDefault();
    const form = event.target;
    const formData = new FormData();
    
    const bucket = form.bucket.value;
    const path = form.path.value;
    const files = form.files.files;
    
    // Show progress
    const progressDiv = document.querySelector('.upload-progress');
    const progressFill = document.getElementById('progressFill');
    const progressText = document.getElementById('progressText');
    progressDiv.style.display = 'block';
    
    let uploaded = 0;
    const total = files.length;
    
    for (let i = 0; i < files.length; i++) {
        const file = files[i];
        const fileFormData = new FormData();
        fileFormData.append('bucket', bucket);
        fileFormData.append('path', path);
        fileFormData.append('file', file);
        
        try {
            const response = await fetch('/api/storage/upload', {
                method: 'POST',
                body: fileFormData
            });
            
            if (!response.ok) {
                throw new Error(`Failed to upload ${file.name}`);
            }
            
            uploaded++;
            const percent = Math.round((uploaded / total) * 100);
            progressFill.style.width = percent + '%';
            progressText.textContent = percent + '%';
        } catch (error) {
            alert('Error uploading file: ' + error.message);
        }
    }
    
    hideUploadModal();
    window.location.reload();
}

// Folder management
window.showCreateFolderModal = function() {
    document.getElementById('createFolderModal').style.display = 'block';
}

window.hideCreateFolderModal = function() {
    document.getElementById('createFolderModal').style.display = 'none';
}

window.createFolder = async function(event) {
    event.preventDefault();
    const form = event.target;
    const formData = new FormData(form);
    
    try {
        const response = await fetch('/api/storage/folders', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                bucket: formData.get('bucket'),
                path: formData.get('path'),
                name: formData.get('name')
            })
        });
        
        if (response.ok) {
            hideCreateFolderModal();
            window.location.reload();
        } else {
            const error = await response.text();
            alert('Failed to create folder: ' + error);
        }
    } catch (error) {
        alert('Error creating folder: ' + error.message);
    }
}

// File operations
window.downloadFile = async function(element) {
    let bucket, path;
    
    if (typeof element === 'object' && element.dataset) {
        bucket = element.dataset.bucket;
        path = element.dataset.path;
    } else if (arguments.length === 2) {
        // Legacy support for direct parameters
        bucket = arguments[0];
        path = arguments[1];
    } else {
        console.error('Invalid download parameters');
        return;
    }
    
    window.location.href = `/api/storage/${encodeURIComponent(bucket)}/download?path=${encodeURIComponent(path)}`;
}

window.deleteFile = async function(element) {
    let bucket, path;
    
    if (typeof element === 'object' && element.dataset) {
        bucket = element.dataset.bucket;
        path = element.dataset.path;
    } else if (arguments.length === 2) {
        // Legacy support for direct parameters
        bucket = arguments[0];
        path = arguments[1];
    } else {
        console.error('Invalid delete parameters');
        return;
    }
    
    if (!confirm('Are you sure you want to delete this file?')) {
        return;
    }
    
    try {
        const response = await fetch(`/api/storage/${encodeURIComponent(bucket)}/files`, {
            method: 'DELETE',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ path })
        });
        
        if (response.ok) {
            window.location.reload();
        } else {
            const error = await response.text();
            alert('Failed to delete file: ' + error);
        }
    } catch (error) {
        alert('Error deleting file: ' + error.message);
    }
}

// Signed URL Modal functions
window.showSignedURLModal = function(element) {
    const bucket = element.dataset.bucket;
    const path = element.dataset.path;
    const filename = element.dataset.filename;
    
    document.getElementById('signedBucket').value = bucket;
    document.getElementById('signedPath').value = path;
    document.getElementById('signedFileName').textContent = filename;
    
    // Reset form
    document.getElementById('expirySelect').value = '2592000'; // Default to 1 month
    document.getElementById('customExpiryGroup').style.display = 'none';
    document.getElementById('urlResultGroup').style.display = 'none';
    
    document.getElementById('signedURLModal').style.display = 'block';
}

window.hideSignedURLModal = function() {
    document.getElementById('signedURLModal').style.display = 'none';
}

window.handleExpiryChange = function(select) {
    const customGroup = document.getElementById('customExpiryGroup');
    if (select.value === 'custom') {
        customGroup.style.display = 'block';
    } else {
        customGroup.style.display = 'none';
    }
}

window.generateSignedURL = async function() {
    const bucket = document.getElementById('signedBucket').value;
    const path = document.getElementById('signedPath').value;
    const expirySelect = document.getElementById('expirySelect').value;
    
    let expiry;
    if (expirySelect === 'custom') {
        expiry = parseInt(document.getElementById('customExpiry').value);
        if (!expiry || expiry < 60 || expiry > 31536000) {
            alert('Please enter a valid expiry time between 60 and 31536000 seconds');
            return;
        }
    } else {
        expiry = parseInt(expirySelect);
    }
    
    try {
        const response = await fetch('/api/storage/signed-url', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                bucket: bucket,
                path: path,
                expiry: expiry
            })
        });
        
        if (response.ok) {
            const data = await response.json();
            const fullURL = window.location.origin + data.url;
            
            document.getElementById('signedURLResult').value = fullURL;
            
            // Calculate expiry date
            const expiryDate = new Date();
            expiryDate.setSeconds(expiryDate.getSeconds() + expiry);
            document.getElementById('expiryNote').textContent = 
                `This URL will expire on ${expiryDate.toLocaleString()}`;
            
            document.getElementById('urlResultGroup').style.display = 'block';
        } else {
            const error = await response.text();
            alert('Failed to generate signed URL: ' + error);
        }
    } catch (error) {
        alert('Error generating signed URL: ' + error.message);
    }
}

window.copySignedURL = function() {
    const urlInput = document.getElementById('signedURLResult');
    urlInput.select();
    document.execCommand('copy');
    
    // Show feedback
    const copyBtn = event.target.closest('button');
    const originalContent = copyBtn.innerHTML;
    copyBtn.innerHTML = 'Copied!';
    setTimeout(() => {
        copyBtn.innerHTML = originalContent;
    }, 2000);
}

// File preview
window.currentFile = null;

window.selectFile = async function(element) {
    let bucket, path;
    
    if (typeof element === 'object' && element.dataset) {
        bucket = element.dataset.bucket;
        path = element.dataset.path;
    } else if (arguments.length === 2) {
        // Legacy support for direct parameters
        bucket = arguments[0];
        path = arguments[1];
    } else {
        console.error('Invalid file selection parameters');
        return;
    }
    
    window.currentFile = { bucket, path };
    
    try {
        const response = await fetch(`/api/storage/${encodeURIComponent(bucket)}/view?path=${encodeURIComponent(path)}`);
        if (!response.ok) {
            throw new Error('Failed to load file');
        }
        
        const data = await response.json();
        showFilePreview(data);
    } catch (error) {
        console.error('Error loading file:', error);
    }
}

window.showFilePreview = function(data) {
    const preview = document.getElementById('filePreview');
    const fileName = document.getElementById('previewFileName');
    const image = document.getElementById('previewImage');
    const text = document.getElementById('previewText');
    const info = document.getElementById('previewInfo');
    const type = document.getElementById('previewType');
    const size = document.getElementById('previewSize');
    const modified = document.getElementById('previewModified');
    
    fileName.textContent = data.path.split('/').pop();
    type.textContent = data.contentType;
    
    // Hide all preview types
    image.style.display = 'none';
    text.style.display = 'none';
    info.style.display = 'block';
    
    if (data.contentType.startsWith('image/')) {
        // Show image
        image.src = `/api/storage/${window.currentFile.bucket}/download?path=${window.currentFile.path}`;
        image.style.display = 'block';
    } else if (data.editable) {
        // Show text content
        text.textContent = data.content;
        text.style.display = 'block';
    }
    
    // Setup download button
    document.getElementById('downloadBtn').onclick = () => window.downloadFile(window.currentFile.bucket, window.currentFile.path);
    document.getElementById('deleteBtn').onclick = () => window.deleteFile(window.currentFile.bucket, window.currentFile.path);
    
    preview.style.display = 'flex';
}

window.closeFilePreview = function() {
    document.getElementById('filePreview').style.display = 'none';
    window.currentFile = null;
}

window.showFileViewer = function() {
    // Placeholder for file viewer functionality
    alert('File viewer coming soon');
}

window.showBucketPolicies = function() {
    // Placeholder for bucket policies
    alert('Bucket policies management coming soon');
}

window.showBucketSettings = function() {
    // Placeholder for bucket settings
    alert('Bucket settings coming soon');
}

// Initialize on page load
document.addEventListener('DOMContentLoaded', function() {
    // Close modals when clicking outside
    window.onclick = function(event) {
        if (event.target.classList.contains('modal')) {
            event.target.style.display = 'none';
        }
    };
});

})(); // End of IIFE