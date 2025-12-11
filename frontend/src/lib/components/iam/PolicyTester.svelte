<script lang="ts">
	import { authFetch } from '$lib/api';

	interface TestInput {
		userId: string;
		resource: string;
		action: string;
	}

	interface TestResult {
		allowed?: boolean;
		error?: string;
		matchedPolicies?: string[];
		userRoles?: string[];
	}

	let testInput: TestInput = {
		userId: '',
		resource: '',
		action: ''
	};

	let testResult: TestResult | null = null;
	let testing = false;

	async function handleTest() {
		testing = true;
		try {
			const response = await authFetch('/api/admin/iam/test-permission', {
				method: 'POST',
				body: JSON.stringify(testInput)
			});

			testResult = await response.json();
		} catch (error) {
			testResult = { error: error instanceof Error ? error.message : 'Unknown error' };
		} finally {
			testing = false;
		}
	}

	function clearResults() {
		testResult = null;
	}
</script>

<div class="section">
	<div class="section-header">
		<h2>Policy Tester</h2>
		<p class="section-subtitle">Test permission checks with different inputs</p>
	</div>
	
	<div class="test-form">
		<div class="form-row">
			<div class="form-group">
				<label for="test-user">User ID</label>
				<input 
					id="test-user" 
					type="text" 
					bind:value={testInput.userId} 
					placeholder="user-123"
					on:input={clearResults}
				/>
			</div>
			
			<div class="form-group">
				<label for="test-resource">Resource</label>
				<input 
					id="test-resource" 
					type="text" 
					bind:value={testInput.resource} 
					placeholder="/api/storage/files"
					on:input={clearResults}
				/>
			</div>
			
			<div class="form-group">
				<label for="test-action">Action</label>
				<input 
					id="test-action" 
					type="text" 
					bind:value={testInput.action} 
					placeholder="read"
					on:input={clearResults}
				/>
			</div>
		</div>
		
		<button 
			class="btn btn-primary" 
			on:click={handleTest}
			disabled={testing || !testInput.userId || !testInput.resource || !testInput.action}
		>
			{testing ? 'Testing...' : 'Test Permission'}
		</button>
	</div>
	
	{#if testResult}
		<div class="test-result" class:allowed={testResult.allowed} class:denied={!testResult.allowed && !testResult.error}>
			{#if testResult.error}
				<div class="result-error">
					<strong>Error:</strong> {testResult.error}
				</div>
			{:else}
				<div class="result-status">
					<span class="status-icon">{testResult.allowed ? '✓' : '✗'}</span>
					<span class="status-text">Permission {testResult.allowed ? 'Allowed' : 'Denied'}</span>
				</div>
				
				{#if testResult.matchedPolicies && testResult.matchedPolicies.length > 0}
					<div class="matched-policies">
						<h4>Matched Policies:</h4>
						<ul>
							{#each testResult.matchedPolicies as policy}
								<li>{policy}</li>
							{/each}
						</ul>
					</div>
				{/if}
				
				{#if testResult.userRoles && testResult.userRoles.length > 0}
					<div class="user-roles">
						<h4>User Roles:</h4>
						<ul>
							{#each testResult.userRoles as role}
								<li>{role}</li>
							{/each}
						</ul>
					</div>
				{/if}
			{/if}
		</div>
	{/if}
</div>

<style>
	.section {
		background: white;
		border-radius: 8px;
		padding: 2rem;
		box-shadow: 0 2px 4px rgba(0,0,0,0.1);
	}
	
	.section-header {
		margin-bottom: 2rem;
	}
	
	.section-header h2 {
		margin: 0 0 0.5rem;
		color: #333;
	}
	
	.section-subtitle {
		margin: 0;
		color: #666;
		font-size: 0.9rem;
	}
	
	.test-form {
		background: #f9f9f9;
		padding: 1.5rem;
		border-radius: 8px;
		margin-bottom: 1.5rem;
	}
	
	.form-row {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1rem;
		margin-bottom: 1.5rem;
	}
	
	.form-group {
		display: flex;
		flex-direction: column;
	}
	
	.form-group label {
		margin-bottom: 0.5rem;
		font-weight: 500;
		color: #333;
	}
	
	.form-group input {
		padding: 0.5rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		font-size: 1rem;
	}
	
	.btn {
		padding: 0.5rem 1rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		background: white;
		color: #333;
		cursor: pointer;
		transition: all 0.3s;
	}
	
	.btn:hover:not(:disabled) {
		background: #f5f5f5;
	}
	
	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	
	.btn-primary {
		background: #4CAF50;
		color: white;
		border-color: #4CAF50;
	}
	
	.btn-primary:hover:not(:disabled) {
		background: #45a049;
	}
	
	.test-result {
		padding: 1.5rem;
		border-radius: 8px;
		border: 2px solid;
	}
	
	.test-result.allowed {
		background: #d4edda;
		border-color: #28a745;
	}
	
	.test-result.denied {
		background: #f8d7da;
		border-color: #dc3545;
	}
	
	.result-error {
		color: #721c24;
	}
	
	.result-status {
		display: flex;
		align-items: center;
		gap: 1rem;
		font-size: 1.2rem;
		font-weight: 600;
		margin-bottom: 1rem;
	}
	
	.status-icon {
		font-size: 1.5rem;
	}
	
	.allowed .result-status {
		color: #155724;
	}
	
	.denied .result-status {
		color: #721c24;
	}
	
	.matched-policies,
	.user-roles {
		margin-top: 1rem;
	}
	
	.matched-policies h4,
	.user-roles h4 {
		margin: 0 0 0.5rem;
		font-size: 0.9rem;
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}
	
	.matched-policies ul,
	.user-roles ul {
		margin: 0;
		padding-left: 1.5rem;
	}
	
	.matched-policies li,
	.user-roles li {
		margin: 0.25rem 0;
		font-family: monospace;
		font-size: 0.9rem;
	}
</style>