<script lang="ts">
	import { onMount } from 'svelte';
	import { 
		Calculator, Plus, Edit2, Trash2, Search, Filter,
		DollarSign, TrendingUp, BarChart3, Zap, Code2,
		Variable, FileText, CheckCircle, XCircle, Play, AlertCircle, Settings, ArrowLeft
	} from 'lucide-svelte';
	import { api } from '$lib/api';
	import { requireAdmin } from '$lib/utils/auth';
	import FormulaEditor from '$lib/components/FormulaEditor.svelte';

	interface PricingTemplate {
		id: string;
		name: string;
		display_name: string;
		description: string;
		category: string;
		price_formula: string;
		condition_formula?: string;
		variables?: any;
		is_active: boolean;
		created_at: string;
		updated_at: string;
	}

	interface PricingRule {
		id: string;
		name: string;
		product_type_id?: string;
		group_type_id?: string;
		template_id?: string;
		formula: string;
		priority: number;
		conditions: any[];
		is_active: boolean;
	}

	let pricingTemplates: PricingTemplate[] = [];
	let pricingRules: PricingRule[] = [];
	let availableVariables: any[] = [];
	let loading = true;
	let activeTab = 'templates';
	let searchQuery = '';
	let selectedCategory = 'all';
	let showCreateModal = false;
	let showEditModal = false;
	let showTestModal = false;
	let selectedTemplate: PricingTemplate | null = null;
	let selectedRule: PricingRule | null = null;
	
	// Form data for new template
	let newTemplate: Partial<PricingTemplate> = {
		name: '',
		display_name: '',
		description: '',
		category: 'discount',
		price_formula: '',
		condition_formula: '',
		variables: {},
		is_active: true
	};

	// Test data for formula testing
	let testVariables: any = {};
	let testResult: any = null;
	let testError: string = '';
	let showFormulaEditor = false;
	let editingFormulaType: 'price' | 'condition' | null = null;
	let tempFormula = '';

	// Categories for templates
	const templateCategories = [
		{ value: 'discount', label: 'Discount' },
		{ value: 'membership', label: 'Membership' },
		{ value: 'complex', label: 'Complex' },
		{ value: 'custom', label: 'Custom' }
	];

	// Common formula templates
	const formulaTemplates = [
		{
			name: 'Simple Markup',
			formula: 'base_price * (1 + markup_percentage / 100)',
			variables: ['base_price', 'markup_percentage']
		},
		{
			name: 'Quantity Discount',
			formula: 'base_price * quantity * (1 - min(quantity * 0.01, 0.3))',
			variables: ['base_price', 'quantity']
		},
		{
			name: 'Tiered Pricing',
			formula: 'quantity <= 10 ? base_price * quantity : quantity <= 50 ? base_price * quantity * 0.9 : base_price * quantity * 0.8',
			variables: ['base_price', 'quantity']
		},
		{
			name: 'Subscription',
			formula: 'monthly_price * (billing_cycle == "yearly" ? 10 : billing_cycle == "quarterly" ? 2.7 : 1)',
			variables: ['monthly_price', 'billing_cycle']
		}
	];

	$: filteredTemplates = pricingTemplates.filter(template => {
		const matchesSearch = template.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
			template.display_name.toLowerCase().includes(searchQuery.toLowerCase()) ||
			template.description?.toLowerCase().includes(searchQuery.toLowerCase());
		const matchesCategory = selectedCategory === 'all' || template.category === selectedCategory;
		return matchesSearch && matchesCategory;
	});


	onMount(async () => {
		if (!requireAdmin()) return;
		await loadPricingData();
	});

	async function loadPricingData() {
		try {
			loading = true;
			const [templatesRes, variablesRes] = await Promise.all([
				api.get('/products/pricing-templates'),
				api.get('/products/variables')
			]);
			pricingTemplates = templatesRes || [];
			availableVariables = variablesRes || [];
		} catch (error) {
			console.error('Failed to load pricing data:', error);
			pricingTemplates = [];
			availableVariables = [];
		} finally {
			loading = false;
		}
	}

	function getCategoryColor(category: string) {
		switch (category) {
			case 'ecommerce': return 'bg-blue-100 text-blue-700';
			case 'subscription': return 'bg-purple-100 text-purple-700';
			case 'service': return 'bg-orange-100 text-orange-700';
			case 'custom': return 'bg-gray-100 text-gray-700';
			default: return 'bg-gray-100 text-gray-700';
		}
	}

	function extractVariables(formula: string): string[] {
		// Extract variable names from formula (basic regex, can be improved)
		const regex = /\b[a-z_][a-z0-9_]*\b/gi;
		const matches = formula.match(regex) || [];
		const keywords = ['if', 'else', 'min', 'max', 'round', 'floor', 'ceil', 'abs'];
		return [...new Set(matches.filter(m => !keywords.includes(m.toLowerCase())))];
	}

	function applyFormulaTemplate(template: any) {
		newTemplate.price_formula = template.formula;
		newTemplate.variables = { required: template.variables };
	}
	
	async function createTemplate() {
		try {
			// Extract variables from formula
			const priceVars = extractVariables(newTemplate.price_formula || '');
			const conditionVars = extractVariables(newTemplate.condition_formula || '');
			const allVars = [...new Set([...priceVars, ...conditionVars])];
			newTemplate.variables = { required: allVars };
			
			const result = await api.post('/products/pricing-templates', newTemplate);
			if (result) {
				// Reset form
				newTemplate = {
					name: '',
					display_name: '',
					description: '',
					category: 'discount',
					price_formula: '',
					condition_formula: '',
					variables: {},
					priority: 0,
					is_active: true
				};
				showCreateModal = false;
				// Reload templates
				await loadPricingData();
			}
		} catch (error) {
			console.error('Failed to create pricing template:', error);
			alert('Failed to create pricing template');
		}
	}
	
	async function editTemplate(template: PricingTemplate) {
		selectedTemplate = { ...template };
		showEditModal = true;
	}
	
	async function updateTemplate() {
		if (!selectedTemplate) return;
		
		try {
			// Extract variables from formulas
			const priceVars = extractVariables(selectedTemplate.price_formula || '');
			const conditionVars = extractVariables(selectedTemplate.condition_formula || '');
			const allVars = [...new Set([...priceVars, ...conditionVars])];
			selectedTemplate.variables = { required: allVars };
			
			const result = await api.put(`/products/pricing-templates/${selectedTemplate.id}`, selectedTemplate);
			if (result) {
				showEditModal = false;
				selectedTemplate = null;
				// Reload templates
				await loadPricingData();
			}
		} catch (error) {
			console.error('Failed to update pricing template:', error);
			alert('Failed to update pricing template');
		}
	}
	
	async function deleteTemplate(id: string) {
		if (!confirm('Are you sure you want to delete this pricing template? Products using this template will need to be updated.')) return;
		
		try {
			await api.delete(`/products/pricing-templates/${id}`);
			// Reload templates
			await loadPricingData();
		} catch (error) {
			console.error('Failed to delete pricing template:', error);
			alert('Failed to delete pricing template');
		}
	}

	function openTestModal(template: PricingTemplate) {
		selectedTemplate = template;
		testVariables = {};
		testResult = null;
		testError = '';
		// Initialize test variables with defaults
		template.variables_used.forEach(varName => {
			testVariables[varName] = 0;
		});
		showTestModal = true;
	}

	async function testFormula() {
		if (!selectedTemplate) return;
		
		try {
			testError = '';
			// In a real implementation, this would call an API to evaluate the formula
			// For now, we'll show a mock result
			testResult = {
				input: { ...testVariables },
				output: 99.99,
				execution_time: '0.5ms'
			};
		} catch (error) {
			testError = 'Failed to evaluate formula: ' + error;
			testResult = null;
		}
	}

	// Condition builder state
	let conditions: any[] = [];
	let editingConditions = false;

	function addCondition() {
		conditions = [...conditions, {
			field: '',
			operator: '==',
			value: ''
		}];
	}

	function removeCondition(index: number) {
		conditions = conditions.filter((_, i) => i !== index);
	}

	function saveConditions(item: any) {
		item.conditions = conditions;
		editingConditions = false;
		conditions = [];
	}
