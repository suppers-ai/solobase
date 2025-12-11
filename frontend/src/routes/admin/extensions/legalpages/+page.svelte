<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import {
		FileText, Save, Eye, Globe, Clock,
		Bold, Italic, List, ListOrdered,
		Heading1, Heading2, Link
	} from 'lucide-svelte';
	import { currentUser, userRoles } from '$lib/stores/auth';
	import { api } from '$lib/api';
	import { Editor } from '@tiptap/core';
	import StarterKit from '@tiptap/starter-kit';
	import LinkExtension from '@tiptap/extension-link';

	// Check if user is admin
	$: if ($currentUser && (!$userRoles || !$userRoles.includes('admin'))) {
		goto('/profile');
	}

	let loading = true;
	let saving = false;
	let selectedTab: 'terms' | 'privacy' = 'terms';
	let showPreview = false;

	// Document data
	let termsTitle = 'Terms and Conditions';
	let privacyTitle = 'Privacy Policy';
	let termsVersion = '';
	let privacyVersion = '';

	// Editors
	let termsEditor: Editor;
	let privacyEditor: Editor;
	let termsElement: HTMLElement;
	let privacyElement: HTMLElement;

	// Success/error messages
	let message = '';
	let messageType: 'success' | 'error' | '' = '';

	// Reactive bindings
	$: currentTitle = selectedTab === 'terms' ? termsTitle : privacyTitle;
	$: currentVersion = selectedTab === 'terms' ? termsVersion : privacyVersion;

	function updateTitle(value: string) {
		if (selectedTab === 'terms') {
			termsTitle = value;
		} else {
			privacyTitle = value;
		}
	}

	onMount(async () => {
		loading = false;

		// Wait for DOM to be ready
		await new Promise(resolve => setTimeout(resolve, 100));

		// Initialize TipTap editors only after elements are in DOM
		if (termsElement && privacyElement) {
			// Initialize terms editor
			termsEditor = new Editor({
				element: termsElement,
				extensions: [
					StarterKit,
					LinkExtension.configure({
						openOnClick: false,
						HTMLAttributes: {
							class: 'text-blue-600 underline cursor-pointer'
						}
					})
				],
				content: '<p>Loading terms and conditions...</p>',
				editorProps: {
					attributes: {
						class: 'prose prose-sm max-w-none focus:outline-none p-4'
					}
				},
				onCreate: ({ editor }) => {
					console.log('Terms editor created', editor);
				},
				onUpdate: ({ editor }) => {
					console.log('Terms editor updated');
				}
			});

			// Initialize privacy editor
			privacyEditor = new Editor({
				element: privacyElement,
				extensions: [
					StarterKit,
					LinkExtension.configure({
						openOnClick: false,
						HTMLAttributes: {
							class: 'text-blue-600 underline cursor-pointer'
						}
					})
				],
				content: '<p>Loading privacy policy...</p>',
				editorProps: {
					attributes: {
						class: 'prose prose-sm max-w-none focus:outline-none p-4'
					}
				},
				onCreate: ({ editor }) => {
					console.log('Privacy editor created', editor);
				},
				onUpdate: ({ editor }) => {
					console.log('Privacy editor updated');
				}
			});

			// Load existing documents after editors are ready
			await loadDocuments();
		} else {
			console.error('Editor elements not found');
		}
	});

	onDestroy(() => {
		if (termsEditor) {
			termsEditor.destroy();
		}
		if (privacyEditor) {
			privacyEditor.destroy();
		}
	});

	async function loadDocuments() {
		try {
			// Token is stored in httpOnly cookie
			const headers = {
				'Content-Type': 'application/json'
			};

			// Load terms
			const termsResponse = await fetch('/api/admin/ext/legalpages/documents/terms', {
				headers
			});
			if (termsResponse.ok) {
				const termsDoc = await termsResponse.json();
				if (termsDoc && termsDoc.content) {
					if (termsEditor) {
						termsEditor.commands.setContent(termsDoc.content);
					}
					termsTitle = termsDoc.title || 'Terms and Conditions';
					termsVersion = termsDoc.status === 'published'
						? `Version ${termsDoc.version} - Published`
						: `Version ${termsDoc.version} - Draft`;
				} else {
					const defaultContent = '<p>Enter your terms and conditions here...</p>';
					if (termsEditor) {
						termsEditor.commands.setContent(defaultContent);
					}
					termsVersion = 'No saved versions yet';
				}
			} else if (termsResponse.status === 404) {
				// Document doesn't exist yet
				const defaultContent = '<p>Enter your terms and conditions here...</p>';
				if (termsEditor) {
					termsEditor.commands.setContent(defaultContent);
				}
				termsVersion = 'No saved versions yet';
			} else if (termsResponse.status === 401) {
				console.error('Authentication failed - redirecting to login');
				showMessage('Authentication failed. Please log in again.', 'error');
				goto('/auth/login');
				return;
			} else {
				console.error('Failed to load terms:', termsResponse.status, termsResponse.statusText);
				const defaultContent = '<p>Enter your terms and conditions here...</p>';
				if (termsEditor) {
					termsEditor.commands.setContent(defaultContent);
				}
				termsVersion = 'Error loading document';
			}

			// Load privacy policy
			const privacyResponse = await fetch('/api/admin/ext/legalpages/documents/privacy', {
				headers
			});
			if (privacyResponse.ok) {
				const privacyDoc = await privacyResponse.json();
				if (privacyDoc && privacyDoc.content) {
					if (privacyEditor) {
						privacyEditor.commands.setContent(privacyDoc.content);
					}
					privacyTitle = privacyDoc.title || 'Privacy Policy';
					privacyVersion = privacyDoc.status === 'published'
						? `Version ${privacyDoc.version} - Published`
						: `Version ${privacyDoc.version} - Draft`;
				} else {
					const defaultContent = '<p>Enter your privacy policy here...</p>';
					if (privacyEditor) {
						privacyEditor.commands.setContent(defaultContent);
					}
					privacyVersion = 'No saved versions yet';
				}
			} else if (privacyResponse.status === 404) {
				// Document doesn't exist yet
				const defaultContent = '<p>Enter your privacy policy here...</p>';
				if (privacyEditor) {
					privacyEditor.commands.setContent(defaultContent);
				}
				privacyVersion = 'No saved versions yet';
			} else if (privacyResponse.status === 401) {
				console.error('Authentication failed - redirecting to login');
				showMessage('Authentication failed. Please log in again.', 'error');
				goto('/auth/login');
				return;
			} else {
				console.error('Failed to load privacy:', privacyResponse.status, privacyResponse.statusText);
				const defaultContent = '<p>Enter your privacy policy here...</p>';
				if (privacyEditor) {
					privacyEditor.commands.setContent(defaultContent);
				}
				privacyVersion = 'Error loading document';
			}
		} catch (error) {
			console.error('Error loading documents:', error);
			showMessage('Failed to load documents', 'error');
		}
	}

	async function saveDocument(publish = false) {
		saving = true;
		const editor = selectedTab === 'terms' ? termsEditor : privacyEditor;
		const title = selectedTab === 'terms' ? termsTitle : privacyTitle;
		const content = editor.getHTML();

		try {
			// Token is stored in httpOnly cookie
			const response = await fetch(`/api/admin/ext/legalpages/documents/${selectedTab}`, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
				},
				body: JSON.stringify({ title, content })
			});

			if (response.ok) {
				const doc = await response.json();
				showMessage(
					publish ? 'Document published successfully!' : 'Document saved successfully!',
					'success'
				);

				// Update version info
				if (selectedTab === 'terms') {
					termsVersion = publish
						? `Version ${doc.version} - Published`
						: `Version ${doc.version} - Draft`;
				} else {
					privacyVersion = publish
						? `Version ${doc.version} - Published`
						: `Version ${doc.version} - Draft`;
				}
			} else {
				showMessage('Failed to save document', 'error');
			}
		} catch (error) {
			console.error('Error saving document:', error);
			showMessage('Error saving document', 'error');
		} finally {
			saving = false;
		}
	}

	function showMessage(msg: string, type: 'success' | 'error') {
		message = msg;
		messageType = type;
		setTimeout(() => {
			message = '';
			messageType = '';
		}, 5000);
	}

	function formatText(command: string) {
		const editor = selectedTab === 'terms' ? termsEditor : privacyEditor;

		switch (command) {
			case 'bold':
				editor.chain().focus().toggleBold().run();
				break;
			case 'italic':
				editor.chain().focus().toggleItalic().run();
				break;
			case 'h1':
				editor.chain().focus().toggleHeading({ level: 1 }).run();
				break;
			case 'h2':
				editor.chain().focus().toggleHeading({ level: 2 }).run();
				break;
			case 'ul':
				editor.chain().focus().toggleBulletList().run();
				break;
			case 'ol':
				editor.chain().focus().toggleOrderedList().run();
				break;
			case 'link':
				const url = prompt('Enter URL:');
				if (url) {
					editor.chain().focus().setLink({ href: url }).run();
				}
				break;
		}
	}

	function getPreviewContent() {
		const editor = selectedTab === 'terms' ? termsEditor : privacyEditor;
		return editor ? editor.getHTML() : '';
	}

