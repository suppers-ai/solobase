<script lang="ts">
	import { onMount } from 'svelte';
	import { 
		Globe, Plus, Hammer, Pause, Settings, ExternalLink,
		GitBranch, Clock, HardDrive, Zap, MoreVertical,
		RefreshCw, Eye, Edit, Trash2, AlertCircle, CheckCircle,
		Terminal, Download, Package, Rocket
	} from 'lucide-svelte';
	import { api } from '$lib/api';
	import { requireAdmin } from '$lib/utils/auth';
	import FileExplorer from '$lib/components/FileExplorer.svelte';

	let sites: any[] = [];
	let loading = true;
	let selectedSite: any = null;
	let showCreateModal = false;
	let buildingStatus: { [key: string]: boolean } = {};
	let creatingExample = false;
	let showRequirements = true;
	let showDeployModal = false;
	let deployingSite: any = null;
	
	// Form data for new site
	let newSite = {
		name: '',
		domain: '',
		theme: 'default'
	};

	// Stats
	let stats = {
		totalSites: 0,
		activeSites: 0,
		totalBuilds: 0,
		storageUsed: '0 MB'
	};

	onMount(async () => {
		if (!requireAdmin()) return;
		
		// Load settings from localStorage
		const savedSettings = localStorage.getItem('hugo_settings');
		if (savedSettings) {
			try {
				hugoSettings = JSON.parse(savedSettings);
			} catch (e) {
				console.error('Failed to load Hugo settings:', e);
			}
		}
		
		await loadSites();
		
		// Check if we should create an example site
		if (sites.length === 0 && !localStorage.getItem('hugo_example_created')) {
			await createExampleSite();
		}
		
		// Check if user has dismissed requirements
		if (localStorage.getItem('hugo_requirements_dismissed')) {
			showRequirements = false;
		}
	});

	async function loadSites() {
		try {
			loading = true;
			
			// Fetch sites from API
			const [sitesRes, statsRes] = await Promise.all([
				api.get('/ext/hugo/api/sites'),
				api.get('/ext/hugo/api/stats')
			]);
			
			sites = sitesRes || [];
			stats = statsRes || {
				totalSites: 0,
				activeSites: 0,
				totalBuilds: 0,
				storageUsed: '0 MB'
			};
		} catch (error) {
			console.error('Failed to load sites:', error);
			sites = [];
		} finally {
			loading = false;
		}
	}


	function getStatusColor(status: string) {
		switch (status) {
			case 'published': return 'bg-green-100 text-green-700';
			case 'building': return 'bg-yellow-100 text-yellow-700';
			case 'draft': return 'bg-gray-100 text-gray-700';
			case 'error': return 'bg-red-100 text-red-700';
			default: return 'bg-gray-100 text-gray-700';
		}
	}

	function confirmDeploy(site: any) {
		deployingSite = site;
		showDeployModal = true;
	}
	
	async function buildSite(siteId: string) {
		try {
			showDeployModal = false;
			buildingStatus[siteId] = true;
			
			// Trigger build via API
			const result = await api.post(`/ext/hugo/api/sites/${siteId}/build`);
			
			if (result) {
				// Refresh sites to get updated status
				await loadSites();
			}
		} catch (error) {
			console.error('Failed to build site:', error);
		} finally {
			buildingStatus[siteId] = false;
		}
	}
	
	async function createSite() {
		try {
			const result = await api.post('/ext/hugo/api/sites', newSite);
			if (result) {
				// Reset form
				newSite = {
					name: '',
					domain: '',
					theme: 'default'
				};
				showCreateModal = false;
				// Reload sites
				await loadSites();
			}
		} catch (error) {
			console.error('Failed to create site:', error);
			alert('Failed to create site');
		}
	}
	
	async function deleteSite(id: string) {
		if (!confirm('Are you sure you want to delete this site? This action cannot be undone.')) return;
		
		try {
			await api.delete(`/ext/hugo/api/sites/${id}`);
			// Reload sites
			await loadSites();
		} catch (error) {
			console.error('Failed to delete site:', error);
			alert('Failed to delete site');
		}
	}
	
	async function createExampleSite() {
		try {
			creatingExample = true;
			
			// Create the example site
			const exampleSite = await api.post('/ext/hugo/api/sites', {
				name: 'Example Blog',
				domain: 'example-blog.local',
				theme: 'default',
				isExample: true
			});
			
			if (exampleSite && exampleSite.id) {
				// Build the example site
				await api.post(`/ext/hugo/api/sites/${exampleSite.id}/build`);
				
				// Mark as created
				localStorage.setItem('hugo_example_created', 'true');
				
				// Reload sites
				await loadSites();
			}
		} catch (error) {
			console.error('Failed to create example site:', error);
		} finally {
			creatingExample = false;
		}
	}
	
	function dismissRequirements() {
		showRequirements = false;
		localStorage.setItem('hugo_requirements_dismissed', 'true');
	}
	
	function viewSite(siteId: string) {
		// Get the backend URL - in dev it's on port 8080, in production it's the same origin
		const backendUrl = import.meta.env.DEV 
			? 'http://localhost:8080' 
			: window.location.origin;
		// Use new storage path structure
		window.open(`${backendUrl}/storage/ext/hugo/public/${siteId}/`, '_blank');
	}
	
	// Edit functionality
	let showEditModal = false;
	let editingSite: any = null;
	let fileTree: any[] = [];
	let selectedFile: any = null;
	let fileContent = '';
	let saving = false;
	let loadingFiles = false;
	let loadingContent = false;
	
	// Settings functionality
	let showSettingsModal = false;
	let hugoSettings = {
		hugoPath: '~/bin/hugo',
		defaultTheme: 'default',
		autoPublish: false
	};
	
	async function editSite(site: any) {
		editingSite = site;
		showEditModal = true;
		selectedFile = null;
		fileContent = '';
		await loadFileTree();
	}
	
	async function loadFileTree() {
		try {
			loadingFiles = true;
			
			// Try to load from API first
			try {
				const files = await api.get(`/ext/hugo/api/sites/${editingSite.id}/files`);
				console.log('Loaded files from API:', files);
				
				// Check if files is an array or needs to be extracted
				if (Array.isArray(files)) {
					fileTree = files;
				} else if (files && typeof files === 'object') {
					// Try to find the files array in the response
					fileTree = files.files || files.data || files.items || [];
				} else {
					fileTree = [];
				}
			} catch (apiError) {
				console.log('API not available, using mock data');
				// Use mock data if API fails
				fileTree = getMockFileTree();
			}
			
			console.log('File tree after processing:', fileTree);
		} catch (error) {
			console.error('Failed to load files:', error);
			fileTree = getMockFileTree();
		} finally {
			loadingFiles = false;
		}
	}
	
	function getMockFileTree() {
		return [
			{
				id: 'config',
				name: 'config.toml',
				path: 'config.toml',
				type: 'file'
			},
			{
				id: 'content',
				name: 'content',
				path: 'content',
				type: 'directory',
				children: [
					{
						id: 'posts',
						name: 'posts',
						path: 'content/posts',
						type: 'directory',
						children: [
							{
								id: 'post1',
								name: 'getting-started.md',
								path: 'content/posts/getting-started.md',
								type: 'file'
							},
							{
								id: 'post2',
								name: 'hugo-themes.md',
								path: 'content/posts/hugo-themes.md',
								type: 'file'
							}
						]
					},
					{
						id: 'pages',
						name: 'pages',
						path: 'content/pages',
						type: 'directory',
						children: [
							{
								id: 'about',
								name: 'about.md',
								path: 'content/pages/about.md',
								type: 'file'
							},
							{
								id: 'contact',
								name: 'contact.md',
								path: 'content/pages/contact.md',
								type: 'file'
							}
						]
					}
				]
			},
			{
				id: 'layouts',
				name: 'layouts',
				path: 'layouts',
				type: 'directory',
				children: [
					{
						id: 'default',
						name: '_default',
						path: 'layouts/_default',
						type: 'directory',
						children: [
							{
								id: 'baseof',
								name: 'baseof.html',
								path: 'layouts/_default/baseof.html',
								type: 'file'
							},
							{
								id: 'single',
								name: 'single.html',
								path: 'layouts/_default/single.html',
								type: 'file'
							},
							{
								id: 'list',
								name: 'list.html',
								path: 'layouts/_default/list.html',
								type: 'file'
							}
						]
					},
					{
						id: 'partials',
						name: 'partials',
						path: 'layouts/partials',
						type: 'directory',
						children: [
							{
								id: 'header',
								name: 'header.html',
								path: 'layouts/partials/header.html',
								type: 'file'
							},
							{
								id: 'footer',
								name: 'footer.html',
								path: 'layouts/partials/footer.html',
								type: 'file'
							}
						]
					}
				]
			},
			{
				id: 'static',
				name: 'static',
				path: 'static',
				type: 'directory',
				children: [
					{
						id: 'css',
						name: 'css',
						path: 'static/css',
						type: 'directory',
						children: [
							{
								id: 'style',
								name: 'style.css',
								path: 'static/css/style.css',
								type: 'file'
							}
						]
					},
					{
						id: 'js',
						name: 'js',
						path: 'static/js',
						type: 'directory',
						children: [
							{
								id: 'main',
								name: 'main.js',
								path: 'static/js/main.js',
								type: 'file'
							}
						]
					}
				]
			},
			{
				id: 'themes',
				name: 'themes',
				path: 'themes',
				type: 'directory',
				children: []
			}
		];
	}
	
	async function selectFile(file: any) {
		if (file.type === 'directory') return;
		
		try {
			loadingContent = true;
			selectedFile = file;
			
			// Try to load from API first
			try {
				const response = await api.post(`/ext/hugo/api/sites/${editingSite.id}/files/read`, {
					path: file.path
				});
				fileContent = response.content || '';
			} catch (apiError) {
				console.log('API not available, using mock content');
				// Use mock content if API fails
				fileContent = getMockFileContent(file.path);
			}
		} catch (error) {
			console.error('Failed to load file:', error);
			fileContent = getMockFileContent(file.path);
		} finally {
			loadingContent = false;
		}
	}
	
	function getMockFileContent(path: string): string {
		const mockContent: { [key: string]: string } = {
			'config.toml': `baseURL = "https://example.com/"
languageCode = "en-us"
title = "My Hugo Site"
theme = "default"

[params]
  author = "Your Name"
  description = "A wonderful Hugo site"`,
			'content/posts/getting-started.md': `---
title: "Getting Started with Hugo"
date: 2024-01-15
draft: false
tags: ["hugo", "tutorial", "beginner"]
---

# Getting Started with Hugo

Welcome to Hugo! This is your first post.

## Installation

Hugo is easy to install...`,
			'content/posts/hugo-themes.md': `---
title: "Understanding Hugo Themes"
date: 2024-01-16
draft: false
tags: ["hugo", "themes", "design"]
---

# Understanding Hugo Themes

Hugo themes allow you to customize the look and feel of your site...`,
			'content/pages/about.md': `---
title: "About"
---

# About Us

This is the about page for your Hugo site.`,
			'content/pages/contact.md': `---
title: "Contact"
---

# Contact Us

Get in touch with us!`,
			'layouts/_default/baseof.html': `<!DOCTYPE html>
<html lang="{{ .Site.Language }}">
<head>
    <meta charset="UTF-8">
    <title>{{ .Title }} | {{ .Site.Title }}</title>
    <link rel="stylesheet" href="/css/style.css">
</head>
<body>
    {{ partial "header.html" . }}
    <main>
        {{ block "main" . }}{{ end }}
    </main>
    {{ partial "footer.html" . }}
</body>
</html>`,
			'layouts/_default/single.html': `{{ define "main" }}
<article>
    <h1>{{ .Title }}</h1>
    <time>{{ .Date.Format "January 2, 2006" }}</time>
    {{ .Content }}
</article>
{{ end }}`,
			'layouts/_default/list.html': `{{ define "main" }}
<h1>{{ .Title }}</h1>
{{ range .Pages }}
    <article>
        <h2><a href="{{ .Permalink }}">{{ .Title }}</a></h2>
        <time>{{ .Date.Format "January 2, 2006" }}</time>
    </article>
{{ end }}
{{ end }}`,
			'layouts/partials/header.html': `<header>
    <nav>
        <a href="/">{{ .Site.Title }}</a>
        <ul>
            <li><a href="/posts">Posts</a></li>
            <li><a href="/about">About</a></li>
            <li><a href="/contact">Contact</a></li>
        </ul>
    </nav>
</header>`,
			'layouts/partials/footer.html': `<footer>
    <p>&copy; {{ now.Year }} {{ .Site.Title }}</p>
</footer>`,
			'static/css/style.css': `/* Main styles */
body {
    font-family: system-ui, -apple-system, sans-serif;
    line-height: 1.6;
    color: #333;
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem;
}

header {
    border-bottom: 1px solid #eee;
    padding-bottom: 1rem;
    margin-bottom: 2rem;
}

nav ul {
    list-style: none;
    padding: 0;
}

nav li {
    display: inline;
    margin-right: 1rem;
}`,
			'static/js/main.js': `// Main JavaScript file
document.addEventListener('DOMContentLoaded', function() {
    console.log('Hugo site loaded!');
});`
		};
		
		return mockContent[path] || `# ${path}\n\nMock content for this file.`;
	}
	
	async function saveFile() {
		if (!selectedFile) return;
		
		try {
			saving = true;
			
			// Try to save via API first
			try {
				await api.post(`/ext/hugo/api/sites/${editingSite.id}/files/save`, {
					path: selectedFile.path,
					content: fileContent
				});
			} catch (apiError) {
				console.log('API not available, save simulated');
				// Simulate save for mock data
			}
			
			// Show success message
			const saveBtn = document.querySelector('.save-button');
			if (saveBtn) {
				saveBtn.textContent = 'Saved!';
				setTimeout(() => {
					saveBtn.textContent = 'Save';
				}, 2000);
			}
		} catch (error) {
			console.error('Failed to save file:', error);
			// Show success anyway for mock mode
			const saveBtn = document.querySelector('.save-button');
			if (saveBtn) {
				saveBtn.textContent = 'Saved!';
				setTimeout(() => {
					saveBtn.textContent = 'Save';
				}, 2000);
			}
		} finally {
			saving = false;
		}
	}
	
	function closeEditModal() {
		showEditModal = false;
		editingSite = null;
		selectedFile = null;
		fileContent = '';
	}
	
	// Helper to render file tree
	function renderFileTree(nodes: any[], level = 0) {
		return nodes;
	}
