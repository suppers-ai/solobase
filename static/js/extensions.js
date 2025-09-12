// Extensions Management JavaScript
(function() {
    'use strict';
    
    // Check if already initialized to prevent re-declaration
    if (window.extensionsInitialized) {
        return;
    }
    window.extensionsInitialized = true;

    // Store extension data for modal display
    window.extensionsData = {};

    // Initialize extensions page
    window.initializeExtensionsPage = function() {
        console.log('Initializing extensions page...');
        
        // Parse extension data from DOM
        const extensionCards = document.querySelectorAll('.extension-card');
        extensionCards.forEach(card => {
            const id = card.dataset.extensionId;
            if (id) {
                const nameEl = card.querySelector('.extension-name');
                const descEl = card.querySelector('.extension-description');
                const versionEl = card.querySelector('.meta-item i[data-lucide="tag"]')?.parentElement;
                const authorEl = card.querySelector('.meta-item i[data-lucide="user"]')?.parentElement;
                const categoryEl = card.querySelector('.category-badge');
                const statusEl = card.querySelector('.status-badge');
                const iconEl = card.querySelector('.extension-icon i');
                
                window.extensionsData[id] = {
                    id: id,
                    name: nameEl?.textContent || 'Unknown Extension',
                    description: descEl?.textContent || 'No description available',
                    version: versionEl?.textContent?.replace(/^v/, '') || '1.0.0',
                    author: authorEl?.textContent?.trim() || 'Unknown',
                    category: categoryEl?.textContent?.trim() || 'uncategorized',
                    enabled: statusEl?.classList.contains('active') || false,
                    icon: iconEl?.getAttribute('data-lucide') || 'puzzle'
                };
            }
        });
        
        // Initialize Lucide icons if available
        if (typeof lucide !== 'undefined') {
            lucide.createIcons();
        }
    };

    // Show extension details modal
    window.showExtensionDetails = function(extensionId) {
        const ext = window.extensionsData[extensionId];
        if (!ext) {
            console.error('Extension not found:', extensionId);
            return;
        }
        
        // Update modal content
        document.getElementById('modalExtensionName').textContent = ext.name;
        document.getElementById('modalExtensionVersion').textContent = 'v' + ext.version;
        document.getElementById('modalExtensionStatus').textContent = ext.enabled ? 'Active' : 'Inactive';
        document.getElementById('modalExtensionStatus').className = ext.enabled ? 'status-badge active' : 'status-badge inactive';
        document.getElementById('modalExtensionCategory').textContent = ext.category;
        document.getElementById('modalExtensionAuthor').textContent = ext.author;
        
        // Get detailed description based on extension
        const detailedDesc = getExtensionDescription(extensionId);
        document.getElementById('modalExtensionDescription').textContent = detailedDesc || ext.description;
        
        // Update features list
        const featuresList = document.getElementById('modalExtensionFeatures');
        const features = getExtensionFeatures(extensionId);
        featuresList.innerHTML = features.map(feature => 
            `<li><i data-lucide="check"></i> ${feature}</li>`
        ).join('');
        
        // Update endpoints
        const endpointsList = document.getElementById('modalExtensionEndpoints');
        const endpoints = getExtensionEndpoints(extensionId);
        endpointsList.innerHTML = endpoints.map(endpoint => 
            `<div class="endpoint-item">
                <code>${endpoint.path}</code>
                <span class="endpoint-desc">${endpoint.description}</span>
            </div>`
        ).join('');
        
        // Update permissions
        const permissionsList = document.getElementById('modalExtensionPermissions');
        const permissions = getExtensionPermissions(extensionId);
        permissionsList.innerHTML = permissions.map(permission => 
            `<span class="permission-badge">${permission}</span>`
        ).join('');
        
        // Update toggle button
        const toggleBtn = document.getElementById('modalToggleBtn');
        const toggleText = document.getElementById('modalToggleText');
        toggleBtn.dataset.extensionId = extensionId;
        toggleBtn.dataset.currentState = ext.enabled;
        
        if (ext.enabled) {
            toggleBtn.className = 'btn-secondary';
            toggleText.textContent = 'Disable';
        } else {
            toggleBtn.className = 'btn-primary';
            toggleText.textContent = 'Enable';
        }
        
        // Show modal
        document.getElementById('extensionDetailsModal').style.display = 'block';
        
        // Reinitialize Lucide icons in modal
        if (typeof lucide !== 'undefined') {
            setTimeout(() => lucide.createIcons(), 100);
        }
    };

    // Close extension details modal
    window.closeExtensionDetails = function() {
        document.getElementById('extensionDetailsModal').style.display = 'none';
    };

    // Toggle extension from modal
    window.toggleExtensionFromModal = function() {
        const toggleBtn = document.getElementById('modalToggleBtn');
        const extensionId = toggleBtn.dataset.extensionId;
        const currentState = toggleBtn.dataset.currentState === 'true';
        
        toggleExtension(extensionId, !currentState);
    };

    // Toggle extension state
    window.toggleExtension = async function(extensionId, enable) {
        try {
            // Show loading state
            const toggleBtns = document.querySelectorAll(`[data-ext-id="${extensionId}"]`);
            toggleBtns.forEach(btn => {
                btn.disabled = true;
                btn.innerHTML = '<i data-lucide="loader"></i>';
            });
            
            // Make API call
            const response = await fetch(`/api/extensions/${extensionId}/toggle`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ enabled: enable })
            });
            
            if (response.ok) {
                // Update local state
                if (window.extensionsData[extensionId]) {
                    window.extensionsData[extensionId].enabled = enable;
                }
                
                // Update UI
                updateExtensionUI(extensionId, enable);
                
                // Show success message
                showNotification(`Extension ${enable ? 'enabled' : 'disabled'} successfully`, 'success');
            } else {
                throw new Error('Failed to toggle extension');
            }
        } catch (error) {
            console.error('Error toggling extension:', error);
            showNotification('Failed to toggle extension', 'error');
        } finally {
            // Restore button state
            const toggleBtns = document.querySelectorAll(`[data-ext-id="${extensionId}"]`);
            toggleBtns.forEach(btn => {
                btn.disabled = false;
                btn.innerHTML = enable ? '<i data-lucide="power-off"></i>' : '<i data-lucide="power"></i>';
            });
            
            if (typeof lucide !== 'undefined') {
                lucide.createIcons();
            }
        }
    };

    // Update extension UI after toggle
    function updateExtensionUI(extensionId, enabled) {
        // Update card
        const card = document.querySelector(`[data-extension-id="${extensionId}"]`);
        if (card) {
            if (enabled) {
                card.classList.add('enabled');
                card.classList.remove('disabled');
            } else {
                card.classList.remove('enabled');
                card.classList.add('disabled');
            }
            
            const statusBadge = card.querySelector('.status-badge');
            if (statusBadge) {
                statusBadge.textContent = enabled ? 'Active' : 'Inactive';
                statusBadge.className = enabled ? 'status-badge active' : 'status-badge inactive';
            }
            
            const toggleBtn = card.querySelector('.toggle-btn');
            if (toggleBtn) {
                toggleBtn.className = enabled ? 'toggle-btn active' : 'toggle-btn';
            }
        }
        
        // Update modal if open
        const modal = document.getElementById('extensionDetailsModal');
        if (modal && modal.style.display !== 'none') {
            const modalStatus = document.getElementById('modalExtensionStatus');
            if (modalStatus) {
                modalStatus.textContent = enabled ? 'Active' : 'Inactive';
                modalStatus.className = enabled ? 'status-badge active' : 'status-badge inactive';
            }
            
            const toggleBtn = document.getElementById('modalToggleBtn');
            const toggleText = document.getElementById('modalToggleText');
            if (toggleBtn && toggleText) {
                toggleBtn.dataset.currentState = enabled;
                if (enabled) {
                    toggleBtn.className = 'btn-secondary';
                    toggleText.textContent = 'Disable';
                } else {
                    toggleBtn.className = 'btn-primary';
                    toggleText.textContent = 'Enable';
                }
            }
        }
        
        // Update stats
        updateExtensionStats();
    }

    // Update extension statistics
    function updateExtensionStats() {
        const extensions = Object.values(window.extensionsData);
        const activeCount = extensions.filter(ext => ext.enabled).length;
        const officialCount = extensions.filter(ext => ext.category === 'official').length;
        const communityCount = extensions.filter(ext => ext.category === 'community').length;
        
        // Update stat cards
        const statValues = document.querySelectorAll('.stat-value');
        if (statValues[0]) statValues[0].textContent = extensions.length;
        if (statValues[1]) statValues[1].textContent = activeCount;
        if (statValues[2]) statValues[2].textContent = officialCount;
        if (statValues[3]) statValues[3].textContent = communityCount;
    }

    // Refresh extensions list
    window.refreshExtensions = async function() {
        try {
            // Show loading state
            const refreshBtn = event.target?.closest('button');
            if (refreshBtn) {
                const originalContent = refreshBtn.innerHTML;
                refreshBtn.disabled = true;
                refreshBtn.innerHTML = '<i data-lucide="loader" class="animate-spin"></i> Refreshing...';
            }
            
            // Reload the page (or make API call to get fresh data)
            window.location.reload();
        } catch (error) {
            console.error('Error refreshing extensions:', error);
            showNotification('Failed to refresh extensions', 'error');
        }
    };

    // Show extension marketplace (placeholder)
    window.showExtensionMarketplace = function() {
        showNotification('Extension marketplace coming soon!', 'info');
    };

    // Show notification
    function showNotification(message, type = 'info') {
        // Create notification element if it doesn't exist
        let notification = document.getElementById('notification');
        if (!notification) {
            notification = document.createElement('div');
            notification.id = 'notification';
            notification.style.cssText = `
                position: fixed;
                top: 20px;
                right: 20px;
                padding: 12px 20px;
                border-radius: 8px;
                color: white;
                font-weight: 500;
                z-index: 10000;
                transition: all 0.3s ease;
                display: none;
            `;
            document.body.appendChild(notification);
        }
        
        // Set notification style based on type
        const colors = {
            success: '#10b981',
            error: '#ef4444',
            info: '#3b82f6',
            warning: '#f59e0b'
        };
        
        notification.style.backgroundColor = colors[type] || colors.info;
        notification.textContent = message;
        notification.style.display = 'block';
        
        // Auto-hide after 3 seconds
        setTimeout(() => {
            notification.style.display = 'none';
        }, 3000);
    }

    // Get detailed description for extension
    function getExtensionDescription(extensionId) {
        const descriptions = {
            'auth-oauth': 'Complete OAuth 2.0 authentication system supporting multiple providers including Google, GitHub, Facebook, and more. Handles token management, refresh tokens, and secure session handling.',
            'storage-s3': 'S3-compatible object storage integration with support for multipart uploads, signed URLs, bucket management, and CDN integration. Works with AWS S3, MinIO, and other S3-compatible services.',
            'analytics': 'Comprehensive analytics and tracking system that monitors user behavior, page views, custom events, and performance metrics. Includes real-time dashboards and exportable reports.',
            'email-smtp': 'Email service integration supporting SMTP providers, templating, attachments, and bulk sending with queue management. Includes bounce handling and delivery tracking.',
            'payment-stripe': 'Stripe payment processing integration with support for subscriptions, one-time payments, webhooks, and customer portal access.',
            'search-elastic': 'Elasticsearch integration providing full-text search, faceted search, autocomplete, and advanced query capabilities across your data.',
            'cache-redis': 'Redis caching layer for improved performance with support for session storage, rate limiting, and real-time pub/sub messaging.',
            'queue-jobs': 'Background job processing system with scheduled tasks, retries, priority queues, and worker management.',
            'media-processing': 'Image and video processing extension with thumbnail generation, format conversion, compression, and CDN optimization.',
            'webhooks': 'Webhook management system for integrating with external services. Create, manage, and monitor webhooks with retry logic and delivery logs.'
        };
        
        return descriptions[extensionId] || null;
    }

    // Get features for extension
    function getExtensionFeatures(extensionId) {
        const features = {
            'auth-oauth': [
                'OAuth 2.0 authentication with multiple providers',
                'Secure token management and refresh',
                'Session handling with remember me option',
                'Social login integration',
                'Two-factor authentication support'
            ],
            'storage-s3': [
                'S3-compatible object storage',
                'Multipart upload for large files',
                'Signed URL generation for secure access',
                'Bucket management and policies',
                'CDN integration for fast delivery'
            ],
            'analytics': [
                'Real-time analytics dashboard',
                'Custom event tracking',
                'User behavior analysis',
                'Performance metrics monitoring',
                'Exportable reports and data'
            ],
            'email-smtp': [
                'SMTP email sending',
                'HTML and plain text templates',
                'Attachment support',
                'Bulk email with queue management',
                'Delivery tracking and bounce handling'
            ],
            'payment-stripe': [
                'One-time payment processing',
                'Subscription management',
                'Customer portal integration',
                'Webhook event handling',
                'Invoice and receipt generation'
            ],
            'search-elastic': [
                'Full-text search capabilities',
                'Faceted search and filtering',
                'Autocomplete suggestions',
                'Advanced query syntax',
                'Search analytics and relevance tuning'
            ],
            'cache-redis': [
                'High-performance caching',
                'Session storage',
                'Rate limiting',
                'Real-time pub/sub messaging',
                'Data expiration policies'
            ],
            'queue-jobs': [
                'Background job processing',
                'Scheduled and recurring tasks',
                'Priority queue management',
                'Retry logic with backoff',
                'Worker scaling and monitoring'
            ],
            'media-processing': [
                'Image resizing and optimization',
                'Video transcoding',
                'Thumbnail generation',
                'Format conversion',
                'CDN-ready output'
            ],
            'webhooks': [
                'Create and manage webhooks',
                'Event-driven triggers',
                'Retry mechanisms with exponential backoff',
                'Delivery logs and monitoring',
                'Custom payload templates'
            ]
        };
        
        return features[extensionId] || [
            'Core functionality',
            'API integration',
            'Data processing',
            'Event handling',
            'Configuration management'
        ];
    }

    // Get endpoints for extension
    function getExtensionEndpoints(extensionId) {
        const endpoints = {
            'auth-oauth': [
                { path: '/ext/auth/login', description: 'Initiate OAuth login' },
                { path: '/ext/auth/callback', description: 'Handle OAuth callback' },
                { path: '/ext/auth/logout', description: 'Logout user session' },
                { path: '/ext/auth/refresh', description: 'Refresh access token' },
                { path: '/ext/auth/profile', description: 'Get user profile' }
            ],
            'storage-s3': [
                { path: '/ext/storage/upload', description: 'Upload files' },
                { path: '/ext/storage/download', description: 'Download files' },
                { path: '/ext/storage/delete', description: 'Delete files' },
                { path: '/ext/storage/list', description: 'List files in bucket' },
                { path: '/ext/storage/signed-url', description: 'Generate signed URL' }
            ],
            'analytics': [
                { path: '/ext/analytics/track', description: 'Track custom event' },
                { path: '/ext/analytics/pageview', description: 'Track page view' },
                { path: '/ext/analytics/report', description: 'Get analytics report' },
                { path: '/ext/analytics/export', description: 'Export analytics data' },
                { path: '/ext/analytics/dashboard', description: 'View dashboard' }
            ],
            'email-smtp': [
                { path: '/ext/email/send', description: 'Send email' },
                { path: '/ext/email/queue', description: 'Queue bulk emails' },
                { path: '/ext/email/status', description: 'Check email status' },
                { path: '/ext/email/templates', description: 'Manage templates' },
                { path: '/ext/email/logs', description: 'View email logs' }
            ],
            'payment-stripe': [
                { path: '/ext/payment/checkout', description: 'Create checkout session' },
                { path: '/ext/payment/subscribe', description: 'Create subscription' },
                { path: '/ext/payment/cancel', description: 'Cancel subscription' },
                { path: '/ext/payment/portal', description: 'Customer portal' },
                { path: '/ext/payment/webhooks', description: 'Handle Stripe webhooks' }
            ],
            'webhooks': [
                { path: '/ext/webhooks/list', description: 'List all webhooks' },
                { path: '/ext/webhooks/create', description: 'Create new webhook' },
                { path: '/ext/webhooks/test', description: 'Test webhook delivery' },
                { path: '/ext/webhooks/logs', description: 'View delivery logs' },
                { path: '/ext/webhooks/delete', description: 'Delete webhook' }
            ]
        };
        
        return endpoints[extensionId] || [
            { path: `/ext/${extensionId}/api`, description: 'Main API endpoint' },
            { path: `/ext/${extensionId}/config`, description: 'Configuration endpoint' },
            { path: `/ext/${extensionId}/health`, description: 'Health check' },
            { path: `/ext/${extensionId}/status`, description: 'Status endpoint' }
        ];
    }

    // Get permissions for extension
    function getExtensionPermissions(extensionId) {
        const permissions = {
            'auth-oauth': [
                'auth.users.read',
                'auth.users.write',
                'auth.sessions.manage',
                'auth.tokens.create'
            ],
            'storage-s3': [
                'storage.files.read',
                'storage.files.write',
                'storage.files.delete',
                'storage.buckets.manage'
            ],
            'analytics': [
                'analytics.events.write',
                'analytics.reports.read',
                'database.read',
                'users.profile.read'
            ],
            'email-smtp': [
                'email.send',
                'email.templates.read',
                'email.templates.write',
                'queue.write'
            ],
            'payment-stripe': [
                'payment.process',
                'payment.subscriptions.manage',
                'users.billing.read',
                'users.billing.write'
            ],
            'search-elastic': [
                'search.index.write',
                'search.query.execute',
                'database.read'
            ],
            'cache-redis': [
                'cache.read',
                'cache.write',
                'cache.delete',
                'pubsub.publish'
            ],
            'queue-jobs': [
                'queue.jobs.create',
                'queue.jobs.manage',
                'workers.control'
            ],
            'media-processing': [
                'media.process',
                'storage.files.read',
                'storage.files.write'
            ],
            'webhooks': [
                'webhooks.create',
                'webhooks.manage',
                'webhooks.execute',
                'logs.write'
            ]
        };
        
        return permissions[extensionId] || [
            'database.read',
            'api.access'
        ];
    }

    // Close modal when clicking outside
    window.addEventListener('click', function(event) {
        const modal = document.getElementById('extensionDetailsModal');
        if (event.target === modal || event.target.classList.contains('modal-overlay')) {
            closeExtensionDetails();
        }
    });

    // Initialize on DOMContentLoaded
    document.addEventListener('DOMContentLoaded', function() {
        if (window.location.pathname === '/extensions') {
            window.initializeExtensionsPage();
        }
    });

    // Initialize on HTMX navigation
    document.addEventListener('htmx:afterSwap', function(event) {
        if (window.location.pathname === '/extensions') {
            setTimeout(window.initializeExtensionsPage, 100);
        }
    });

})(); // End of IIFE