</script>

<div class="page-container">
	<!-- Header -->
	<div class="page-header">
		<a href="/admin/extensions/products" class="back-button">
			<ArrowLeft size={20} />
		</a>
		<div class="header-content">
			<div class="header-left">
				<div class="header-title">
					<Calculator size={24} />
					<h1>Pricing & Formulas</h1>
				</div>
				<p class="header-subtitle">Configure dynamic pricing rules and formulas for your products</p>
			</div>
		</div>
	</div>

	<!-- Tabs -->
	<div class="tabs">
		<button class="tab {activeTab === 'templates' ? 'active' : ''}" on:click={() => activeTab = 'templates'}>
			<FileText size={16} />
			Templates
		</button>
		<button class="tab {activeTab === 'variables' ? 'active' : ''}" on:click={() => activeTab = 'variables'}>
			<Variable size={16} />
			Variables
		</button>
	</div>

	<!-- Main Content -->
	<div class="content-card">
		<!-- Toolbar -->
		<div class="toolbar">
			<div class="toolbar-left">
				<div class="search-box">
					<Search size={16} />
					<input 
						type="text" 
						placeholder="Search {activeTab}..."
						bind:value={searchQuery}
					/>
				</div>
				{#if activeTab === 'templates'}
					<select class="filter-select" bind:value={selectedCategory}>
						<option value="all">All Categories</option>
						{#each templateCategories as cat}
							<option value={cat.value}>{cat.label}</option>
						{/each}
					</select>
				{/if}
			</div>
			<div class="toolbar-right">
				<button class="btn-icon">
					<Filter size={16} />
				</button>
			</div>
		</div>

		{#if activeTab === 'templates'}
			<!-- Pricing Templates -->
			{#if loading}
				<div class="loading-container">
					<div class="loading loading-spinner loading-lg text-cyan-600"></div>
				</div>
			{:else if filteredTemplates.length === 0}
				<div class="empty-state">
					<Calculator size={48} class="text-gray-400" />
					<h3>No pricing templates found</h3>
					<p>Create your first pricing template to start building dynamic pricing</p>
					<button class="btn btn-primary mt-4" on:click={() => showCreateModal = true}>
						<Plus size={16} />
						Create Template
					</button>
				</div>
			{:else}
				<div class="template-grid">
					{#each filteredTemplates as template}
						<div class="template-card" on:click={() => editTemplate(template)} role="button" tabindex="0" on:keypress={(e) => e.key === 'Enter' && editTemplate(template)}>
							<div class="template-header">
								<div class="template-icon">
									<Calculator size={24} />
								</div>
								<span class="status-badge {template.is_active ? 'status-active' : 'status-inactive'}">
									{#if template.is_active}
										<CheckCircle size={12} />
										Active
									{:else}
										<XCircle size={12} />
										Inactive
									{/if}
								</span>
							</div>
							<div class="template-content">
								<h3 class="template-name">{template.display_name}</h3>
								<code class="template-code">{template.name}</code>
								<p class="template-description">{template.description}</p>
								
								<div class="formula-box">
									<div class="formula-label">
										<Code2 size={14} />
										Formula
									</div>
									<code class="formula-text">{template.price_formula || 'No formula defined'}</code>
								</div>
								
								{#if template.condition_formula}
									<div class="formula-box condition-box">
										<div class="formula-label">
											<AlertCircle size={14} />
											CONDITION
										</div>
										<code class="formula-text">{template.condition_formula}</code>
									</div>
								{/if}
								
								{#if template.variables && template.variables.required && Array.isArray(template.variables.required)}
									<div class="variables-section">
										<p class="variables-label">Required Variables:</p>
										<div class="variables-list">
											{#each template.variables.required as variable}
												<span class="variable-badge">{variable}</span>
											{/each}
										</div>
									</div>
								{/if}
								
								<div class="template-footer">
									<span class="category-badge {getCategoryColor(template.category)}">
										{template.category}
									</span>
								</div>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		{:else if activeTab === 'variables'}
			<!-- Variables Overview -->
			<div class="variables-container">
				<div class="variables-header">
					<h3>Available Variables</h3>
					<p>These variables can be used in your pricing formulas</p>
				</div>
				<div class="variables-grid">
					{#each availableVariables as variable}
						<div class="variable-card">
							<div class="variable-icon">
								<Variable size={20} />
							</div>
							<div class="variable-details">
								<code class="variable-name">{variable.name}</code>
								<p class="variable-display">{variable.display_name}</p>
								<span class="variable-type">{variable.type}</span>
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}
	</div>
</div>

<style>
	.page-container {
		padding: 1.5rem;
		max-width: 1400px;
		margin: 0 auto;
	}

	.page-header {
		background: white;
		border-radius: 0.5rem;
		padding: 1.5rem;
		margin-bottom: 1.5rem;
		border: 1px solid #e5e7eb;
		position: relative;
	}
	
	.back-button {
		position: absolute;
		top: 1.5rem;
		left: 1.5rem;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 36px;
		height: 36px;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		background: white;
		color: #6b7280;
		text-decoration: none;
		transition: all 0.2s;
	}
	
	.back-button:hover {
		background: #f9fafb;
		color: #111827;
	}

	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-left: 48px;
	}

	.header-title {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 0.5rem;
	}

	.header-title h1 {
		font-size: 1.5rem;
		font-weight: 600;
		color: #111827;
		margin: 0;
	}

	.header-subtitle {
		color: #6b7280;
		font-size: 0.875rem;
		margin: 0;
	}

	.header-actions {
		display: flex;
		gap: 0.75rem;
	}

	.tabs {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1.5rem;
		background: white;
		padding: 0.5rem;
		border-radius: 0.5rem;
		border: 1px solid #e5e7eb;
	}

	.tab {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		background: transparent;
		border: none;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		color: #6b7280;
		cursor: pointer;
		transition: all 0.2s;
	}

	.tab:hover {
		background: #f3f4f6;
		color: #374151;
	}

	.tab.active {
		background: #06b6d4;
		color: white;
	}

	.content-card {
		background: white;
		border-radius: 0.5rem;
		border: 1px solid #e5e7eb;
		overflow: hidden;
	}

	.toolbar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem 1.5rem;
		border-bottom: 1px solid #e5e7eb;
		gap: 1rem;
	}

	.toolbar-left {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		flex: 1;
	}

	.toolbar-right {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.search-box {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 0.75rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		background: white;
		flex: 1;
		max-width: 320px;
	}

	.search-box input {
		border: none;
		outline: none;
		flex: 1;
		font-size: 0.875rem;
	}

	.filter-select {
		padding: 0.5rem 0.75rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		background: white;
	}

	.btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		border: none;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-primary {
		background: #06b6d4;
		color: white;
	}

	.btn-primary:hover {
		background: #0891b2;
	}

	.btn-secondary {
		background: white;
		color: #374151;
		border: 1px solid #e5e7eb;
	}
	
	.btn-danger {
		background: #ef4444;
		color: white;
	}
	
	.btn-danger:hover {
		background: #dc2626;
	}

	.btn-secondary:hover {
		background: #f9fafb;
	}

	.btn-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		background: white;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-icon:hover {
		background: #f9fafb;
	}

	.btn-icon-danger {
		color: #ef4444;
	}

	.btn-icon-danger:hover {
		background: #fee2e2;
		border-color: #fca5a5;
	}

	.btn-link {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		background: none;
		border: none;
		color: #06b6d4;
		font-size: 0.75rem;
		cursor: pointer;
		padding: 0;
	}

	.btn-link:hover {
		color: #0891b2;
	}

	.template-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(400px, 1fr));
		gap: 1.5rem;
		padding: 1.5rem;
	}

	.template-card {
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
		overflow: hidden;
		transition: all 0.2s;
		cursor: pointer;
	}

	.template-card:hover {
		box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
		transform: translateY(-2px);
	}

	.template-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		background: #f9fafb;
		border-bottom: 1px solid #e5e7eb;
	}
	
	.template-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 40px;
		height: 40px;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		color: #06b6d4;
	}

	.template-badges {
		display: flex;
		gap: 0.5rem;
	}

	.template-actions {
		display: flex;
		gap: 0.5rem;
	}

	.template-content {
		padding: 1rem;
	}

	.template-name {
		font-size: 1.125rem;
		font-weight: 600;
		color: #111827;
		margin: 0 0 0.25rem 0;
	}

	.template-code {
		display: inline-block;
		font-family: 'Courier New', monospace;
		font-size: 0.75rem;
		color: #6b7280;
		background: #f3f4f6;
		padding: 0.125rem 0.375rem;
		border-radius: 0.25rem;
		margin-bottom: 0.75rem;
	}

	.template-description {
		font-size: 0.875rem;
		color: #6b7280;
		margin: 0 0 1rem 0;
	}

	.formula-box {
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		padding: 0.75rem;
		margin-bottom: 1rem;
	}

	.formula-label {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		font-size: 0.75rem;
		font-weight: 600;
		color: #6b7280;
		text-transform: uppercase;
		margin-bottom: 0.5rem;
	}

	.formula-text {
		display: block;
		font-family: 'Courier New', monospace;
		font-size: 0.875rem;
		color: #059669;
		background: white;
		padding: 0.5rem;
		border-radius: 0.25rem;
		word-break: break-all;
	}
	
	.condition-box {
		background: #fef3c7;
		border-color: #fbbf24;
		margin-top: 0.5rem;
	}
	
	.condition-box .formula-label {
		color: #92400e;
	}
	
	.condition-box .formula-text {
		color: #92400e;
		background: #fffbeb;
	}
	
	.formula-display {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding: 1rem;
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
	}
	
	.formula-code {
		display: block;
		padding: 0.75rem;
		background: white;
		border: 1px solid #d1d5db;
		border-radius: 0.25rem;
		font-family: 'Courier New', monospace;
		font-size: 0.875rem;
		color: #0891b2;
		line-height: 1.5;
		word-break: break-all;
	}
	
	.formula-code.condition {
		background: #fffbeb;
		border-color: #fbbf24;
		color: #92400e;
	}
	
	.formula-placeholder {
		display: block;
		padding: 0.75rem;
		color: #9ca3af;
		font-style: italic;
		font-size: 0.875rem;
	}
	
	.btn-edit-formula {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		background: white;
		border: 1px solid #d1d5db;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
		cursor: pointer;
		transition: all 0.2s;
		align-self: flex-start;
	}
	
	.btn-edit-formula:hover {
		background: #f3f4f6;
		border-color: #9ca3af;
		color: #111827;
	}
	
	.help-text {
		font-size: 0.75rem;
		color: #6b7280;
		margin-top: 0.25rem;
	}

	.category-badge {
		display: inline-block;
		padding: 0.25rem 0.75rem;
		border-radius: 9999px;
		font-size: 0.75rem;
		font-weight: 500;
	}

	.default-badge {
		display: inline-block;
		padding: 0.25rem 0.75rem;
		background: #fbbf24;
		color: #78350f;
		border-radius: 9999px;
		font-size: 0.75rem;
		font-weight: 500;
	}

	.variables-section {
		margin-bottom: 1rem;
	}

	.variables-label {
		font-size: 0.75rem;
		font-weight: 600;
		color: #6b7280;
		text-transform: uppercase;
		margin: 0 0 0.5rem 0;
	}

	.variables-list {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.variable-badge {
		display: inline-block;
		padding: 0.25rem 0.5rem;
		background: #ecfdf5;
		color: #059669;
		font-size: 0.75rem;
		font-family: 'Courier New', monospace;
		font-weight: 500;
		border-radius: 0.25rem;
	}

	.template-footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding-top: 1rem;
		border-top: 1px solid #f3f4f6;
	}

	.status-badge {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.25rem 0.75rem;
		border-radius: 9999px;
		font-size: 0.75rem;
		font-weight: 500;
		text-transform: uppercase;
	}

	.status-active {
		background: #d1fae5;
		color: #065f46;
	}

	.status-inactive {
		background: #fee2e2;
		color: #991b1b;
	}

	.rules-container, .variables-container {
		padding: 1.5rem;
	}

	.rules-header, .variables-header {
		margin-bottom: 1.5rem;
	}

	.rules-header h3, .variables-header h3 {
		font-size: 1.125rem;
		font-weight: 600;
		color: #111827;
		margin: 0 0 0.5rem 0;
	}

	.rules-header p, .variables-header p {
		font-size: 0.875rem;
		color: #6b7280;
		margin: 0;
	}

	.rules-list {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.rule-item {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 1rem;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
	}

	.rule-content {
		flex: 1;
	}

	.rule-content h4 {
		font-size: 0.875rem;
		font-weight: 600;
		color: #111827;
		margin: 0 0 0.25rem 0;
	}

	.rule-formula {
		display: inline-block;
		font-family: 'Courier New', monospace;
		font-size: 0.75rem;
		color: #059669;
		background: #ecfdf5;
		padding: 0.25rem 0.5rem;
		border-radius: 0.25rem;
		margin-bottom: 0.5rem;
	}

	.rule-conditions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.condition-badge {
		display: inline-block;
		padding: 0.25rem 0.5rem;
		background: #fef3c7;
		color: #92400e;
		font-size: 0.75rem;
		border-radius: 0.25rem;
	}

	.variables-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
		gap: 1rem;
	}

	.variable-card {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 1rem;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.5rem;
	}

	.variable-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 40px;
		height: 40px;
		background: #ecfdf5;
		border-radius: 0.375rem;
		color: #059669;
	}

	.variable-details {
		flex: 1;
	}

	.variable-name {
		display: block;
		font-family: 'Courier New', monospace;
		font-size: 0.875rem;
		font-weight: 600;
		color: #059669;
		margin-bottom: 0.25rem;
	}

	.variable-display {
		font-size: 0.75rem;
		color: #111827;
		margin: 0 0 0.25rem 0;
	}

	.variable-type {
		display: inline-block;
		padding: 0.125rem 0.375rem;
		background: #f3f4f6;
		color: #6b7280;
		font-size: 0.625rem;
		border-radius: 0.25rem;
		text-transform: uppercase;
	}

	.loading-container {
		display: flex;
		justify-content: center;
		align-items: center;
		padding: 4rem;
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 4rem;
		text-align: center;
	}

	.empty-state h3 {
		margin: 1rem 0 0.5rem 0;
		font-size: 1.125rem;
		font-weight: 600;
		color: #111827;
	}

	.empty-state p {
		margin: 0;
		color: #6b7280;
		font-size: 0.875rem;
	}

	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 9999;
	}

	.modal {
		background: white;
		border-radius: 0.5rem;
		width: 90%;
		max-width: 800px;
		max-height: 90vh;
		overflow-y: auto;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-bottom: 1px solid #e5e7eb;
	}

	.modal-header h2 {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 600;
		color: #111827;
	}

	.modal-body {
		padding: 1.5rem;
	}

	.form-group {
		margin-bottom: 1rem;
	}

	.form-group label {
		display: block;
		margin-bottom: 0.5rem;
		font-size: 0.875rem;
		font-weight: 500;
		color: #374151;
	}

	.form-group input,
	.form-group select,
	.form-group textarea {
		width: 100%;
		padding: 0.5rem 0.75rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		font-size: 0.875rem;
	}

	.form-group textarea {
		font-family: 'Courier New', monospace;
	}

	.form-group input:focus,
	.form-group select:focus,
	.form-group textarea:focus {
		outline: none;
		border-color: #06b6d4;
		box-shadow: 0 0 0 3px rgba(6, 182, 212, 0.1);
	}

	.form-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
	}

	.modal-footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-top: 1px solid #e5e7eb;
	}
	
	.modal-footer-left {
		display: flex;
		gap: 0.75rem;
	}
	
	.modal-footer-right {
		display: flex;
		gap: 0.75rem;
	}

	.formula-templates {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 0.5rem;
		margin-top: 0.5rem;
	}

	.template-option {
		padding: 0.75rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		cursor: pointer;
		transition: all 0.2s;
	}

	.template-option:hover {
		background: #f9fafb;
		border-color: #06b6d4;
	}

	.template-option h4 {
		font-size: 0.875rem;
		font-weight: 600;
		color: #111827;
		margin: 0 0 0.25rem 0;
	}

	.template-option code {
		display: block;
		font-size: 0.75rem;
		color: #6b7280;
		word-break: break-all;
	}

	.test-section {
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		padding: 1rem;
		margin-top: 1rem;
	}

	.test-variables {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1rem;
		margin-bottom: 1rem;
	}

	.test-variable {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.test-variable label {
		font-size: 0.75rem;
		font-weight: 500;
		color: #374151;
	}

	.test-variable input {
		padding: 0.375rem 0.5rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.25rem;
		font-size: 0.875rem;
	}

	.test-result {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		padding: 1rem;
		margin-top: 1rem;
	}

	.test-result.success {
		border-color: #10b981;
		background: #ecfdf5;
	}

	.test-result.error {
		border-color: #ef4444;
		background: #fee2e2;
	}

	.test-result h4 {
		font-size: 0.875rem;
		font-weight: 600;
		color: #111827;
		margin: 0 0 0.5rem 0;
	}

	.test-output {
		font-size: 1.5rem;
		font-weight: 600;
		color: #059669;
		margin: 0.5rem 0;
	}

	.test-details {
		font-size: 0.75rem;
		color: #6b7280;
	}

	.error-message {
		color: #ef4444;
		font-size: 0.875rem;
		margin: 0;
	}
</style>

<!-- Create Template Modal -->
{#if showCreateModal}
	<div class="modal-overlay" on:click={() => showCreateModal = false}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h2>Create New Pricing Template</h2>
				<button class="btn-icon" on:click={() => showCreateModal = false}>
					×
				</button>
			</div>
			<div class="modal-body">
				<div class="form-row">
					<div class="form-group">
						<label for="name">Template Name</label>
						<input type="text" id="name" bind:value={newTemplate.name} 
							placeholder="e.g., standard_pricing, bulk_discount" />
					</div>
					<div class="form-group">
						<label for="display_name">Display Name</label>
						<input type="text" id="display_name" bind:value={newTemplate.display_name} 
							placeholder="e.g., Standard Pricing, Bulk Discount" />
					</div>
				</div>
				<div class="form-group">
					<label for="description">Description</label>
					<textarea id="description" bind:value={newTemplate.description} rows="2" 
						placeholder="Describe what this pricing template does"></textarea>
				</div>
				<div class="form-row">
					<div class="form-group">
						<label for="category">Category</label>
						<select id="category" bind:value={newTemplate.category}>
							{#each templateCategories as cat}
								<option value={cat.value}>{cat.label}</option>
							{/each}
						</select>
					</div>
					<div class="form-group">
						<label>
							<input type="checkbox" bind:checked={newTemplate.is_default} />
							Set as default template
						</label>
					</div>
				</div>
				<div class="form-group">
					<label for="formula">Pricing Formula</label>
					<textarea id="formula" bind:value={newTemplate.formula} rows="4" 
						placeholder="e.g., base_price * quantity * (1 - discount_rate / 100)"></textarea>
					
					<div class="formula-templates">
						{#each formulaTemplates as template}
							<div class="template-option" on:click={() => applyFormulaTemplate(template)}>
								<h4>{template.name}</h4>
								<code>{template.formula}</code>
							</div>
						{/each}
					</div>
				</div>
				<div class="form-group">
					<label for="is_active">Status</label>
					<select id="is_active" bind:value={newTemplate.is_active}>
						<option value={true}>Active</option>
						<option value={false}>Inactive</option>
					</select>
				</div>
			</div>
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={() => showCreateModal = false}>Cancel</button>
				<button class="btn btn-primary" on:click={createTemplate}>Create Template</button>
			</div>
		</div>
	</div>
{/if}

<!-- Edit Template Modal -->
{#if showEditModal && selectedTemplate}
	<div class="modal-overlay" on:click={() => showEditModal = false}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h2>Edit Pricing Template</h2>
				<button class="btn-icon" on:click={() => showEditModal = false}>
					×
				</button>
			</div>
			<div class="modal-body">
				<div class="form-row">
					<div class="form-group">
						<label for="edit-name">Template Name</label>
						<input type="text" id="edit-name" bind:value={selectedTemplate.name} />
					</div>
					<div class="form-group">
						<label for="edit-display_name">Display Name</label>
						<input type="text" id="edit-display_name" bind:value={selectedTemplate.display_name} />
					</div>
				</div>
				<div class="form-group">
					<label for="edit-description">Description</label>
					<textarea id="edit-description" bind:value={selectedTemplate.description} rows="2"></textarea>
				</div>
				<div class="form-group">
					<label for="edit-category">Category</label>
					<select id="edit-category" bind:value={selectedTemplate.category}>
						{#each templateCategories as cat}
							<option value={cat.value}>{cat.label}</option>
						{/each}
					</select>
				</div>
				<div class="form-group">
					<label>Pricing Formula</label>
					<div class="formula-display">
						{#if selectedTemplate.price_formula}
							<code class="formula-code">{selectedTemplate.price_formula}</code>
						{:else}
							<span class="formula-placeholder">No formula defined</span>
						{/if}
						<button type="button" class="btn-edit-formula" on:click={() => {
							editingFormulaType = 'price';
							tempFormula = selectedTemplate.price_formula || '';
							showFormulaEditor = true;
						}}>
							<Settings size={16} />
							Edit Formula
						</button>
					</div>
				</div>
				<div class="form-group">
					<label>Condition Formula (Optional)</label>
					<div class="formula-display">
						{#if selectedTemplate.condition_formula}
							<code class="formula-code condition">{selectedTemplate.condition_formula}</code>
						{:else}
							<span class="formula-placeholder">No condition defined</span>
						{/if}
						<button type="button" class="btn-edit-formula" on:click={() => {
							editingFormulaType = 'condition';
							tempFormula = selectedTemplate.condition_formula || '';
							showFormulaEditor = true;
						}}>
							<Settings size={16} />
							Edit Condition
						</button>
					</div>
					<p class="help-text">When should this pricing template apply?</p>
				</div>
				<div class="form-group">
					<label for="edit-is_active">Status</label>
					<select id="edit-is_active" bind:value={selectedTemplate.is_active}>
						<option value={true}>Active</option>
						<option value={false}>Inactive</option>
					</select>
				</div>
			</div>
			<div class="modal-footer">
				<div class="modal-footer-left">
					<button class="btn btn-danger" on:click={() => {
						if (confirm('Are you sure you want to delete this pricing template? Products using this template will need to be updated.')) {
							deleteTemplate(selectedTemplate.id);
							showEditModal = false;
						}
					}}>
						<Trash2 size={16} />
						Delete
					</button>
				</div>
				<div class="modal-footer-right">
					<button class="btn btn-secondary" on:click={() => showEditModal = false}>Cancel</button>
					<button class="btn btn-primary" on:click={updateTemplate}>Save</button>
				</div>
			</div>
		</div>
	</div>
{/if}

<!-- Test Formula Modal -->
{#if showTestModal && selectedTemplate}
	<div class="modal-overlay" on:click={() => showTestModal = false}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h2>Test Formula - {selectedTemplate.display_name}</h2>
				<button class="btn-icon" on:click={() => showTestModal = false}>
					×
				</button>
			</div>
			<div class="modal-body">
				<div class="formula-box">
					<div class="formula-label">
						<Code2 size={14} />
						Formula
					</div>
					<code class="formula-text">{selectedTemplate.price_formula}</code>
				</div>
				
				<div class="test-section">
					<h3>Test Variables</h3>
					<div class="test-variables">
						{#if selectedTemplate.variables?.required && Array.isArray(selectedTemplate.variables.required)}
							{#each selectedTemplate.variables.required as varName}
								<div class="test-variable">
									<label for="test-{varName}">{varName}</label>
									<input type="number" id="test-{varName}" 
										bind:value={testVariables[varName]} 
										placeholder="Enter value" />
								</div>
							{/each}
						{/if}
					</div>
					
					<button class="btn btn-primary" on:click={testFormula}>
						<Play size={16} />
						Run Test
					</button>
				</div>
				
				{#if testResult}
					<div class="test-result success">
						<h4>Test Result</h4>
						<div class="test-output">${testResult.output}</div>
						<div class="test-details">
							Execution time: {testResult.execution_time}
						</div>
					</div>
				{/if}
				
				{#if testError}
					<div class="test-result error">
						<h4>Test Error</h4>
						<p class="error-message">{testError}</p>
					</div>
				{/if}
			</div>
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={() => showTestModal = false}>Close</button>
			</div>
		</div>
	</div>
{/if}

<!-- Formula Editor Modal -->
<FormulaEditor
	bind:show={showFormulaEditor}
	title={editingFormulaType === 'price' ? 'Edit Pricing Formula' : 'Edit Condition Formula'}
	formula={tempFormula}
	variables={availableVariables}
	isConditionFormula={editingFormulaType === 'condition'}
	on:createVariable={async (e) => {
		const newVar = e.detail;
		try {
			// Create the variable via API
			const result = await api.post('/products/variables', {
				name: newVar.name,
				display_name: newVar.displayName,
				value_type: newVar.valueType,
				type: 'user',
				description: newVar.description,
				is_active: true
			});
			
			if (result) {
				// Reload variables
				await loadPricingData();
			}
		} catch (error) {
			console.error('Failed to create variable:', error);
			alert('Failed to create variable');
		}
	}}
	on:save={(e) => {
		if (selectedTemplate) {
			if (editingFormulaType === 'price') {
				selectedTemplate.price_formula = e.detail;
			} else if (editingFormulaType === 'condition') {
				selectedTemplate.condition_formula = e.detail;
			}
		}
		showFormulaEditor = false;
		editingFormulaType = null;
	}}
/>