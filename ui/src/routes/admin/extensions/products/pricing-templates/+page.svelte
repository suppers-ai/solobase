<script>
	import { onMount } from 'svelte';
	import { Plus, Edit2, Trash2, Code, DollarSign, Filter, Copy, ChevronRight, AlertCircle } from 'lucide-svelte';

	let templates = [];
	let loading = true;
	let error = null;
	let showCreateModal = false;
	let showEditModal = false;
	let editingTemplate = null;
	let selectedCategory = 'all';

	// Form data
	let formData = {
		name: '',
		display_name: '',
		description: '',
		price_formula: '',
		condition_formula: '',
		category: 'standard',
		priority: 0,
		variables: {},
		is_active: true
	};

	// Variable management
	let newVariableName = '';
	let newVariableRequired = true;

	onMount(() => {
		loadTemplates();
	});

	async function loadTemplates() {
		loading = true;
		error = null;
		try {
			const response = await fetch('/api/products/pricing-templates');
			if (!response.ok) throw new Error('Failed to load templates');
			templates = await response.json();
		} catch (err) {
			error = err.message;
		} finally {
			loading = false;
		}
	}

	function resetForm() {
		formData = {
			name: '',
			display_name: '',
			description: '',
			price_formula: '',
			condition_formula: '',
			category: 'standard',
			priority: 0,
			variables: {},
			is_active: true
		};
		newVariableName = '';
		newVariableRequired = true;
	}

	function openCreateModal() {
		resetForm();
		showCreateModal = true;
	}

	function openEditModal(template) {
		editingTemplate = template;
		formData = { ...template };
		if (!formData.variables) formData.variables = {};
		showEditModal = true;
	}

	function addVariable() {
		if (newVariableName && !formData.variables.required?.includes(newVariableName)) {
			if (!formData.variables.required) formData.variables.required = [];
			if (!formData.variables.optional) formData.variables.optional = [];
			
			if (newVariableRequired) {
				formData.variables.required = [...formData.variables.required, newVariableName];
			} else {
				formData.variables.optional = [...formData.variables.optional, newVariableName];
			}
			formData = formData;
			newVariableName = '';
		}
	}

	function removeVariable(name, isRequired) {
		if (isRequired) {
			formData.variables.required = formData.variables.required.filter(v => v !== name);
		} else {
			formData.variables.optional = formData.variables.optional.filter(v => v !== name);
		}
		formData = formData;
	}

	async function saveTemplate() {
		try {
			const url = showEditModal 
				? `/api/products/pricing-templates/${editingTemplate.id}`
				: '/api/products/pricing-templates';
			
			const method = showEditModal ? 'PUT' : 'POST';
			
			const response = await fetch(url, {
				method,
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(formData)
			});

			if (!response.ok) throw new Error('Failed to save template');

			await loadTemplates();
			showCreateModal = false;
			showEditModal = false;
			resetForm();
		} catch (err) {
			alert('Error saving template: ' + err.message);
		}
	}

	async function deleteTemplate(template) {
		if (!confirm(`Are you sure you want to delete the template "${template.display_name}"?`)) {
			return;
		}

		try {
			const response = await fetch(`/api/products/pricing-templates/${template.id}`, {
				method: 'DELETE'
			});

			if (!response.ok) throw new Error('Failed to delete template');

			await loadTemplates();
		} catch (err) {
			alert('Error deleting template: ' + err.message);
		}
	}

	function getCategoryColor(category) {
		switch (category) {
			case 'standard': return 'badge-primary';
			case 'discount': return 'badge-success';
			case 'dynamic': return 'badge-warning';
			case 'subscription': return 'badge-info';
			case 'shipping': return 'badge-secondary';
			default: return 'badge-ghost';
		}
	}

	function copyFormula(formula) {
		navigator.clipboard.writeText(formula);
	}

	$: filteredTemplates = selectedCategory === 'all' 
		? templates 
		: templates.filter(t => t.category === selectedCategory);
</script>