</script>

<div class="min-h-screen bg-gray-50">
	<div class="container mx-auto px-4 py-8">
		<!-- Header -->
		<div class="mb-8">
			<div class="flex items-center gap-3 mb-2">
				<FileText class="w-8 h-8 text-blue-600" />
				<h1 class="text-3xl font-bold text-gray-900">Legal Pages</h1>
			</div>
			<p class="text-gray-600">Manage your terms and conditions and privacy policy documents</p>
		</div>

		<!-- Success/Error Messages -->
		{#if message}
			<div class="mb-4 p-4 rounded-lg {messageType === 'success' ? 'bg-green-50 text-green-800' : 'bg-red-50 text-red-800'}">
				{message}
			</div>
		{/if}

		<!-- Tab Navigation -->
		<div class="bg-white rounded-lg shadow mb-4">
			<div class="border-b border-gray-200">
				<nav class="flex">
					<button
						class="px-6 py-3 text-sm font-medium border-b-2 transition-colors {
							selectedTab === 'terms'
								? 'border-blue-500 text-blue-600'
								: 'border-transparent text-gray-500 hover:text-gray-700'
						}"
						on:click={() => { selectedTab = 'terms'; showPreview = false; }}
					>
						Terms and Conditions
					</button>
					<button
						class="px-6 py-3 text-sm font-medium border-b-2 transition-colors {
							selectedTab === 'privacy'
								? 'border-blue-500 text-blue-600'
								: 'border-transparent text-gray-500 hover:text-gray-700'
						}"
						on:click={() => { selectedTab = 'privacy'; showPreview = false; }}
					>
						Privacy Policy
					</button>
				</nav>
			</div>
		</div>

		<!-- Editor/Preview Area -->
		<div class="bg-white rounded-lg shadow">
			{#if loading}
				<div class="flex items-center justify-center h-96">
					<div class="text-center">
						<div class="inline-block animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
						<p class="mt-4 text-gray-600">Loading editor...</p>
					</div>
				</div>
			{:else}
				<!-- Editor Mode -->
				<div class="p-6">
					<!-- Title Input -->
					<div class="mb-4">
						<label class="block text-sm font-medium text-gray-700 mb-2">
							Document Title
						</label>
						<input
							type="text"
							class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
							value={currentTitle}
							on:input={(e) => updateTitle(e.currentTarget.value)}
						/>
					</div>

					<!-- Editor Toolbar -->
					<div class="mb-4 p-2 bg-gray-50 border border-gray-200 rounded-t-md flex gap-2 flex-wrap">
						<button
							class="p-2 rounded hover:bg-gray-200 transition-colors"
							on:click={() => formatText('bold')}
							title="Bold"
						>
							<Bold class="w-4 h-4" />
						</button>
						<button
							class="p-2 rounded hover:bg-gray-200 transition-colors"
							on:click={() => formatText('italic')}
							title="Italic"
						>
							<Italic class="w-4 h-4" />
						</button>
						<button
							class="p-2 rounded hover:bg-gray-200 transition-colors"
							on:click={() => formatText('h1')}
							title="Heading 1"
						>
							<Heading1 class="w-4 h-4" />
						</button>
						<button
							class="p-2 rounded hover:bg-gray-200 transition-colors"
							on:click={() => formatText('h2')}
							title="Heading 2"
						>
							<Heading2 class="w-4 h-4" />
						</button>
						<button
							class="p-2 rounded hover:bg-gray-200 transition-colors"
							on:click={() => formatText('ul')}
							title="Bullet List"
						>
							<List class="w-4 h-4" />
						</button>
						<button
							class="p-2 rounded hover:bg-gray-200 transition-colors"
							on:click={() => formatText('ol')}
							title="Numbered List"
						>
							<ListOrdered class="w-4 h-4" />
						</button>
						<button
							class="p-2 rounded hover:bg-gray-200 transition-colors"
							on:click={() => formatText('link')}
							title="Insert Link"
						>
							<Link class="w-4 h-4" />
						</button>
					</div>

					<!-- Editor Content -->
					<div class="border border-gray-200 rounded-b-md bg-white min-h-[400px] relative">
						<!-- Always render editors -->
						<div
							bind:this={termsElement}
							class="editor-container {selectedTab === 'terms' && !showPreview ? '' : 'sr-only'}"
						></div>
						<div
							bind:this={privacyElement}
							class="editor-container {selectedTab === 'privacy' && !showPreview ? '' : 'sr-only'}"
						></div>

						<!-- Preview overlay -->
						{#if showPreview}
							<div class="absolute inset-0 bg-white p-4 overflow-auto z-10">
								<div class="mb-4 bg-yellow-50 border border-yellow-200 rounded p-3 text-sm text-yellow-800">
									Preview Mode - This is how your document will appear to users
								</div>
								<div class="prose max-w-none">
									{@html getPreviewContent()}
								</div>
							</div>
						{/if}
					</div>

					<!-- Version Info -->
					<div class="mt-4 text-sm text-gray-600 flex items-center gap-2">
						<Clock class="w-4 h-4" />
						{currentVersion}
					</div>
				</div>
			{/if}

			<!-- Action Buttons -->
			<div class="p-6 border-t border-gray-200 flex justify-between">
				<div class="flex gap-2">
					<button
						class="px-4 py-2 bg-gray-600 text-white rounded-md hover:bg-gray-700 transition-colors flex items-center gap-2"
						on:click={() => showPreview = !showPreview}
					>
						<Eye class="w-4 h-4" />
						{showPreview ? 'Edit' : 'Preview'}
					</button>
				</div>

				<div class="flex gap-2">
					<button
						class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors flex items-center gap-2 disabled:opacity-50"
						on:click={() => saveDocument(false)}
						disabled={saving || showPreview}
					>
						<Save class="w-4 h-4" />
						{saving ? 'Saving...' : 'Save Draft'}
					</button>
					<button
						class="px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 transition-colors flex items-center gap-2 disabled:opacity-50"
						on:click={() => saveDocument(true)}
						disabled={saving || showPreview}
					>
						<Globe class="w-4 h-4" />
						{saving ? 'Publishing...' : 'Save & Publish'}
					</button>
				</div>
			</div>
		</div>

		<!-- Quick Links -->
		<div class="mt-6 flex gap-4 text-sm">
			<a
				href="/api/ext/legalpages/terms"
				target="_blank"
				class="text-blue-600 hover:underline flex items-center gap-1"
			>
				<Globe class="w-4 h-4" />
				View Public Terms Page
			</a>
			<a
				href="/api/ext/legalpages/privacy"
				target="_blank"
				class="text-blue-600 hover:underline flex items-center gap-1"
			>
				<Globe class="w-4 h-4" />
				View Public Privacy Page
			</a>
		</div>
	</div>
</div>

<style>
	.editor-container {
		min-height: 400px;
	}

	:global(.ProseMirror) {
		min-height: 400px;
		padding: 1rem;
		outline: none;
	}

	:global(.ProseMirror:focus) {
		outline: none;
		border-color: #3b82f6;
	}

	:global(.ProseMirror p) {
		margin-bottom: 1em;
		line-height: 1.6;
	}

	:global(.ProseMirror h1) {
		font-size: 1.875rem;
		font-weight: bold;
		margin-bottom: 1rem;
		margin-top: 1.5rem;
	}

	:global(.ProseMirror h2) {
		font-size: 1.5rem;
		font-weight: bold;
		margin-bottom: 0.75rem;
		margin-top: 1.25rem;
	}

	:global(.ProseMirror ul, .ProseMirror ol) {
		padding-left: 1.5rem;
		margin-bottom: 1rem;
	}

	:global(.ProseMirror li) {
		margin-bottom: 0.25rem;
	}

	:global(.ProseMirror a) {
		color: #3b82f6;
		text-decoration: underline;
		cursor: pointer;
	}

	:global(.ProseMirror a:hover) {
		color: #2563eb;
	}

	:global(.ProseMirror strong) {
		font-weight: bold;
	}

	:global(.ProseMirror em) {
		font-style: italic;
	}
</style>