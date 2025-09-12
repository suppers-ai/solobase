<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { 
		X, Play, Info, HelpCircle, AlertCircle, CheckCircle, 
		Variable, Calculator, Code2, Copy, Sparkles, Shield, Plus
	} from 'lucide-svelte';
	
	export let show = false;
	export let title = 'Formula Editor';
	export let formula = '';
	export let variables: Array<{
		name: string;
		displayName: string;
		valueType: string;
		description?: string;
		defaultValue?: any;
	}> = [];
	export let isConditionFormula = false;
	
	const dispatch = createEventDispatcher();
	
	// Test variables for evaluating the formula
	let testVariables: Record<string, any> = {};
	let testResult = '';
	let testError = '';
	let validationResult = '';
	let validationError = '';
	let showHelp = false;
	let activeTab: 'editor' | 'help' = 'editor';
	let showCreateVariable = false;
	let newVariable = {
		name: '',
		displayName: '',
		valueType: 'number',
		description: ''
	};
	
	// Extract variables used in the formula
	function getUsedVariables(formulaStr: string): string[] {
		if (!formulaStr) return [];
		
		// Extract all potential variable names from the formula
		// This regex matches word boundaries that are likely variables
		const matches = formulaStr.match(/\b[a-zA-Z_][a-zA-Z0-9_]*\b/g) || [];
		
		// Filter to only include variables that are in our available variables list
		const availableVarNames = variables.map(v => v.name);
		const usedVars = [...new Set(matches)].filter(match => 
			availableVarNames.includes(match) && 
			!['Math', 'true', 'false', 'null', 'undefined'].includes(match)
		);
		
		return usedVars;
	}
	
	// Get variables actually used in the current formula
	$: usedVariableNames = getUsedVariables(formula);
	$: usedVariables = variables.filter(v => usedVariableNames.includes(v.name));
	
	// Initialize test variables with defaults for used variables only
	$: if (usedVariables.length > 0) {
		// Remove variables that are no longer used
		Object.keys(testVariables).forEach(key => {
			if (!usedVariableNames.includes(key)) {
				delete testVariables[key];
			}
		});
		
		// Add/update variables that are used
		usedVariables.forEach(v => {
			if (!(v.name in testVariables)) {
				testVariables[v.name] = v.defaultValue ?? (
					v.valueType === 'boolean' ? false :
					v.valueType === 'number' ? 0 :
					v.valueType === 'string' ? '' :
					null
				);
			}
		});
	} else {
		testVariables = {};
	}
	
	// Formula snippets for quick insertion
	const formulaSnippets = [
		{ label: 'Basic Price', formula: 'base_price * quantity' },
		{ label: 'Percentage Discount', formula: 'base_price * (1 - discount_percentage / 100)' },
		{ label: 'Tiered Pricing', formula: 'quantity <= 10 ? base_price : quantity <= 50 ? base_price * 0.9 : base_price * 0.8' },
		{ label: 'Member Discount', formula: 'base_price * (is_member ? 0.9 : 1.0)' },
		{ label: 'Bulk Discount', formula: 'base_price * quantity * (quantity >= 100 ? 0.7 : quantity >= 50 ? 0.8 : quantity >= 10 ? 0.9 : 1.0)' },
		{ label: 'Time-based', formula: 'base_price * (is_weekend ? 1.2 : 1.0)' },
		{ label: 'Tax Calculation', formula: 'base_price * (1 + tax_rate)' },
		{ label: 'Minimum Charge', formula: 'Math.max(base_price * quantity, minimum_charge)' },
	];
	
	const conditionSnippets = [
		{ label: 'Quantity Check', formula: 'quantity >= 10' },
		{ label: 'Member Only', formula: 'is_member == true' },
		{ label: 'Date Range', formula: 'date >= start_date && date <= end_date' },
		{ label: 'Location Based', formula: 'location == "US" || location == "CA"' },
		{ label: 'Product Type', formula: 'product_type == "premium"' },
		{ label: 'Combined', formula: 'is_member && quantity >= 10' },
	];
	
	const operators = [
		{ symbol: '+', description: 'Addition' },
		{ symbol: '-', description: 'Subtraction' },
		{ symbol: '*', description: 'Multiplication' },
		{ symbol: '/', description: 'Division' },
		{ symbol: '%', description: 'Modulo' },
		{ symbol: '**', description: 'Exponentiation' },
		{ symbol: '?:', description: 'Ternary (condition ? true : false)' },
		{ symbol: '>', description: 'Greater than' },
		{ symbol: '<', description: 'Less than' },
		{ symbol: '>=', description: 'Greater than or equal' },
		{ symbol: '<=', description: 'Less than or equal' },
		{ symbol: '==', description: 'Equal to' },
		{ symbol: '!=', description: 'Not equal to' },
		{ symbol: '&&', description: 'Logical AND' },
		{ symbol: '||', description: 'Logical OR' },
		{ symbol: '!', description: 'Logical NOT' },
	];
	
	const functions = [
		{ name: 'Math.max(a, b)', description: 'Returns the larger value' },
		{ name: 'Math.min(a, b)', description: 'Returns the smaller value' },
		{ name: 'Math.round(x)', description: 'Rounds to nearest integer' },
		{ name: 'Math.floor(x)', description: 'Rounds down' },
		{ name: 'Math.ceil(x)', description: 'Rounds up' },
		{ name: 'Math.abs(x)', description: 'Absolute value' },
		{ name: 'Math.pow(x, y)', description: 'x to the power of y' },
		{ name: 'Math.sqrt(x)', description: 'Square root' },
	];
	
	function insertSnippet(snippet: string) {
		formula = snippet;
	}
	
	function insertVariable(varName: string) {
		formula = formula ? `${formula} ${varName}` : varName;
	}
	
	function insertOperator(op: string) {
		formula = formula ? `${formula} ${op} ` : op;
	}
	
	function validateFormula() {
		validationError = '';
		validationResult = '';
		
		if (!formula) {
			validationError = 'Formula is empty';
			return;
		}
		
		try {
			// Try to parse the formula
			const testVars = {};
			usedVariables.forEach(v => {
				testVars[v.name] = v.valueType === 'boolean' ? false :
									v.valueType === 'number' ? 1 :
									v.valueType === 'string' ? 'test' : null;
			});
			
			const context = { ...testVars, Math };
			const keys = Object.keys(context);
			const values = Object.values(context);
			
			// Create a function that evaluates the formula
			const evaluator = new Function(...keys, `return ${formula}`);
			// Try to evaluate it
			evaluator(...values);
			
			validationResult = 'Formula syntax is valid ✓';
		} catch (error) {
			validationError = `Syntax error: ${error.message}`;
		}
	}
	
	async function testFormula() {
		testError = '';
		testResult = '';
		
		if (!formula) {
			testError = 'Please enter a formula';
			return;
		}
		
		try {
			// Create a safe evaluation context
			const context = { ...testVariables, Math };
			const keys = Object.keys(context);
			const values = Object.values(context);
			
			// Create a function that evaluates the formula
			const evaluator = new Function(...keys, `return ${formula}`);
			const result = evaluator(...values);
			
			if (isConditionFormula) {
				testResult = result ? 'TRUE ✓ (Template will apply)' : 'FALSE ✗ (Template will not apply)';
			} else {
				testResult = `Result: ${typeof result === 'number' ? result.toFixed(2) : result}`;
			}
		} catch (error) {
			testError = `Error: ${error.message}`;
		}
	}
	
	function saveFormula() {
		dispatch('save', formula);
		close();
	}
	
	function close() {
		show = false;
		testError = '';
		testResult = '';
		validationError = '';
		validationResult = '';
		activeTab = 'editor';
	}
	
	function copyFormula() {
		navigator.clipboard.writeText(formula);
	}
	
	function createVariable() {
		if (!newVariable.name || !newVariable.displayName) {
			alert('Please provide both name and display name for the variable');
			return;
		}
		
		// Emit event to parent to create the variable
		dispatch('createVariable', newVariable);
		
		// Add to local variables list
		variables = [...variables, {
			name: newVariable.name,
			displayName: newVariable.displayName,
			valueType: newVariable.valueType,
			description: newVariable.description
		}];
		
		// Reset form
		newVariable = {
			name: '',
			displayName: '',
			valueType: 'number',
			description: ''
		};
		showCreateVariable = false;
	}
