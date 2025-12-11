<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { Play, AlertCircle, CheckCircle, Clock } from 'lucide-svelte';

	export let query = '';
	export let executing = false;
	export let error = '';
	export let executionTime = 0;
	export let affectedRows = 0;
	export let results: any[] = [];

	const dispatch = createEventDispatcher();

	function executeQuery() {
		if (query.trim()) {
			dispatch('execute', query);
		}
	}

	function handleKeyDown(event: KeyboardEvent) {
		// Execute query on Ctrl/Cmd + Enter
		if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
			event.preventDefault();
			executeQuery();
		}
	}

	function formatExecutionTime(ms: number): string {
		if (ms < 1000) {
			return `${ms}ms`;
		}
		return `${(ms / 1000).toFixed(2)}s`;
	}
</script>

<div class="sql-editor-container">
	<div class="editor-header">
		<h3>SQL Query</h3>
		<button 
			class="execute-button" 
			on:click={executeQuery}
			disabled={!query.trim() || executing}
			aria-label="Execute query (Ctrl+Enter)"
		>
			<Play size={16} />
			{executing ? 'Executing...' : 'Execute'}
		</button>
	</div>

	<div class="editor-wrapper">
		<textarea
			bind:value={query}
			on:keydown={handleKeyDown}
			placeholder="Enter your SQL query here..."
			class="sql-input"
			disabled={executing}
			aria-label="SQL query editor"
			spellcheck="false"
		></textarea>
	</div>

	{#if error}
		<div class="message error" role="alert">
			<AlertCircle size={16} />
			<span>{error}</span>
		</div>
	{:else if executionTime > 0}
		<div class="message success" role="status">
			<CheckCircle size={16} />
			<span>Query executed successfully</span>
			<div class="execution-stats">
				<span class="stat">
					<Clock size={14} />
					{formatExecutionTime(executionTime)}
				</span>
				{#if affectedRows > 0}
					<span class="stat">
						{affectedRows} row{affectedRows === 1 ? '' : 's'} affected
					</span>
				{/if}
				{#if results.length > 0}
					<span class="stat">
						{results.length} row{results.length === 1 ? '' : 's'} returned
					</span>
				{/if}
			</div>
		</div>
	{/if}

	{#if results.length > 0}
		<div class="results-container">
			<h4>Query Results</h4>
			<div class="results-wrapper">
				<table class="results-table">
					<thead>
						<tr>
							{#each Object.keys(results[0]) as column}
								<th>{column}</th>
							{/each}
						</tr>
					</thead>
					<tbody>
						{#each results as row}
							<tr>
								{#each Object.values(row) as value}
									<td>{value === null ? 'NULL' : value}</td>
								{/each}
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</div>
	{/if}
</div>

<style>
	.sql-editor-container {
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.editor-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		border-bottom: 1px solid #e5e7eb;
	}

	.editor-header h3 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
	}

	.execute-button {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		background: #189AB4;
		color: white;
		border: none;
		border-radius: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.execute-button:hover:not(:disabled) {
		background: #157a8f;
	}

	.execute-button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.editor-wrapper {
		padding: 1rem;
	}

	.sql-input {
		width: 100%;
		min-height: 200px;
		padding: 1rem;
		background: #f9fafb;
		border: 1px solid #d1d5db;
		border-radius: 0.375rem;
		font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
		font-size: 0.875rem;
		line-height: 1.5;
		resize: vertical;
		transition: all 0.2s;
	}

	.sql-input:focus {
		outline: none;
		border-color: #189AB4;
		box-shadow: 0 0 0 3px rgba(24, 154, 180, 0.1);
		background: white;
	}

	.sql-input:disabled {
		opacity: 0.7;
		cursor: not-allowed;
	}

	.message {
		display: flex;
		align-items: flex-start;
		gap: 0.5rem;
		margin: 0 1rem;
		padding: 0.75rem;
		border-radius: 0.375rem;
		font-size: 0.875rem;
	}

	.message.error {
		background: #fee2e2;
		color: #991b1b;
		border: 1px solid #fecaca;
	}

	.message.success {
		background: #d1fae5;
		color: #065f46;
		border: 1px solid #a7f3d0;
	}

	.execution-stats {
		display: flex;
		gap: 1rem;
		margin-left: auto;
	}

	.stat {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		font-size: 0.75rem;
		opacity: 0.8;
	}

	.results-container {
		flex: 1;
		display: flex;
		flex-direction: column;
		padding: 1rem;
		overflow: hidden;
	}

	.results-container h4 {
		margin: 0 0 0.75rem 0;
		font-size: 0.875rem;
		font-weight: 600;
		color: #374151;
	}

	.results-wrapper {
		flex: 1;
		overflow: auto;
		border: 1px solid #e5e7eb;
		border-radius: 0.375rem;
	}

	.results-table {
		width: 100%;
		border-collapse: collapse;
	}

	.results-table th {
		position: sticky;
		top: 0;
		background: #f9fafb;
		border-bottom: 1px solid #e5e7eb;
		padding: 0.5rem;
		text-align: left;
		font-size: 0.75rem;
		font-weight: 600;
		color: #374151;
	}

	.results-table td {
		padding: 0.5rem;
		border-bottom: 1px solid #f3f4f6;
		font-size: 0.75rem;
	}

	.results-table tbody tr:hover {
		background: #f9fafb;
	}
</style>