<div class="container mx-auto p-6">
	<div class="flex justify-between items-center mb-6">
		<div>
			<h1 class="text-3xl font-bold">Pricing Templates</h1>
			<p class="text-base-content/60 mt-2">
				Manage reusable pricing formulas and conditions
			</p>
		</div>
		<button 
			class="btn btn-primary" 
			on:click={openCreateModal}
		>
			<Plus class="w-4 h-4 mr-2" />
			Create Template
		</button>
	</div>

	<!-- Category Filter -->
	<div class="tabs tabs-boxed mb-6">
		<button 
			class="tab {selectedCategory === 'all' ? 'tab-active' : ''}"
			on:click={() => selectedCategory = 'all'}
		>
			All
		</button>
		<button 
			class="tab {selectedCategory === 'standard' ? 'tab-active' : ''}"
			on:click={() => selectedCategory = 'standard'}
		>
			Standard
		</button>
		<button 
			class="tab {selectedCategory === 'discount' ? 'tab-active' : ''}"
			on:click={() => selectedCategory = 'discount'}
		>
			Discount
		</button>
		<button 
			class="tab {selectedCategory === 'dynamic' ? 'tab-active' : ''}"
			on:click={() => selectedCategory = 'dynamic'}
		>
			Dynamic
		</button>
		<button 
			class="tab {selectedCategory === 'subscription' ? 'tab-active' : ''}"
			on:click={() => selectedCategory = 'subscription'}
		>
			Subscription
		</button>
		<button 
			class="tab {selectedCategory === 'shipping' ? 'tab-active' : ''}"
			on:click={() => selectedCategory = 'shipping'}
		>
			Shipping
		</button>
	</div>

	<!-- Templates List -->
	{#if loading}
		<div class="flex justify-center py-12">
			<span class="loading loading-spinner loading-lg"></span>
		</div>
	{:else if error}
		<div class="alert alert-error">
			<AlertCircle class="w-4 h-4" />
			<span>{error}</span>
		</div>
	{:else if filteredTemplates.length === 0}
		<div class="card bg-base-200 p-12 text-center">
			<p class="text-base-content/60">No templates found</p>
		</div>
	{:else}
		<div class="grid gap-4">
			{#each filteredTemplates as template}
				<div class="card bg-base-200">
					<div class="card-body">
						<div class="flex justify-between items-start">
							<div class="flex-1">
								<div class="flex items-center gap-3 mb-2">
									<h3 class="text-xl font-semibold">{template.display_name}</h3>
									<span class="badge {getCategoryColor(template.category)} badge-sm">
										{template.category}
									</span>
									{#if template.priority > 0}
										<span class="badge badge-outline badge-sm">
											Priority: {template.priority}
										</span>
									{/if}
									{#if !template.is_active}
										<span class="badge badge-ghost badge-sm">Inactive</span>
									{/if}
								</div>
								
								<p class="text-base-content/60 mb-4">{template.description}</p>

								<!-- Price Formula -->
								<div class="mb-3">
									<div class="flex items-center gap-2 mb-1">
										<DollarSign class="w-4 h-4 text-primary" />
										<span class="font-semibold text-sm">Price Formula:</span>
									</div>
									<div class="bg-base-100 rounded-lg p-3 font-mono text-sm relative group">
										<code>{template.price_formula}</code>
										<button
											class="btn btn-ghost btn-xs absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity"
											on:click={() => copyFormula(template.price_formula)}
										>
											<Copy class="w-3 h-3" />
										</button>
									</div>
								</div>

								<!-- Condition Formula (if exists) -->
								{#if template.condition_formula}
									<div class="mb-3">
										<div class="flex items-center gap-2 mb-1">
											<Filter class="w-4 h-4 text-warning" />
											<span class="font-semibold text-sm">Condition:</span>
										</div>
										<div class="bg-base-100 rounded-lg p-3 font-mono text-sm relative group">
											<code>{template.condition_formula}</code>
											<button
												class="btn btn-ghost btn-xs absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity"
												on:click={() => copyFormula(template.condition_formula)}
											>
												<Copy class="w-3 h-3" />
											</button>
										</div>
									</div>
								{/if}

								<!-- Required Variables -->
								{#if template.variables?.required?.length > 0}
									<div class="flex items-center gap-2 mt-3">
										<span class="text-sm font-semibold">Required Variables:</span>
										<div class="flex flex-wrap gap-1">
											{#each template.variables.required as variable}
												<span class="badge badge-sm badge-outline">
													{variable}
												</span>
											{/each}
										</div>
									</div>
								{/if}
							</div>

							<div class="flex gap-2">
								<button
									class="btn btn-ghost btn-sm"
									on:click={() => openEditModal(template)}
								>
									<Edit2 class="w-4 h-4" />
								</button>
								<button
									class="btn btn-ghost btn-sm text-error"
									on:click={() => deleteTemplate(template)}
								>
									<Trash2 class="w-4 h-4" />
								</button>
							</div>
						</div>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<!-- Create/Edit Modal -->
{#if showCreateModal || showEditModal}
	<div class="modal modal-open">
		<div class="modal-box max-w-4xl">
			<h3 class="font-bold text-lg mb-4">
				{showEditModal ? 'Edit Pricing Template' : 'Create New Pricing Template'}
			</h3>

			<div class="space-y-4">
				<!-- Basic Info -->
				<div class="grid grid-cols-2 gap-4">
					<div class="form-control">
						<label class="label">
							<span class="label-text">Template Name</span>
						</label>
						<input
							type="text"
							class="input input-bordered"
							bind:value={formData.name}
							placeholder="e.g., volume_discount"
						/>
					</div>

					<div class="form-control">
						<label class="label">
							<span class="label-text">Display Name</span>
						</label>
						<input
							type="text"
							class="input input-bordered"
							bind:value={formData.display_name}
							placeholder="e.g., Volume Discount"
						/>
					</div>
				</div>

				<!-- Category and Priority -->
				<div class="grid grid-cols-2 gap-4">
					<div class="form-control">
						<label class="label">
							<span class="label-text">Category</span>
						</label>
						<select class="select select-bordered" bind:value={formData.category}>
							<option value="standard">Standard</option>
							<option value="discount">Discount</option>
							<option value="dynamic">Dynamic</option>
							<option value="subscription">Subscription</option>
							<option value="shipping">Shipping</option>
							<option value="bundle">Bundle</option>
							<option value="complex">Complex</option>
						</select>
					</div>

					<div class="form-control">
						<label class="label">
							<span class="label-text">Priority</span>
							<span class="label-text-alt">Higher priority templates are evaluated first</span>
						</label>
						<input
							type="number"
							class="input input-bordered"
							bind:value={formData.priority}
							placeholder="0"
							min="0"
							max="1000"
						/>
					</div>
				</div>

				<!-- Description -->
				<div class="form-control">
					<label class="label">
						<span class="label-text">Description</span>
					</label>
					<textarea
						class="textarea textarea-bordered"
						bind:value={formData.description}
						placeholder="Describe what this template does..."
						rows="2"
					></textarea>
				</div>

				<!-- Price Formula -->
				<div class="form-control">
					<label class="label">
						<span class="label-text">Price Formula</span>
						<span class="label-text-alt">Required</span>
					</label>
					<textarea
						class="textarea textarea-bordered font-mono text-sm"
						bind:value={formData.price_formula}
						placeholder="e.g., base_price * quantity * (quantity >= 100 ? 0.8 : 1.0)"
						rows="3"
					></textarea>
					<label class="label">
						<span class="label-text-alt">
							Use JavaScript-like syntax. Variables: base_price, quantity, tax_rate, etc.
						</span>
					</label>
				</div>

				<!-- Condition Formula -->
				<div class="form-control">
					<label class="label">
						<span class="label-text">Condition Formula</span>
						<span class="label-text-alt">Optional - when this template should apply</span>
					</label>
					<textarea
						class="textarea textarea-bordered font-mono text-sm"
						bind:value={formData.condition_formula}
						placeholder="e.g., quantity >= 10 && is_member == true"
						rows="2"
					></textarea>
					<label class="label">
						<span class="label-text-alt">
							Leave empty to always apply. Use boolean expressions.
						</span>
					</label>
				</div>

				<!-- Variables -->
				<div class="form-control">
					<label class="label">
						<span class="label-text">Required Variables</span>
					</label>
					<div class="flex gap-2 mb-2">
						<input
							type="text"
							class="input input-bordered flex-1"
							bind:value={newVariableName}
							placeholder="Variable name..."
							on:keydown={(e) => e.key === 'Enter' && addVariable()}
						/>
						<select class="select select-bordered" bind:value={newVariableRequired}>
							<option value={true}>Required</option>
							<option value={false}>Optional</option>
						</select>
						<button 
							class="btn btn-primary"
							on:click={addVariable}
						>
							Add
						</button>
					</div>

					<!-- Required Variables List -->
					{#if formData.variables?.required?.length > 0}
						<div class="mb-2">
							<span class="text-sm font-semibold">Required:</span>
							<div class="flex flex-wrap gap-2 mt-1">
								{#each formData.variables.required as variable}
									<span class="badge badge-primary gap-2">
										{variable}
										<button on:click={() => removeVariable(variable, true)}>
											×
										</button>
									</span>
								{/each}
							</div>
						</div>
					{/if}

					<!-- Optional Variables List -->
					{#if formData.variables?.optional?.length > 0}
						<div>
							<span class="text-sm font-semibold">Optional:</span>
							<div class="flex flex-wrap gap-2 mt-1">
								{#each formData.variables.optional as variable}
									<span class="badge badge-ghost gap-2">
										{variable}
										<button on:click={() => removeVariable(variable, false)}>
											×
										</button>
									</span>
								{/each}
							</div>
						</div>
					{/if}
				</div>

				<!-- Active Status -->
				<div class="form-control">
					<label class="label cursor-pointer justify-start gap-3">
						<input
							type="checkbox"
							class="checkbox"
							bind:checked={formData.is_active}
						/>
						<span class="label-text">Active</span>
					</label>
				</div>
			</div>

			<div class="modal-action">
				<button 
					class="btn btn-ghost" 
					on:click={() => {
						showCreateModal = false;
						showEditModal = false;
						resetForm();
					}}
				>
					Cancel
				</button>
				<button 
					class="btn btn-primary" 
					on:click={saveTemplate}
				>
					{showEditModal ? 'Update' : 'Create'} Template
				</button>
			</div>
		</div>
		<div class="modal-backdrop" on:click={() => {
			showCreateModal = false;
			showEditModal = false;
			resetForm();
		}}></div>
	</div>
{/if}