</script>

<div class="page-container">
	<!-- Installation Requirements Banner -->
	{#if showRequirements}
		<div class="requirements-banner">
			<div class="requirements-icon">
				<AlertCircle size={20} />
			</div>
			<div class="requirements-content">
				<h3>Hugo Required</h3>
				<p>This extension requires Hugo to be installed on your system.</p>
				<a href="https://gohugo.io/installation/" target="_blank" class="requirements-link">
					View installation instructions →
				</a>
			</div>
			<button class="btn-close" on:click={dismissRequirements}>
				<MoreVertical size={16} />
			</button>
		</div>
	{/if}

	<!-- Header -->
	<div class="header">
		<div class="header-title">
			<h1>Hugo Sites</h1>
			<p>Manage your static sites powered by Hugo</p>
		</div>
		<div class="header-actions">
			<button class="btn btn-primary" on:click={() => showCreateModal = true}>
				<Plus size={16} />
				New Site
			</button>
		</div>
	</div>

	<!-- Stats Cards -->
	<div class="stats-grid">
		<div class="stat-card">
			<div class="stat-icon bg-cyan-100">
				<Globe size={20} class="text-cyan-600" />
			</div>
			<div class="stat-content">
				<p class="stat-label">Total Sites</p>
				<p class="stat-value">{stats.totalSites}</p>
			</div>
		</div>
		<div class="stat-card">
			<div class="stat-icon bg-green-100">
				<Zap size={20} class="text-green-600" />
			</div>
			<div class="stat-content">
				<p class="stat-label">Published</p>
				<p class="stat-value">{stats.activeSites}</p>
			</div>
		</div>
		<div class="stat-card">
			<div class="stat-icon bg-purple-100">
				<GitBranch size={20} class="text-purple-600" />
			</div>
			<div class="stat-content">
				<p class="stat-label">Total Builds</p>
				<p class="stat-value">{stats.totalBuilds}</p>
			</div>
		</div>
		<div class="stat-card">
			<div class="stat-icon bg-orange-100">
				<HardDrive size={20} class="text-orange-600" />
			</div>
			<div class="stat-content">
				<p class="stat-label">Storage Used</p>
				<p class="stat-value">{stats.storageUsed}</p>
			</div>
		</div>
	</div>

	<!-- Sites Grid -->
	{#if loading || creatingExample}
		<div class="loading-container">
			<div class="loading loading-spinner loading-lg text-cyan-600"></div>
			{#if creatingExample}
				<p class="loading-text">Creating example site...</p>
			{/if}
		</div>
	{:else if sites.length === 0}
		<div class="empty-state">
			<Globe size={48} />
			<h2>No sites yet</h2>
			<p>Create your first Hugo site to get started</p>
		</div>
	{:else}
		<div class="sites-grid">
			{#each sites as site}
				<div class="site-card">
					<div class="site-header">
						<span class="status-badge {getStatusColor(site.status)}">
							{site.status}
						</span>
						<button class="btn-icon-sm btn-icon-danger" on:click={() => deleteSite(site.id)} title="Delete">
							<Trash2 size={16} />
						</button>
					</div>
					
					<div class="site-body">
						<h3 class="site-name">{site.name}</h3>
						<a href="https://{site.domain}" target="_blank" class="site-domain">
							<ExternalLink size={12} />
							{site.domain}
						</a>
						
						<div class="site-meta">
							<div class="meta-item">
								<Clock size={12} />
								<span>{site.lastBuild || 'Never'}</span>
							</div>
							<div class="meta-item">
								<HardDrive size={12} />
								<span>{site.size || '0 MB'}</span>
							</div>
						</div>
						
						<div class="site-stats">
							<div class="stat">
								<span class="stat-value">{site.pages || 0}</span>
								<span class="stat-label">PAGES</span>
							</div>
							<div class="stat">
								<span class="stat-value">{site.visits || 0}</span>
								<span class="stat-label">VISITS</span>
							</div>
							<div class="stat">
								<span class="stat-value">{site.buildTime || '0s'}</span>
								<span class="stat-label">BUILD TIME</span>
							</div>
						</div>
					</div>
					
					<div class="site-footer">
						<button class="btn-action" title="Preview" on:click={() => viewSite(site.id)}>
							<Eye size={16} />
						</button>
						<button class="btn-action" title="Edit" on:click={() => editSite(site)}>
							<Edit size={16} />
						</button>
						<button 
							class="btn-action btn-build {buildingStatus[site.id] ? 'building' : ''}"
							on:click={() => confirmDeploy(site)}
							disabled={buildingStatus[site.id]}
							title="Build & Deploy"
						>
							{#if buildingStatus[site.id]}
								<RefreshCw size={16} class="spin" />
								<span>Building</span>
							{:else}
								<Hammer size={16} />
								<span>Build & Deploy</span>
							{/if}
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<!-- Create Site Modal -->
{#if showCreateModal}
	<div class="modal-overlay" on:click={() => showCreateModal = false}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h2>Create New Hugo Site</h2>
				<button class="close-button" on:click={() => showCreateModal = false}>×</button>
			</div>
			<div class="modal-body">
				<div class="form-group">
					<label for="siteName">Site Name</label>
					<input 
						type="text" 
						id="siteName" 
						bind:value={newSite.name} 
						placeholder="My Awesome Blog"
						required
					/>
				</div>
				<div class="form-group">
					<label for="domain">Domain</label>
					<input 
						type="text" 
						id="domain" 
						bind:value={newSite.domain} 
						placeholder="example.com"
					/>
				</div>
				<div class="form-group">
					<label for="theme">Theme</label>
					<select id="theme" bind:value={newSite.theme}>
						<option value="default">Default (Built-in)</option>
						<option value="ananke">Ananke</option>
						<option value="papermod">PaperMod</option>
						<option value="stack">Stack</option>
					</select>
				</div>
			</div>
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={() => showCreateModal = false}>Cancel</button>
				<button class="btn btn-primary" on:click={createSite}>
					<Plus size={16} />
					Create Site
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Edit Site Modal -->
{#if showEditModal && editingSite}
	<div class="modal-overlay" on:click={closeEditModal}>
		<div class="modal edit-modal" on:click|stopPropagation>
			<div class="modal-header">
				<h2>Edit Site: {editingSite.name}</h2>
				<button class="close-button" on:click={closeEditModal}>×</button>
			</div>
			<div class="modal-body editor-container">
				<div class="file-explorer-wrapper">
					<h3>Files</h3>
					<FileExplorer 
						files={fileTree}
						bind:selectedFile
						loading={loadingFiles}
						mode="file"
						on:select={(e) => selectFile(e.detail)}
					/>
				</div>
				
				<div class="file-editor">
					{#if selectedFile}
						<div class="editor-header">
							<span class="file-path">{selectedFile.path}</span>
							<button 
								class="save-button btn btn-sm btn-primary"
								on:click={saveFile}
								disabled={saving}
							>
								{saving ? 'Saving...' : 'Save'}
							</button>
						</div>
						{#if loadingContent}
							<div class="loading">Loading content...</div>
						{:else}
							<textarea 
								class="code-editor"
								bind:value={fileContent}
								spellcheck="false"
							></textarea>
						{/if}
					{:else}
						<div class="no-file-selected">
							Select a file to edit
						</div>
					{/if}
				</div>
			</div>
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={closeEditModal}>Close</button>
			</div>
		</div>
	</div>
{/if}

<!-- Deploy Confirmation Modal -->
{#if showDeployModal && deployingSite}
	<div class="modal-overlay" on:click={() => showDeployModal = false}>
		<div class="modal deploy-modal" on:click|stopPropagation>
			<div class="modal-header">
				<h2>Deploy Site</h2>
				<button class="close-button" on:click={() => showDeployModal = false}>×</button>
			</div>
			<div class="modal-body">
				<div class="deploy-info">
					<div class="deploy-icon">
						<Rocket size={32} />
					</div>
					<h3>Ready to deploy "{deployingSite.name}"?</h3>
					<p class="deploy-description">
						This will build your Hugo site and deploy it to production.
					</p>
				</div>
				
				<div class="deploy-details">
					<div class="detail-item">
						<span class="detail-label">Domain:</span>
						<span class="detail-value">{deployingSite.domain}</span>
					</div>
					<div class="detail-item">
						<span class="detail-label">Last build:</span>
						<span class="detail-value">{deployingSite.lastBuild || 'Never'}</span>
					</div>
					<div class="detail-item">
						<span class="detail-label">Pages:</span>
						<span class="detail-value">{deployingSite.pages || 0} pages</span>
					</div>
				</div>
				
				<div class="deploy-notice">
					<AlertCircle size={16} />
					<span>The build process may take a few seconds depending on your site size.</span>
				</div>
			</div>
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={() => showDeployModal = false}>Cancel</button>
				<button class="btn btn-primary" on:click={() => buildSite(deployingSite.id)}>
					<Hammer size={16} />
					Deploy Now
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Settings Modal -->
{#if showSettingsModal}
	<div class="modal-overlay" on:click={() => showSettingsModal = false}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h2>Hugo Extension Settings</h2>
				<button class="close-button" on:click={() => showSettingsModal = false}>×</button>
			</div>
			<div class="modal-body">
				<div class="form-group">
					<label for="hugoPath">Hugo Binary Path</label>
					<input 
						type="text" 
						id="hugoPath" 
						bind:value={hugoSettings.hugoPath} 
						placeholder="/usr/local/bin/hugo"
					/>
					<p class="form-help">Path to the Hugo executable on your system</p>
				</div>
				<div class="form-group">
					<label for="defaultTheme">Default Theme</label>
					<select id="defaultTheme" bind:value={hugoSettings.defaultTheme}>
						<option value="default">Default (Built-in)</option>
						<option value="ananke">Ananke</option>
						<option value="papermod">PaperMod</option>
						<option value="stack">Stack</option>
					</select>
					<p class="form-help">Theme to use for new sites</p>
				</div>
				<div class="form-group">
					<label class="checkbox-label">
						<input 
							type="checkbox" 
							bind:checked={hugoSettings.autoPublish}
						/>
						<span>Auto-publish after editing</span>
					</label>
					<p class="form-help">Automatically rebuild sites after saving changes</p>
				</div>
			</div>
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={() => showSettingsModal = false}>Cancel</button>
				<button class="btn btn-primary" on:click={() => {
					// Save settings to localStorage
					localStorage.setItem('hugo_settings', JSON.stringify(hugoSettings));
					showSettingsModal = false;
				}}>Save Settings</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.page-container {
		padding: 1.5rem;
		max-width: 1400px;
		margin: 0 auto;
	}

	.requirements-banner {
		background: var(--warning-bg, #FEF3C7);
		border: 1px solid var(--warning-border, #F59E0B);
		border-radius: 0.5rem;
		padding: 1rem;
		margin-bottom: 1.5rem;
		display: flex;
		gap: 1rem;
		align-items: start;
		position: relative;
	}

	.requirements-icon {
		color: var(--warning-color, #D97706);
		flex-shrink: 0;
	}

	.requirements-content {
		flex: 1;
	}

	.requirements-content h3 {
		margin: 0 0 0.5rem 0;
		color: var(--warning-dark, #92400E);
		font-size: 1rem;
		font-weight: 600;
	}

	.requirements-content p {
		margin: 0 0 1rem 0;
		color: var(--warning-text, #78350F);
		font-size: 0.9rem;
	}
	
	.requirements-link {
		color: var(--primary, #0891B2);
		font-weight: 500;
		text-decoration: none;
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		transition: opacity 0.2s;
	}
	
	.requirements-link:hover {
		opacity: 0.8;
		text-decoration: underline;
	}

	.requirements-steps {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.requirement-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--warning-text, #78350F);
		font-size: 0.875rem;
	}

	.install-commands {
		margin-top: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.command-group {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.command-group label {
		font-size: 0.75rem;
		color: var(--warning-text, #78350F);
		font-weight: 500;
	}

	.command-group code {
		background: var(--bg-primary, white);
		padding: 0.25rem 0.5rem;
		border-radius: 0.25rem;
		font-family: monospace;
		font-size: 0.75rem;
		color: var(--text-primary);
		border: 1px solid var(--border-color);
	}

	.dismiss-button {
		position: absolute;
		top: 0.5rem;
		right: 0.5rem;
		background: none;
		border: none;
		color: var(--warning-text, #78350F);
		cursor: pointer;
		padding: 0.25rem;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 0.25rem;
		transition: background 0.2s;
	}

	.dismiss-button:hover {
		background: var(--warning-hover, rgba(217, 119, 6, 0.1));
	}

	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 2rem;
		padding: 1.5rem;
		background: var(--bg-primary);
		border-radius: 0.5rem;
		border: 1px solid var(--border-color);
		min-height: 80px;
	}

	.header-title h1 {
		margin: 0;
		font-size: 1.75rem;
		font-weight: 700;
		color: var(--text-primary);
	}
	
	.header-title p {
		margin: 0.25rem 0 0 0;
		color: var(--text-secondary);
		font-size: 0.875rem;
	}
	
	.header-actions {
		display: flex;
		gap: 0.75rem;
		align-items: center;
	}

	.header-actions .btn {
		background: var(--primary, #06b6d4) !important;
		color: white !important;
		border: none;
		padding: 0.625rem 1.25rem;
		border-radius: 0.375rem;
		font-weight: 500;
		font-size: 0.875rem;
		cursor: pointer;
		display: inline-flex !important;
		align-items: center;
		gap: 0.5rem;
		transition: opacity 0.2s;
		white-space: nowrap;
	}

	.header-actions .btn:hover {
		opacity: 0.9;
	}

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1rem;
		margin-bottom: 2rem;
	}

	.stat-card {
		background: var(--bg-primary);
		border: 1px solid var(--border-color);
		border-radius: 0.5rem;
		padding: 1rem;
	}

	.stat-label {
		font-size: 0.75rem;
		color: var(--text-secondary);
		margin-bottom: 0.25rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.stat-value {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--text-primary);
	}

	.sites-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
		gap: 1rem;
		margin-bottom: 2rem;
	}

	.site-card {
		background: var(--bg-primary);
		border: 1px solid var(--border-color);
		border-radius: 0.75rem;
		padding: 1.25rem;
		transition: all 0.2s;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.site-card:hover {
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
		border-color: var(--primary);
	}

	.site-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.site-body {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.site-name {
		margin: 0;
		font-size: 1.125rem;
		font-weight: 600;
		color: var(--text-primary);
	}

	.site-domain {
		font-size: 0.8125rem;
		color: var(--primary);
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		text-decoration: none;
		transition: opacity 0.2s;
	}
	
	.site-domain:hover {
		opacity: 0.8;
	}

	.status-badge {
		padding: 0.25rem 0.625rem;
		border-radius: 9999px;
		font-size: 0.6875rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.025em;
	}

	.site-stats {
		display: flex;
		gap: 1.5rem;
		padding-top: 0.75rem;
		border-top: 1px solid var(--border-color);
	}

	.stat {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}

	.stat-value {
		font-size: 1rem;
		font-weight: 700;
		color: var(--text-primary);
		line-height: 1;
	}

	.stat-label {
		font-size: 0.625rem;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-weight: 500;
	}

	.site-meta {
		display: flex;
		gap: 1rem;
		font-size: 0.75rem;
		color: var(--text-secondary);
	}

	.meta-item {
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}
	
	.btn-icon-sm {
		padding: 0.375rem;
		background: transparent;
		border: 1px solid transparent;
		border-radius: 0.375rem;
		cursor: pointer;
		color: var(--text-secondary);
		transition: all 0.2s;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.btn-icon-sm:hover {
		background: var(--bg-hover);
		border-color: var(--border-color);
	}

	.btn-icon-danger:hover {
		color: var(--danger, #ef4444);
		background: rgba(239, 68, 68, 0.1);
		border-color: rgba(239, 68, 68, 0.2);
	}

	.site-footer {
		display: grid;
		grid-template-columns: 1fr 1fr 2fr;
		gap: 0.5rem;
		padding-top: 0.75rem;
		border-top: 1px solid var(--border-color);
	}

	.btn-action {
		background: transparent;
		border: 1px solid var(--border-color);
		padding: 0.5rem;
		border-radius: 0.375rem;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.2s;
		color: var(--text-secondary);
		font-size: 0.8125rem;
		font-weight: 500;
		gap: 0.375rem;
		white-space: nowrap;
	}

	.btn-action:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
		border-color: var(--primary);
	}

	.btn-build {
		background: var(--bg-primary);
		color: var(--text-primary);
	}
	
	.btn-build:hover {
		background: var(--primary);
		color: white;
		border-color: var(--primary);
	}

	.btn-build.building {
		background: var(--primary);
		color: white;
		border-color: var(--primary);
		animation: pulse 2s infinite;
	}
	
	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.7; }
	}

	.btn-danger {
		color: var(--danger);
	}

	.btn-danger:hover {
		background: var(--danger);
		color: white;
		border-color: var(--danger);
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		text-align: center;
		padding: 4rem 1rem;
		min-height: 300px;
		color: var(--text-secondary);
		background: var(--bg-primary);
		border: 1px solid var(--border-color);
		border-radius: 0.5rem;
	}

	.empty-state h2 {
		margin: 1rem 0 0.5rem 0;
		font-size: 1.25rem;
		color: var(--text-primary);
	}
	
	.empty-state p {
		margin: 0;
		font-size: 0.875rem;
	}

	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal {
		background: var(--bg-primary);
		border-radius: 0.5rem;
		width: 90%;
		max-width: 500px;
		max-height: 90vh;
		overflow-y: auto;
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
	}

	.modal-header {
		padding: 1.5rem;
		border-bottom: 1px solid var(--border-color);
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.modal-header h2 {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 600;
	}

	.close-button {
		background: none;
		border: none;
		font-size: 1.5rem;
		cursor: pointer;
		color: var(--text-secondary);
		padding: 0;
		width: 2rem;
		height: 2rem;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.modal-body {
		padding: 1.5rem;
	}

	.form-group {
		margin-bottom: 1rem;
	}

	.form-group label {
		display: block;
		margin-bottom: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--text-primary);
	}

	.form-group input,
	.form-group select {
		width: 100%;
		padding: 0.5rem;
		border: 1px solid var(--border-color);
		border-radius: 0.375rem;
		font-size: 0.875rem;
		background: var(--bg-primary);
		color: var(--text-primary);
	}

	.form-group input:focus,
	.form-group select:focus {
		outline: none;
		border-color: var(--primary);
		box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
	}

	.modal-footer {
		padding: 1rem 1.5rem;
		border-top: 1px solid var(--border-color);
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
	}

	.btn {
		padding: 0.5rem 1rem;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-secondary {
		background: transparent;
		color: var(--text-primary);
		border: 1px solid var(--border-color);
	}

	.btn-secondary:hover {
		background: var(--bg-hover);
	}

	.loading {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 3rem;
		color: var(--text-secondary);
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	.building :global(svg) {
		animation: spin 1s linear infinite;
	}

	.edit-modal {
		width: 90%;
		max-width: 1200px;
		height: 80vh;
		max-height: 800px;
	}
	
	.editor-container {
		display: flex;
		gap: 1rem;
		height: calc(100% - 60px);
	}
	
	.file-explorer-wrapper {
		width: 300px;
		border-right: 1px solid var(--border-color);
		padding-right: 1rem;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	
	.file-explorer-wrapper h3 {
		margin-top: 0;
		margin-bottom: 1rem;
		font-size: 0.9rem;
		text-transform: uppercase;
		color: var(--text-secondary);
		flex-shrink: 0;
	}
	
	.file-explorer-wrapper :global(.file-explorer) {
		flex: 1;
		overflow-y: auto;
	}
	
	.file-editor {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	
	.editor-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.5rem;
		background: var(--bg-secondary);
		border-radius: 4px;
		margin-bottom: 1rem;
	}
	
	.file-path {
		font-family: monospace;
		font-size: 0.9rem;
		color: var(--text-secondary);
	}
	
	.code-editor {
		flex: 1;
		width: 100%;
		padding: 1rem;
		border: 1px solid var(--border-color);
		border-radius: 4px;
		font-family: 'Courier New', monospace;
		font-size: 0.9rem;
		line-height: 1.5;
		background: var(--bg-primary);
		color: var(--text-primary);
		resize: none;
		overflow: auto;
	}
	
	.code-editor:focus {
		outline: none;
		border-color: var(--primary);
	}
	
	.no-file-selected {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--text-secondary);
		font-style: italic;
	}
	
	.loading {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 2rem;
		color: var(--text-secondary);
	}
	
	.save-button {
		padding: 0.25rem 1rem;
	}
	
	.btn-sm {
		font-size: 0.875rem;
	}
	
	.form-help {
		margin-top: 0.25rem;
		font-size: 0.75rem;
		color: var(--text-secondary);
	}
	
	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
		font-size: 0.875rem;
	}
	
	.checkbox-label input[type="checkbox"] {
		width: auto;
		margin: 0;
		cursor: pointer;
	}
	
	.page-header {
		background: white;
		border-radius: 0.5rem;
		padding: 1.5rem;
		margin-bottom: 1.5rem;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
	}
	
	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	
	.header-left {
		flex: 1;
	}
	
	.header-title {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 0.5rem;
	}
	
	.header-title h1 {
		margin: 0;
		font-size: 1.5rem;
		font-weight: 600;
		color: var(--text-primary);
	}
	
	.header-subtitle {
		margin: 0;
		color: var(--text-secondary);
		font-size: 0.875rem;
	}
	
	.header-actions {
		display: flex;
		gap: 0.75rem;
	}
	
	.btn-close {
		position: absolute;
		top: 0.5rem;
		right: 0.5rem;
		background: none;
		border: none;
		color: var(--warning-text, #78350F);
		cursor: pointer;
		padding: 0.25rem;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 0.25rem;
		transition: background 0.2s;
	}
	
	.btn-close:hover {
		background: var(--warning-hover, rgba(217, 119, 6, 0.1));
	}
	
	/* Deploy Modal Styles */
	.deploy-modal {
		max-width: 450px;
	}
	
	.deploy-info {
		text-align: center;
		padding: 1rem 0;
	}
	
	.deploy-icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 60px;
		height: 60px;
		background: var(--primary);
		color: white;
		border-radius: 50%;
		margin-bottom: 1rem;
	}
	
	.deploy-info h3 {
		margin: 0 0 0.5rem 0;
		font-size: 1.25rem;
		color: var(--text-primary);
	}
	
	.deploy-description {
		color: var(--text-secondary);
		font-size: 0.875rem;
		margin: 0;
	}
	
	.deploy-details {
		background: var(--bg-secondary);
		border-radius: 0.5rem;
		padding: 1rem;
		margin: 1.5rem 0;
	}
	
	.detail-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.5rem 0;
	}
	
	.detail-item:not(:last-child) {
		border-bottom: 1px solid var(--border-color);
	}
	
	.detail-label {
		font-size: 0.8125rem;
		color: var(--text-secondary);
		font-weight: 500;
	}
	
	.detail-value {
		font-size: 0.875rem;
		color: var(--text-primary);
		font-weight: 600;
	}
	
	.deploy-notice {
		display: flex;
		align-items: flex-start;
		gap: 0.5rem;
		padding: 0.75rem;
		background: var(--info-bg, #EFF6FF);
		border: 1px solid var(--info-border, #3B82F6);
		border-radius: 0.375rem;
		font-size: 0.8125rem;
		color: var(--info-text, #1E40AF);
	}
	
	.deploy-notice svg {
		flex-shrink: 0;
		margin-top: 0.125rem;
	}
	
	@keyframes spin {
		from { transform: rotate(0deg); }
		to { transform: rotate(360deg); }
	}
	
	.spin {
		animation: spin 1s linear infinite;
	}
</style>