</script>

{#if show}
	<div class="modal-overlay" on:click={close}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<div class="header-left">
					<Calculator size={20} />
					<h2>{title}</h2>
					{#if isConditionFormula}
						<span class="condition-badge">Condition</span>
					{/if}
				</div>
				<button class="btn-close" on:click={close}>
					<X size={20} />
				</button>
			</div>
			
			<div class="modal-tabs">
				<button 
					class="tab {activeTab === 'editor' ? 'active' : ''}"
					on:click={() => activeTab = 'editor'}>
					<Code2 size={16} />
					Editor & Test
				</button>
				<button 
					class="tab {activeTab === 'help' ? 'active' : ''}"
					on:click={() => activeTab = 'help'}>
					<HelpCircle size={16} />
					Help
				</button>
			</div>
			
			<div class="modal-body">
				{#if activeTab === 'editor'}
					<div class="editor-section">
						<div class="formula-input-group">
							<label>Formula Expression</label>
							<div class="formula-input-wrapper">
								<textarea 
									bind:value={formula}
									placeholder={isConditionFormula 
										? "Enter a condition that evaluates to true/false (e.g., quantity >= 10)"
										: "Enter a formula to calculate the price (e.g., base_price * quantity)"}
									rows="3"
									class="formula-input"
								/>
								<button class="btn-copy" on:click={copyFormula} title="Copy formula">
									<Copy size={16} />
								</button>
							</div>
						</div>
						
						<!-- Test Variables immediately after formula -->
						{#if formula && usedVariables.length > 0}
							<div class="test-variables">
								<h4>
									<Play size={16} />
									Test Values
								</h4>
								<div class="variable-inputs">
									{#each usedVariables as variable}
										<div class="variable-input">
											<label for="var-{variable.name}">
												{variable.displayName || variable.name}
												{#if variable.description}
													<span class="var-desc" title={variable.description}>
														<Info size={14} />
													</span>
												{/if}
											</label>
											{#if variable.valueType === 'boolean'}
												<select id="var-{variable.name}" bind:value={testVariables[variable.name]}>
													<option value={true}>True</option>
													<option value={false}>False</option>
												</select>
											{:else if variable.valueType === 'number'}
												<input 
													type="number" 
													id="var-{variable.name}"
													bind:value={testVariables[variable.name]}
													placeholder="0"
												/>
											{:else if variable.valueType === 'string'}
												<input 
													type="text" 
													id="var-{variable.name}"
													bind:value={testVariables[variable.name]}
													placeholder="Enter text value"
												/>
											{:else}
												<input 
													type="text" 
													id="var-{variable.name}"
													bind:value={testVariables[variable.name]}
													placeholder="Enter value"
												/>
											{/if}
										</div>
									{/each}
								</div>
							</div>
						{/if}
						
						<div class="formula-actions">
							<button class="btn btn-validate" on:click={validateFormula}>
								<Shield size={16} />
								Validate Syntax
							</button>
							{#if formula}
								<button class="btn btn-primary" on:click={testFormula}>
									<Play size={16} />
									Test Formula
								</button>
							{/if}
						</div>
						
						{#if validationResult}
							<div class="validation-result success">
								<CheckCircle size={16} />
								<span>{validationResult}</span>
							</div>
						{/if}
						{#if validationError}
							<div class="validation-result error">
								<AlertCircle size={16} />
								<span>{validationError}</span>
							</div>
						{/if}
						{#if testResult}
							<div class="test-result success">
								<CheckCircle size={20} />
								<span>{testResult}</span>
							</div>
						{/if}
						{#if testError}
							<div class="test-result error">
								<AlertCircle size={20} />
								<span>{testError}</span>
							</div>
						{/if}
						
						<div class="helpers">
							<div class="helper-section">
								<div class="helper-header">
									<h4>
										<Variable size={16} />
										Available Variables
									</h4>
									<button class="btn-add-variable" on:click={() => showCreateVariable = !showCreateVariable} title="Create new variable">
										<Plus size={14} />
										Create variable
									</button>
								</div>
								{#if showCreateVariable}
									<div class="create-variable-form">
										<input 
											type="text" 
											placeholder="Variable name (e.g., discount_rate)"
											bind:value={newVariable.name}
											class="variable-input"
										/>
										<input 
											type="text" 
											placeholder="Display name (e.g., Discount Rate)"
											bind:value={newVariable.displayName}
											class="variable-input"
										/>
										<select bind:value={newVariable.valueType} class="variable-input">
											<option value="number">Number</option>
											<option value="boolean">Boolean</option>
											<option value="string">Text</option>
										</select>
										<input 
											type="text" 
											placeholder="Description (optional)"
											bind:value={newVariable.description}
											class="variable-input"
										/>
										<div class="create-variable-actions">
											<button class="btn-save-variable" on:click={createVariable}>
												<CheckCircle size={14} />
												Create
											</button>
											<button class="btn-cancel-variable" on:click={() => showCreateVariable = false}>
												<X size={14} />
												Cancel
											</button>
										</div>
									</div>
								{/if}
								<div class="variable-list">
									{#if variables.length > 0}
										{#each variables as variable}
											<button 
												class="variable-chip"
												on:click={() => insertVariable(variable.name)}
												title={variable.description || ''}>
												<span class="variable-name">{variable.name}</span>
												<span class="variable-type">{variable.valueType}</span>
											</button>
										{/each}
									{:else}
										<p class="no-items">No variables available</p>
									{/if}
								</div>
							</div>
							
							<div class="helper-section">
								<h4>
									<Sparkles size={16} />
									Quick Templates
								</h4>
								<div class="snippet-list">
									{#each (isConditionFormula ? conditionSnippets : formulaSnippets) as snippet}
										<button 
											class="snippet-item"
											on:click={() => insertSnippet(snippet.formula)}>
											<span class="snippet-label">{snippet.label}</span>
											<code class="snippet-formula">{snippet.formula}</code>
										</button>
									{/each}
								</div>
							</div>
						</div>
					</div>
				{:else if activeTab === 'help'}
					<div class="help-section">
						<div class="help-group">
							<h4>Operators</h4>
							<div class="operator-list">
								{#each operators as op}
									<div class="operator-item">
										<code>{op.symbol}</code>
										<span>{op.description}</span>
									</div>
								{/each}
							</div>
						</div>
						
						<div class="help-group">
							<h4>Functions</h4>
							<div class="function-list">
								{#each functions as func}
									<div class="function-item">
										<code>{func.name}</code>
										<span>{func.description}</span>
									</div>
								{/each}
							</div>
						</div>
						
						<div class="help-group">
							<h4>Examples</h4>
							<div class="example-list">
								<div class="example">
									<strong>Simple multiplication:</strong>
									<code>base_price * quantity</code>
								</div>
								<div class="example">
									<strong>Conditional pricing:</strong>
									<code>quantity > 100 ? base_price * 0.8 : base_price</code>
								</div>
								<div class="example">
									<strong>Complex calculation:</strong>
									<code>Math.max(base_price * quantity * (1 - discount), minimum_price)</code>
								</div>
							</div>
						</div>
					</div>
				{/if}
			</div>
			
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={close}>Cancel</button>
				<button class="btn btn-primary" on:click={saveFormula}>
					Save Formula
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
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
		max-width: 900px;
		max-height: 90vh;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	
	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-bottom: 1px solid #e5e7eb;
	}
	
	.header-left {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}
	
	.modal-header h2 {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 600;
		color: #111827;
	}
	
	.condition-badge {
		padding: 0.25rem 0.5rem;
		background: #fef3c7;
		color: #92400e;
		font-size: 0.75rem;
		font-weight: 500;
		border-radius: 0.25rem;
	}
	
	.btn-close {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border: none;
		background: transparent;
		color: #6b7280;
		cursor: pointer;
		border-radius: 0.375rem;
		transition: all 0.2s;
	}
	
	.btn-close:hover {
		background: #f3f4f6;
		color: #111827;
	}
	
	.modal-tabs {
		display: flex;
		gap: 0.5rem;
		padding: 0 1.5rem;
		background: #f9fafb;
		border-bottom: 1px solid #e5e7eb;
	}
	
	.tab {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		background: transparent;
		border: none;
		color: #6b7280;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		border-bottom: 2px solid transparent;
		transition: all 0.2s;
	}
	
	.tab:hover {
		color: #111827;
	}
	
	.tab.active {
		color: #06b6d4;
		border-bottom-color: #06b6d4;
	}
	
	.modal-body {
		flex: 1;
		overflow-y: auto;
		padding: 1.25rem 1.5rem;
	}
	
	.editor-section {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}
	
	.formula-input-group {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}
	
	.formula-input-group label {
		font-size: 0.813rem;
		font-weight: 600;
		color: #374151;
	}
	
	.formula-input-wrapper {
		position: relative;
	}
	
	.formula-input {
		width: 100%;
		padding: 0.625rem 0.75rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		font-family: 'Courier New', monospace;
		font-size: 0.875rem;
		resize: vertical;
		min-height: 80px;
	}
	
	.formula-input:focus {
		outline: none;
		border-color: #06b6d4;
		box-shadow: 0 0 0 3px rgba(6, 182, 212, 0.1);
	}
	
	.formula-actions {
		display: flex;
		gap: 0.75rem;
		margin-top: 1rem;
	}
	
	.btn-validate {
		background: linear-gradient(135deg, #fbbf24, #f59e0b);
		color: #78350f;
		border: 1px solid #f59e0b;
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.425rem 0.875rem;
		border-radius: 0.375rem;
		font-size: 0.813rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
	}
	
	.btn-validate:hover {
		background: linear-gradient(135deg, #f59e0b, #ea580c);
		border-color: #d97706;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
	}
	
	.validation-result {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 0.75rem;
		border-radius: 0.375rem;
		font-size: 0.813rem;
		margin-top: 0.5rem;
	}
	
	.validation-result.success {
		background: #ecfdf5;
		color: #065f46;
		border: 1px solid #86efac;
	}
	
	.validation-result.error {
		background: #fef2f2;
		color: #991b1b;
		border: 1px solid #fca5a5;
	}
	
	.btn-copy {
		position: absolute;
		top: 0.5rem;
		right: 0.5rem;
		padding: 0.25rem;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.25rem;
		color: #6b7280;
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.btn-copy:hover {
		background: #f3f4f6;
		color: #111827;
	}
	
	.helpers {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
		margin-top: 1.25rem;
	}
	
	.helper-section {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
	}
	
	.helper-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	
	.helper-section h4 {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		margin: 0;
		font-size: 0.813rem;
		font-weight: 600;
		color: #374151;
	}
	
	.btn-add-variable {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.25rem 0.75rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		background: white;
		color: #06b6d4;
		font-size: 0.75rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.btn-add-variable:hover {
		background: #f0fdfa;
		border-color: #06b6d4;
	}
	
	.create-variable-form {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0.75rem;
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		margin-top: 0.5rem;
	}
	
	.variable-input {
		padding: 0.375rem 0.5rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.25rem;
		font-size: 0.75rem;
		background: white;
	}
	
	.variable-input:focus {
		outline: none;
		border-color: #06b6d4;
		box-shadow: 0 0 0 2px rgba(6, 182, 212, 0.1);
	}
	
	.create-variable-actions {
		display: flex;
		gap: 0.5rem;
		margin-top: 0.25rem;
	}
	
	.btn-save-variable,
	.btn-cancel-variable {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.25rem 0.5rem;
		border: none;
		border-radius: 0.25rem;
		font-size: 0.75rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.btn-save-variable {
		background: #10b981;
		color: white;
	}
	
	.btn-save-variable:hover {
		background: #059669;
	}
	
	.btn-cancel-variable {
		background: white;
		color: #6b7280;
		border: 1px solid #e5e7eb;
	}
	
	.btn-cancel-variable:hover {
		background: #f3f4f6;
	}
	
	.variable-list {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}
	
	.variable-chip {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.25rem 0.5rem;
		background: #ecfdf5;
		border: 1px solid #86efac;
		border-radius: 0.25rem;
		font-size: 0.75rem;
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.variable-chip:hover {
		background: #d1fae5;
		border-color: #34d399;
	}
	
	.variable-name {
		font-weight: 600;
		color: #065f46;
	}
	
	.variable-type {
		color: #047857;
		opacity: 0.7;
	}
	
	.snippet-list {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
		max-height: 160px;
		overflow-y: auto;
		padding-right: 0.25rem;
	}
	
	.snippet-item {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.125rem;
		padding: 0.375rem 0.5rem;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 0.25rem;
		cursor: pointer;
		transition: all 0.2s;
		text-align: left;
	}
	
	.snippet-item:hover {
		background: #f0fdf4;
		border-color: #86efac;
	}
	
	.snippet-label {
		font-size: 0.813rem;
		font-weight: 500;
		color: #111827;
	}
	
	.snippet-formula {
		font-size: 0.75rem;
		color: #6b7280;
		font-family: 'Courier New', monospace;
	}
	
	.test-variables {
		margin-top: 1rem;
		padding: 0.75rem;
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
	}
	
	.test-variables h4 {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		margin: 0 0 0.75rem 0;
		font-size: 0.813rem;
		font-weight: 600;
		color: #374151;
	}
	
	.no-formula-message,
	.no-variables-message {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		padding: 3rem;
		text-align: center;
		color: #6b7280;
	}
	
	.no-formula-message {
		color: #9ca3af;
	}
	
	.no-variables-message .hint {
		font-size: 0.813rem;
		color: #9ca3af;
		margin-top: -0.5rem;
	}
	
	
	.variable-inputs {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: 0.75rem;
	}
	
	.variable-input {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}
	
	.variable-input label {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		font-size: 0.75rem;
		font-weight: 600;
		color: #4b5563;
		text-transform: uppercase;
		letter-spacing: 0.025em;
	}
	
	.var-desc {
		color: #9ca3af;
		cursor: help;
	}
	
	.variable-input input,
	.variable-input select {
		padding: 0.425rem 0.625rem;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
		font-size: 0.813rem;
		background: white;
		transition: all 0.2s;
	}
	
	.variable-input input:focus,
	.variable-input select:focus {
		outline: none;
		border-color: #06b6d4;
		box-shadow: 0 0 0 2px rgba(6, 182, 212, 0.1);
	}
	
	.test-actions {
		display: flex;
		justify-content: center;
		margin-top: 1rem;
	}
	
	.test-minimal {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1rem;
		padding: 1rem;
		background: #f9fafb;
		border-radius: 0.375rem;
	}
	
	.test-minimal .hint {
		color: #6b7280;
		font-size: 0.875rem;
		text-align: center;
	}
	
	.test-result {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.625rem 0.875rem;
		border-radius: 0.375rem;
		font-size: 0.813rem;
		margin-top: 0.5rem;
	}
	
	.test-result.success {
		background: #ecfdf5;
		color: #065f46;
		border: 1px solid #86efac;
	}
	
	.test-result.error {
		background: #fef2f2;
		color: #991b1b;
		border: 1px solid #fca5a5;
	}
	
	.help-section {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}
	
	.help-group h4 {
		margin: 0 0 0.75rem 0;
		font-size: 0.875rem;
		font-weight: 600;
		color: #374151;
	}
	
	.operator-list,
	.function-list {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
		gap: 0.5rem;
	}
	
	.operator-item,
	.function-item {
		display: flex;
		gap: 0.5rem;
		padding: 0.375rem 0.5rem;
		background: #f9fafb;
		border-radius: 0.25rem;
		font-size: 0.813rem;
	}
	
	.operator-item code,
	.function-item code {
		font-weight: 600;
		color: #0891b2;
	}
	
	.example-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	
	.example {
		padding: 0.75rem;
		background: #f9fafb;
		border-radius: 0.375rem;
		font-size: 0.813rem;
	}
	
	.example strong {
		display: block;
		margin-bottom: 0.25rem;
		color: #374151;
	}
	
	.example code {
		color: #0891b2;
		font-family: 'Courier New', monospace;
	}
	
	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid #e5e7eb;
	}
	
	.btn {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.425rem 0.875rem;
		border-radius: 0.375rem;
		font-size: 0.813rem;
		font-weight: 500;
		border: none;
		cursor: pointer;
		transition: all 0.2s;
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
	}
	
	.btn-primary {
		background: linear-gradient(135deg, #06b6d4, #0891b2);
		color: white;
	}
	
	.btn-primary:hover {
		background: linear-gradient(135deg, #0891b2, #0e7490);
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
	}
	
	.btn-secondary {
		background: white;
		color: #374151;
		border: 1px solid #e5e7eb;
	}
	
	.btn-secondary:hover {
		background: #f9fafb;
	}
	
	.no-items {
		color: #9ca3af;
		font-size: 0.813rem;
		font-style: italic;
	}
</style>